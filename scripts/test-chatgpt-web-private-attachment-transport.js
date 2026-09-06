'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const protocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');
const transport = require('../android/app/src/main/assets/chatgpt_web_private_attachment_transport.js');
const jsonRequest = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const file = () => new File(['synthetic fixture\n'], 'fixture.txt', { type: 'text/plain' });
const selected = () => ({ useCase: 'ace_upload', storeInLibrary: false, libraryPersistenceMode: 'required', indexForRetrieval: false });
const blobUrl = 'https://uploads.oaiusercontent.com/fixture?sig=synthetic';
const prepared = () => ({ status: 'success', file_id: 'file-synthetic', upload_url: blobUrl });
const stream = () => [
  { file_id: 'file-synthetic', event: 'file.processing.started', progress: 0 },
  { file_id: 'file-synthetic', event: 'file.processing.file_ready', progress: 100 },
  { file_id: 'file-synthetic', event: 'file.processing.completed', progress: 100, extra: { total_tokens: 8, mime_type: 'text/plain' } },
].map(value => JSON.stringify(value)).join('\n');
function fixture(override = {}) {
  const calls = [], stages = [];
  const binding = {};
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/synthetic' },
    AbortController, setTimeout, clearTimeout,
    fetch: async () => { throw new Error('unexpected_fetch'); },
  };
  const request = async (_, url, init, limits) => {
    calls.push({ url, init, limits });
    if (override.respond) return override.respond(url, init, limits, calls.length);
    return url.endsWith('/files') ? { payload: prepared() } :
      url.endsWith('/process_upload_stream') ? { text: stream() } : { ok: true, status: 201 };
  };
  const instance = transport.create(root, {
    protocol, request,
    acquireHeaders: async () => ({ Authorization: 'Bearer page-local-test', Cookie: 'not-forwarded',
      'openai-sentinel-proof-token': 'not-forwarded', 'ChatGPT-Account-ID': 'workspace-test',
      'oai-device-id': 'device-test' }),
    isCurrent: value => value === binding,
    onProgress: update => stages.push(update.stage), ...override.options,
  });
  return { root, instance, binding, calls, stages, upload: (context = selected()) => instance.upload(file(), context, binding) };
}

test('prepare uses the verified legacy route contract without enabling multipart', () => {
  const result = protocol.prepare(file(), selected());
  assert.equal(result.use_case, 'ace_upload');
  assert.equal(result.supports_direct_azure_multipart, false);
  assert.equal(result.entry_surface, 'chat_composer');
  assert.equal(result.store_in_library, false);
});

test('PDF creation binds the selected model only to the official create request', async () => {
  for (const type of ['application/pdf', 'text/plain']) {
    const pdf = new File(['%PDF-1.7\nsynthetic'], 'fixture.PDF', { type });
    const f = fixture({ respond: async url => url.endsWith('/files') ? { payload: prepared() } :
      url.endsWith('/process_upload_stream') ? { text: stream().replace('text/plain', type) } : {} });
    const context = { ...selected(), modelSlug: 'synthetic-selected-model' };
    const result = await f.instance.upload(pdf, context, f.binding);
    assert.equal(result.ok, true);
    assert.equal(f.calls[0].init.headers['x-oai-model-slug'], context.modelSlug);
    assert.equal(f.calls[1].init.headers['x-oai-model-slug'], undefined);
    assert.equal(f.calls[2].init.headers['x-oai-model-slug'], undefined);
    assert.equal(f.calls[1].init.body, pdf, 'document bytes are not image-normalized or re-encoded');
    assert.equal(JSON.parse(f.calls[0].init.body).mime_type, type);
    assert.equal(result.metadata.mimeType, type);
    assert.equal(result.associated, false);
  }
});

test('PDF model ownership is validated and snapshotted before asynchronous authentication', async () => {
  const pdf = new File(['%PDF-1.7'], 'fixture.pdf', { type: 'application/pdf' });
  for (const modelSlug of [undefined, '', 'High effort', '\u6781\u9ad8', 'm\r\nheader: value', 'a'.repeat(129), {}]) {
    const f = fixture();
    const result = await f.instance.upload(pdf, { ...selected(), modelSlug }, f.binding);
    assert.equal(result.ok, false);
    assert.equal(f.calls.length, 0);
  }
  let release;
  const f = fixture({ options: { acquireHeaders: () => new Promise(resolve => { release = resolve; }) } });
  const context = { ...selected(), modelSlug: 'synthetic-first' };
  const pending = f.instance.upload(pdf, context, f.binding);
  context.modelSlug = 'synthetic-other';
  release({ Authorization: 'Bearer synthetic-token', 'x-oai-model-slug': 'untrusted-cached-model' });
  await pending;
  assert.equal(f.calls[0].init.headers['x-oai-model-slug'], 'synthetic-first');
  const text = fixture();
  await text.upload({ ...selected(), modelSlug: 'synthetic-first' });
  assert.equal(text.calls[0].init.headers['x-oai-model-slug'], undefined);
});

test('reject untyped images, projects and conflicting temporary persistence before any request', async () => {
  for (const patch of [{ isProjectThread: true }, { isTemporaryChat: true }, { gizmoId: 'project' },
    { directoryId: 'folder' }, { useCase: 'guessed' }, { libraryPersistenceMode: 'guessed' }]) {
    const f = fixture();
    assert.equal((await f.upload({ ...selected(), ...patch })).ok, false);
    assert.equal(f.calls.length, 0);
  }
  assert.throws(() => protocol.prepare(new File(['x'], 'x.png', { type: 'image/png' }), selected()));
});

test('temporary private uploads omit persistence mode and preserve explicit privacy metadata', async () => {
  const f = fixture();
  const result = await f.upload({ ...selected(), isTemporaryChat: true, libraryPersistenceMode: undefined });
  assert.equal(result.ok, true);
  assert.equal(result.isTemporaryChat, true);
  const created = JSON.parse(f.calls[0].init.body), processed = JSON.parse(f.calls[2].init.body);
  assert.equal(created.store_in_library, false);
  assert.equal(Object.hasOwn(created, 'library_persistence_mode'), false);
  assert.equal(Object.hasOwn(processed, 'library_persistence_mode'), false);
  assert.deepEqual(processed.metadata, { store_in_library: false, is_temporary_chat: true, is_project_thread: false });
  for (const patch of [{ storeInLibrary: true }, { indexForRetrieval: true },
    { isTemporaryChat: 'true' }, { libraryPersistenceMode: 'opportunistic' }]) {
    const other = fixture();
    const receipt = await other.upload({ ...selected(), libraryPersistenceMode: undefined,
      isTemporaryChat: true, ...patch });
    assert.equal(receipt.ok, false);
    assert.equal(other.calls.length, 0);
  }
});

test('temporary processing cannot confirm an unexpected library-persistence receipt or replay it', async () => {
  const f = fixture({ respond: async url => url.endsWith('/files') ? { payload: prepared() } :
    url.endsWith('/process_upload_stream') ? { text: JSON.stringify({ event: 'file.processing.completed',
      file_id: 'file-synthetic', progress: 100, extra: { library_persistence_result: 'library' } }) } : {} });
  const result = await f.upload({ ...selected(), isTemporaryChat: true, libraryPersistenceMode: undefined });
  assert.equal(result.ok, false);
  assert.equal(result.code, 'processing_metadata_mismatch');
  assert.equal(f.calls.length, 3);
  assert.equal(result.mayHaveSideEffects, true);
});

test('multimodal create/process and ready metadata retain the prepared image dimensions', async () => {
  for (const type of ['image/jpeg', 'image/png', 'image/webp']) {
    const f = fixture({ respond: async url => url.endsWith('/files') ? { payload: prepared() } :
      url.endsWith('/process_upload_stream') ? { text: stream().replace('text/plain', type) } : {} });
    const image = new File(['image bytes'], 'image.png', { type });
    const result = await f.instance.upload(image, { ...selected(), useCase: 'multimodal',
      imageDimensions: { width: 320, height: 240 } }, f.binding);
    assert.equal(result.ok, true);
    assert.deepEqual(result.imageDimensions, { width: 320, height: 240 });
    assert.equal(result.associated, false);
    assert.equal(JSON.parse(f.calls[0].init.body).use_case, 'multimodal');
    assert.equal(JSON.parse(f.calls[2].init.body).use_case, 'multimodal');
    assert.equal(f.calls[1].init.body, image);
    assert.equal(f.calls[1].init.headers['Content-Type'], type);
    assert.equal(f.calls[1].init.headers.authorization, undefined);
  }
});

test('image dimensions are copied before async auth and cannot drift during upload', async () => {
  let release;
  const f = fixture({ options: { acquireHeaders: () => new Promise(resolve => { release = resolve; }) },
    respond: async url => url.endsWith('/files') ? { payload: prepared() } :
      url.endsWith('/process_upload_stream') ? { text: stream().replace('text/plain', 'image/png') } : {} });
  const dimensions = { width: 320, height: 240 };
  const pending = f.instance.upload(new File(['x'], 'x.png', { type: 'image/png' }),
    { ...selected(), useCase: 'multimodal', imageDimensions: dimensions }, f.binding);
  dimensions.width = 9999;
  release({ Authorization: 'Bearer synthetic-auth-token' });
  const result = await pending;
  assert.equal(result.ok, true);
  assert.equal(result.imageDimensions.width, 320);
});

test('wrong image metadata and an image sent as a generic file cannot become ready', async () => {
  const image = new File(['x'], 'x.png', { type: 'image/png' });
  for (const patch of [{ imageDimensions: undefined }, { imageDimensions: { width: 9000, height: 20 } },
    { useCase: 'ace_upload' }, { indexForRetrieval: true }]) {
    const f = fixture();
    const result = await f.instance.upload(image, { ...selected(), useCase: 'multimodal',
      imageDimensions: { width: 320, height: 240 }, ...patch }, f.binding);
    assert.equal(result.ok, false);
    assert.equal(f.calls.length, 0);
  }
  const f = fixture();
  const mismatch = await f.instance.upload(image, { ...selected(), useCase: 'multimodal',
    imageDimensions: { width: 320, height: 240 } }, f.binding);
  assert.equal(mismatch.code, 'processing_metadata_mismatch');
  assert.equal(mismatch.ok, false);
  assert.equal(f.calls.length, 3, 'no automatic repeat after processing');
});

test('reject invalid file sizes, names and MIME types', () => {
  for (const input of [new File([], 'empty.txt', { type: 'text/plain' }),
    { size: 8 * 1024 * 1024 + 1, slice() {}, name: 'large.txt', type: 'text/plain' },
    new File(['x'], '../secret.txt', { type: 'text/plain' }),
    new File(['x'], 'x.txt', { type: '' })]) assert.throws(() => protocol.prepare(input, selected()));
});

test('reservation responses are not legacy file receipts', () => {
  assert.throws(() => protocol.destination({ eligible: true, reservation_id: 'reservation', upload_url: blobUrl }, 'text/plain'));
});

test('upload targets exclude unknown origins, local addresses and redirects', () => {
  for (const url of ['http://uploads.oaiusercontent.com/a?sig=x', 'https://oaiusercontent.com.evil.test/a?sig=x',
    'https://localhost/a?sig=x', 'https://127.0.0.1/a?sig=x', '/backend-api/estuary/upload_content_bytes',
    'https://user:password@uploads.oaiusercontent.com/a?sig=x',
    'https://uploads.oaiusercontent.com:9443/a?sig=x', 'https://uploads.oaiusercontent.com/a']) {
    assert.throws(() => protocol.destination({ ...prepared(), upload_url: url }, 'text/plain'));
  }
});

test('multipart and extra upload-header contracts stay unsupported until verified', () => {
  assert.throws(() => protocol.destination({ ...prepared(), direct_library_upload_strategy: { kind: 'direct_azure_multipart' } }, 'text/plain'));
  assert.throws(() => protocol.destination({ ...prepared(), upload_headers: { Authorization: 'unexpected' } }, 'text/plain'));
  const aws = protocol.destination({ ...prepared(), upload_url: blobUrl + '&X-Amz-Algorithm=synthetic' }, 'text/plain');
  assert.deepEqual(aws.headers, { 'Content-Type': 'text/plain' });
});

test('private upload owns prepare, isolated blob PUT and complete processing without a DOM click', async () => {
  const f = fixture();
  const result = await f.upload();
  assert.equal(result.ok, true);
  assert.equal(result.associated, false);
  assert.equal(result.metadata.fileTokenSize, 8);
  assert.deepEqual(f.stages, ['preparing', 'uploading', 'processing', 'processed']);
  assert.deepEqual(f.calls.map(c => c.init.method), ['POST', 'PUT', 'POST']);
  assert.deepEqual(f.calls.map(c => c.url), ['/backend-api/files', blobUrl, '/backend-api/files/process_upload_stream']);
  assert.equal(f.calls[0].init.headers.authorization, 'Bearer page-local-test');
  assert.equal(f.calls[0].init.headers['chatgpt-account-id'], 'workspace-test');
  assert.equal(f.calls[0].init.headers.Cookie, undefined);
  assert.equal(f.calls[0].init.headers['openai-sentinel-proof-token'], undefined);
  assert.equal(f.calls[1].init.credentials, 'omit');
  assert.deepEqual(Object.keys(f.calls[1].init.headers).sort(), ['Content-Type', 'x-ms-blob-type', 'x-ms-version'].sort());
  for (const call of f.calls) assert.equal(call.init.redirect, 'error');
  assert.equal(JSON.parse(f.calls[2].init.body).file_id, 'file-synthetic');
  assert.equal(JSON.stringify(result).includes('sig='), false);
});

test('NDJSON processing rejects empty, truncated, SSE, intermediate and failed streams', () => {
  for (const text of ['', '{}', '{"event":"processing.started","progress":5}',
    'data: {"event":"processing.completed","progress":100}\n\n',
    stream() + '\n{"file_id":"file-synthetic","event":"file.processing.failed","progress":100}', stream() + '\n{',
    stream().split('\n').slice(0, 2).join('\n'),
    stream() + '\n' + stream().split('\n')[0],
    stream().replaceAll('file-synthetic', 'file-other'),
    JSON.stringify({ file_id: 'file-synthetic', event: 'file.processing.unknown', progress: 100 })]) {
    assert.throws(() => protocol.processed(text, 'file-synthetic'));
  }
  assert.equal(protocol.processed(stream().replace(/\n/g, '\r\n'), 'file-synthetic').eventCount, 3);
});

test('stream metadata and event counts are bounded', () => {
  assert.throws(() => protocol.processed(' '.repeat(256 * 1024 + 1), 'file-synthetic'));
  assert.throws(() => protocol.processed(Array(257).fill(stream().split('\n')[2]).join('\n'), 'file-synthetic'));
  const result = protocol.processed(JSON.stringify({ file_id: 'file-synthetic', event: 'file.processing.completed', progress: 100,
    extra: { transcript: 'not-projected', total_tokens: -1, metadata_object_id: '../invalid' } }), 'file-synthetic');
  assert.deepEqual(result.metadata, {});
});

test('request failure never replays a write or reports association success', async () => {
  for (const failedStep of [1, 2, 3]) {
    const f = fixture({ respond: async (_, __, ___, step) => {
      if (step === failedStep) throw new Error('http_503');
      return step === 1 ? { payload: prepared() } : { ok: true };
    } });
    const result = await f.upload();
    assert.equal(result.ok, false);
    assert.equal(result.mayHaveSideEffects, true);
    assert.equal(f.calls.length, failedStep);
    assert.equal((await f.upload()).code, 'cooldown');
    assert.equal(f.calls.length, failedStep);
  }
});

test('changed conversation discards late prepare and never uploads bytes', async () => {
  let release;
  const waiting = new Promise(resolve => { release = resolve; });
  const f = fixture({ respond: async () => waiting });
  const pending = f.upload();
  await new Promise(resolve => setImmediate(resolve));
  f.root.location.href = 'https://chatgpt.com/c/other';
  release({ payload: prepared() });
  assert.equal((await pending).code, 'context_changed');
  assert.equal(f.calls.length, 1);
});

test('single-flight and cancellation settle while authorization is pending', async () => {
  const f = fixture({ options: { acquireHeaders: () => new Promise(() => {}) } });
  const pending = f.upload();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal((await f.upload()).code, 'busy');
  f.instance.cancel();
  assert.equal((await pending).code, 'cancelled');
  assert.equal(f.calls.length, 0);
  assert.equal(f.instance.snapshot().stage, 'idle');
});

test('caller edits cannot mutate a reserved upload context', async () => {
  let release;
  const waiting = new Promise(resolve => { release = resolve; });
  const f = fixture({ options: { acquireHeaders: () => waiting } });
  const context = selected();
  const pending = f.upload(context);
  context.useCase = 'my_files'; context.storeInLibrary = true;
  release({ Authorization: 'Bearer page-local-test' });
  assert.equal((await pending).ok, true);
  assert.equal(JSON.parse(f.calls[2].init.body).use_case, 'ace_upload');
  assert.equal(JSON.parse(f.calls[2].init.body).metadata.store_in_library, false);
});

test('authorization has an independent deadline and never starts a write after timing out', async () => {
  const f = fixture({ options: { acquireHeaders: () => new Promise(() => {}) } });
  f.root.setTimeout = (action, delay) => setTimeout(action, delay === 7000 ? 5 : delay);
  const result = await f.upload();
  assert.equal(result.code, 'auth_timeout');
  assert.equal(result.mayHaveSideEffects, false);
  assert.equal(f.calls.length, 0);
});

test('late processing cannot associate a cancelled upload', async () => {
  let release;
  const f = fixture({ respond: async (_, __, ___, step) => {
    if (step === 1) return { payload: prepared() };
    if (step === 2) return { ok: true };
    return new Promise(resolve => { release = resolve; });
  } });
  const pending = f.upload();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.instance.snapshot().stage, 'processing');
  f.instance.cancel();
  release({ text: stream() });
  const result = await pending;
  assert.equal(result.code, 'cancelled');
  assert.equal(result.ok, false);
  assert.equal(result.mayHaveSideEffects, true);
  assert.equal(f.stages.includes('processed'), false);
});

test('error details never expose server messages or credentials', async () => {
  const f = fixture({ respond: async () => { throw new Error('secret_value_from_server'); } });
  const result = await f.upload();
  assert.equal(result.code, 'request_failed');
  assert.equal(JSON.stringify(result).includes('secret_value'), false);
});

test('actual bounded request helper aborts a hanging upload rather than replaying it', async () => {
  const f = fixture({ options: { request: (root, url, init, limits) => jsonRequest.request(root, url, init, { ...limits, timeoutMs: 10 }) } });
  f.root.fetch = async (url, init) => {
    f.calls.push({ url, init });
    if (url.endsWith('/files')) return new Response(JSON.stringify(prepared()), { status: 200 });
    return new Promise(() => {});
  };
  const result = await f.upload();
  assert.equal(result.code, 'timeout');
  assert.equal(result.stage, 'uploading');
  assert.equal(f.calls.length, 2);
  assert.equal(f.calls[1].init.signal.aborted, true);
});
