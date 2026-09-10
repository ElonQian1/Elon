'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const fixture = require('./fixtures/chatgpt-attachment-composer');
const assets = '../android/app/src/main/assets/';
const library = require(assets + 'chatgpt_web_private_library_attachment');
const senderModule = require(assets + 'chatgpt_web_private_attachment_send');
const mountedPath = require('node:path').resolve(__dirname, assets + 'chatgpt_web_private_mounted_library_attachment.js');
const HANDLE = 'library_' + 'a'.repeat(32);

function setup(patch = {}) {
  const f = fixture();
  Object.assign(f.fiber.memoizedProps, { conversation: {}, onCreateNewCompletion() {},
    currentModelId: 'synthetic-model', entrySurface: 'chat_composer', isLibraryEnabled: true });
  Object.assign(f.root, { File, crypto: require('node:crypto').webcrypto,
    __elonChatGptPrivateLibraryAttachment: library,
    __elonChatGptPrivateLibraryDownload: require(assets + 'chatgpt_web_private_library_download'),
    __elonChatGptPrivateJsonRequest: require(assets + 'chatgpt_web_private_json_request') });
  if (fs.existsSync(mountedPath)) f.root.__elonChatGptPrivateMountedLibraryAttachment = require(mountedPath);
  const source = { kind: 'file', id: 'external-gdrive:file:synthetic123', file_id: 'file-stale-copy',
    name: 'fixture.txt', mime_type: 'text/plain', file_size_bytes: 45, ...patch };
  let fresh = true;
  f.root.__elonChatGptPrivateLibraryCatalog = { selectAttachment: handle => handle === HANDLE
    ? { source: { ...source }, current: () => fresh } : null };
  const requests = [], responses = [], changes = [];
  const result = { file_id: 'file-materialized', file_name: 'fixture.txt', mime_type: 'text/plain', file_size_bytes: 45 };
  f.root.fetch = async (url, init) => { requests.push({ url, init }); return Response.json(result); };
  const sender = senderModule.create(f.root, { composer: f.composer });
  const attach = (id = 'mcp_mounted1') => sender.attachLibrary({ requestId: id, selected: true,
    value: JSON.stringify({ fileHandle: HANDLE }) }, (...v) => responses.push(v), v => changes.push(v));
  const supported = () => assert.ok(library.create(f.root).descriptor(source), 'mounted file must be attachable');
  return { ...f, source, requests, responses, changes, sender, attach, result, supported, expire: () => { fresh = false; } };
}

test('mounted selection prepares once with retrieval intent and preserves source in the existing send lease', async () => {
  const f = setup(); f.supported();
  await Promise.all([f.attach(), f.attach()]);
  assert.equal(f.requests.length, 1);
  const request = f.requests[0];
  assert.equal(request.url, 'https://chatgpt.com/backend-api/files/library/mounted/materialize');
  assert.equal(request.init.method, 'POST');
  assert.equal(request.init.credentials, 'same-origin');
  assert.equal(request.init.redirect, 'error');
  assert.deepEqual(JSON.parse(request.init.body), { file_id: f.source.id, name: 'fixture.txt',
    mime_type: 'text/plain', index_for_retrieval: true });
  assert.equal(f.changes.length, 1);
  assert.ok(f.responses.every(row => row[1] && row[2] === 'library_attachment_associated'));
  const [entry] = f.store.readyFiles$();
  assert.equal(entry.file.size, 0);
  assert.equal(entry.fileId, 'file-materialized');
  assert.equal(entry.mountedLibraryFileId, f.source.id);
  assert.equal(entry.mountedLibraryMimeType, 'text/plain');
  assert.equal(entry.libraryProvider, 'google_drive');
  assert.equal(entry.libraryEntrypoint, 'composer_library_picker');
  assert.equal(entry.libraryFileId, undefined);
  assert.equal(entry.requiresMountedLibraryMaterialization, undefined);
  assert.equal(entry.fileSpec.size, 45);
  const lease = f.sender.prepareSubmit(f.store);
  assert.equal(lease.current(), true);
  assert.equal(lease.readyFiles[0].mountedLibraryFileId, f.source.id);
  assert.equal(lease.consumeAccepted(), true);
  assert.equal(f.store.files$().length, 0);
  assert.ok(!JSON.stringify([f.responses, f.changes, f.sender.merge([])]).includes(f.source.id));
});

test('all evidenced concrete providers preserve their original ID rather than a copied backing file', async () => {
  for (const [id, provider] of [
    ['external-gdrive:account:syntheticAccount:file:synthetic123', 'google_drive'],
    ['external-box:file:1234567', 'box'], ['external-dropbox:file:id:synthetic123', 'dropbox'],
    ['external-sharepoint:file:v1:item:' + btoa('drive-id').replace(/=+$/, '') + ':' + btoa('item-id').replace(/=+$/, ''), 'sharepoint'],
  ]) {
    const f = setup({ id }); f.supported(); await f.attach();
    assert.equal(f.responses[0][1], true);
    assert.equal(f.store.readyFiles$()[0].libraryProvider, provider);
    assert.equal(JSON.parse(f.requests[0].init.body).file_id, id);
  }
});

test('returned name, document MIME, size and image preview are used without downloading or reuploading bytes', async () => {
  for (const [sourceMime, name, mime] of [
    ['application/vnd.google-apps.document', 'document.docx', 'application/vnd.openxmlformats-officedocument.wordprocessingml.document'],
    ['application/vnd.google-apps.spreadsheet', 'sheet.xlsx', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'],
    ['application/vnd.google-apps.presentation', 'slides.pptx', 'application/vnd.openxmlformats-officedocument.presentationml.presentation'],
    ['image/png', 'image.png', 'image/png'],
  ]) {
    const f = setup({ mime_type: sourceMime, file_size_bytes: null }); f.supported();
    Object.assign(f.result, { file_name: name, mime_type: mime, file_size_bytes: 120,
      preview_file: { file_id: 'file-preview', mime_type: 'image/png' } });
    await f.attach();
    assert.equal(f.responses[0][1], true);
    const item = f.store.readyFiles$()[0];
    assert.equal(item.file.name, name); assert.equal(item.file.type, mime);
    assert.equal(item.fileSpec.size, 120); assert.equal(item.file.size, 0);
    assert.equal(item.previewFile.file_id, 'file-preview');
    assert.equal(item.fileSpec.width, mime === 'image/png' ? 512 : undefined);
    assert.equal(f.sender.merge([])[0].name, name);
    assert.equal(f.sender.remove(f.sender.merge([])[0].id, () => {}, () => {}), true);
    assert.equal(f.requests.length, 1);
  }
});

test('unsupported identities, containers and attachment scopes cause no materialization', async () => {
  for (const patch of [ { id: 'external-gdrive:folder:synthetic123' }, { id: 'external-box:file:0' },
    { library_provider: 'box' }, { external_account: {} }, { project_id: 'project' }, { context_scopes: [] },
    { is_project: true }, { saved_entity: {} }, { trashed_at: 1 }, { name: '../file.txt' },
    { mime_type: 'video/mp4' }, { mime_type: 'application/vnd.google-apps.form' }, { file_size_bytes: 9 * 1024 * 1024 }]) {
    const f = setup(patch); await f.attach();
    assert.equal(f.responses[0][1], false); assert.equal(f.requests.length, 0);
  }
  for (const change of [ f => { f.root.location.href += '?temporary-chat=true'; },
    f => { f.fiber.memoizedProps.isLibraryEnabled = false; },
    f => { f.store.files$.set([{ status: 'ready', tempId: 'other' }]); } ]) {
    const f = setup(); f.supported(); change(f); const before = f.store.files$(); await f.attach();
    assert.equal(f.requests.length, 0); assert.equal(f.store.files$(), before);
  }
});

test('account, document, route, model, selection and cancellation changes never stage a late result', async () => {
  for (const change of [ f => f.setAccount('Bearer synthetic-other-token'),
    f => { f.root.__elonChatGptDocumentToken = 'doc_other_1'; },
    f => { f.root.location.href += '?temporary-chat=true'; }, f => f.setModel('other'),
    f => f.expire(), f => f.sender.cancel(), f => { f.store.files$.set([{ status: 'ready', tempId: 'other' }]); } ]) {
    const f = setup(); f.supported();
    let resolve;
    f.root.fetch = () => new Promise(done => { resolve = done; });
    const pending = f.attach();
    await new Promise(done => setImmediate(done));
    assert.equal(typeof resolve, 'function');
    change(f); resolve(Response.json(f.result)); await pending;
    assert.equal(f.responses[0][1], false);
    assert.equal(f.changes.length, 0);
    assert.ok(f.store.files$().every(row => row.tempId === 'other'));
  }
});

test('timeout or invalid response does not replay the write or auto-fallback when clicked again', async () => {
  for (const patch of [ null, { file_id: 'external-gdrive:file:other' }, { file_name: '../wrong.txt' },
    { file_size_bytes: 9 * 1024 * 1024 }, { mime_type: 'video/mp4' }, { preview_file: 'not-an-object' } ]) {
    const f = setup(); f.supported(); let calls = 0;
    f.root.fetch = async () => { calls++; if (!patch) throw new Error('timeout'); return Response.json({ ...f.result, ...patch }); };
    await f.attach(); await f.attach(); await f.attach('mcp_mounted2');
    assert.equal(calls, 1);
    assert.ok(f.responses.every(row => row[1] === false));
    assert.equal(f.store.files$().length, 0);
    assert.equal(f.responses[2][2], 'library_selection_expired');
  }
});

test('deadline settles even when fetch ignores cancellation, and late completion cannot stage a file', async () => {
  const f = setup(); f.supported(); let resolve, calls = 0;
  f.root.setTimeout = (callback, ms) => setTimeout(callback, ms === 12000 ? 5 : ms);
  f.root.fetch = () => { calls++; return new Promise(done => { resolve = done; }); };
  await f.attach(); await f.attach('mcp_aftertimeout');
  resolve(Response.json(f.result));
  await new Promise(done => setImmediate(done));
  assert.equal(calls, 1); assert.equal(f.changes.length, 0);
  assert.equal(f.store.files$().length, 0);
  assert.ok(f.responses.every(row => !row[1]));
});

test('real catalogue emits an opaque attach action and delegates selected metadata to the single attachment owner', async () => {
  const f = setup(); f.supported(); const events = [];
  const catalog = require(assets + 'chatgpt_web_private_library_catalog').create(f.root);
  f.root.__elonChatGptPrivateLibraryCatalog = catalog;
  f.root.fetch = async (url, init) => {
    f.requests.push({ url, init });
    return Response.json(url.includes('/files/library/nodes') ? { items: [f.source], cursor: null } : f.result);
  };
  await catalog.list({ requestId: 'mcp_list1', value: '{}' }, event => events.push(event), () => {});
  const row = events.at(-1).items[0];
  assert.equal(row.canAttach, true);
  assert.match(row.handle, /^library_[a-f0-9]{32}$/);
  assert.equal(f.requests.length, 1, 'listing is not a materialization write');
  assert.ok(!JSON.stringify(events).includes(f.source.id));
  assert.ok(!JSON.stringify(events).includes(f.source.file_id));
  await f.sender.attachLibrary({ requestId: 'mcp_catalogattach', selected: true,
    value: JSON.stringify({ fileHandle: row.handle }) }, (...value) => f.responses.push(value), () => {});
  assert.equal(f.responses[0][1], true);
  assert.equal(f.requests.length, 2);
  assert.equal(f.store.readyFiles$()[0].mountedLibraryFileId, f.source.id);
  catalog.dispose();
});

test('production asset load order supplies the mounted preparation before catalogue and sender initialization', () => {
  const list = fs.readFileSync(require('node:path').resolve(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  const name = 'chatgpt_web_private_mounted_library_attachment.js';
  assert.ok(list.includes(name));
  for (const next of ['chatgpt_web_private_library_attachment.js', 'chatgpt_web_private_library_catalog.js', 'chatgpt_web_private_attachment_send.js']) {
    assert.ok(list.indexOf(name) < list.indexOf(next));
  }
});
