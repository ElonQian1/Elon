'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { webcrypto, createHash } = require('node:crypto');
const library = require('../android/app/src/main/assets/chatgpt_web_private_attachment_library.js');
const protocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');
const tick = () => new Promise(resolve => setImmediate(resolve));
const file = () => new File(['synthetic library fixture'], 'fixture.txt', { type: 'text/plain' });
const policy = () => ({ storeInLibrary: true, isTemporaryChat: false, isProjectThread: false,
  useCase: 'ace_upload', libraryPersistenceMode: 'opportunistic' });
const reused = () => ({ reusable_library_file: { file_id: 'file-existing', library_file_id: 'library-existing',
  file_name: 'existing-name.txt', mime_type: 'text/plain' } });
const good = (name, value) => ({ name, value, details: { reason: 'Network:Recognized' } });

function fixture(options = {}) {
  const calls = [], gates = [], imports = [], timers = new Set();
  let current = true;
  const controller = new AbortController();
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' }, crypto: webcrypto,
    AbortController, performance: { now: () => performance.now(), getEntriesByName: () => options.loaded === false ? [] : [{}] },
    setTimeout: (fn, ms) => { const id = setTimeout(fn, ms); timers.add(id); return id; },
    clearTimeout: id => { clearTimeout(id); timers.delete(id); } };
  const client = { loadingStatus: options.loading || 'Ready',
    getFeatureGate: name => { gates.push(name); return options.gate ? options.gate(name) : good(name, true); },
    getExperiment: name => ({ ...good(name), groupName: 'treatment', get: () => true, ...options.experiment }) };
  const instance = library.create(root, { protocol,
    loadRuntime: async url => { imports.push(url); return options.loadRuntime ? options.loadRuntime() : { t6: () => client }; },
    request: async (_, url, init, limits) => {
      calls.push({ url, init, limits });
      return options.request ? options.request(url, init) : { payload: reused() };
    } });
  const headers = { authorization: 'Bearer synthetic-token', 'Content-Type': 'application/json' };
  return { root, calls, gates, imports, timers, controller, setCurrent: value => { current = value; },
    run: upload => instance.transfer(options.file || file(), { ...policy(), ...options.context }, headers, {
      signal: controller.signal, assertCurrent: () => { if (!current || controller.signal.aborted) throw new Error('cancelled'); },
      upload,
    }) };
}

const cancelable = signal => new Promise((_, reject) => {
  if (signal.aborted) return reject(new Error('cancelled'));
  signal.addEventListener('abort', () => reject(new Error('cancelled')), { once: true });
});

test('enabled official reuse hashes the exact file and waits for the losing byte upload to stop', async () => {
  const f = fixture(); let stopped = false;
  const result = await f.run(async signal => {
    try { await cancelable(signal); } catch (error) { await tick(); stopped = true; throw error; }
  });
  assert.equal(stopped, true);
  assert.equal(result.kind, 'reused'); assert.equal(result.file.fileId, 'file-existing');
  const call = f.calls[0];
  assert.equal(call.url, '/backend-api/files/library/reuse');
  assert.deepEqual(JSON.parse(call.init.body), { file_size_bytes: file().size,
    sha256_digest: createHash('sha256').update(await file().text()).digest('hex') });
  assert.equal(call.init.credentials, 'include'); assert.equal(call.init.redirect, 'error');
  assert.equal(call.init.headers.authorization, 'Bearer synthetic-token');
  assert.ok(call.limits.timeoutMs <= 1500); assert.equal(call.limits.maxBytes, 16384);
  assert.equal(f.timers.size, 0);
});

test('upload completion wins without waiting for a slow hash, runtime or lookup', { timeout: 3000 }, async () => {
  for (const stage of ['runtime', 'hash', 'lookup']) {
    let release, entered;
    const started = new Promise(resolve => { entered = resolve; });
    const wait = () => new Promise(resolve => { release = resolve; entered(); });
    const f = fixture(stage === 'runtime' ? { loadRuntime: wait } : stage === 'lookup' ? { request: wait } : {});
    if (stage === 'hash') f.root.crypto = { subtle: { digest: wait } };
    let done;
    const run = f.run(() => new Promise(resolve => { done = resolve; }));
    await started;
    assert.equal(typeof release, 'function');
    done('uploaded');
    assert.deepEqual(await run, { kind: 'uploaded', result: 'uploaded' });
    release(stage === 'hash' ? new ArrayBuffer(32) : stage === 'runtime' ? {} : { payload: reused() });
    await tick();
    assert.equal(f.timers.size, 0);
    assert.equal(f.calls.length, stage === 'lookup' ? 1 : 0);
  }
});

test('miss, malformed response and lookup failure leave the original upload in control', async () => {
  for (const response of [{}, { reusable_library_file: null },
    { reusable_library_file: { ...reused().reusable_library_file, file_id: '../bad' } },
    { reusable_library_file: { ...reused().reusable_library_file, mime_type: 'image/png' } },
    { reusable_library_file: { ...reused().reusable_library_file, file_name: '../bad.txt' } }, 'error']) {
    const f = fixture({ request: async () => { if (response === 'error') throw new Error('synthetic failure'); return { payload: response }; } });
    const run = f.run(async signal => { await new Promise(resolve => setTimeout(resolve, 20)); assert.equal(signal.aborted, false); return 1; });
    assert.equal((await run).kind, 'uploaded'); assert.equal(f.timers.size, 0);
  }
});

test('temporary, project, non-library and oversized files never hash or query', async () => {
  for (const context of [{ storeInLibrary: false }, { isTemporaryChat: true }, { isProjectThread: true },
    { projectScopeId: 'project' }, { libraryFileInfo: {} }, { directoryId: 'folder' }, { uploadSource: 'connector' }]) {
    const f = fixture({ context }); assert.equal((await f.run(async () => 1)).kind, 'uploaded');
    assert.equal(f.imports.length, 0); assert.equal(f.calls.length, 0);
  }
  const f = fixture({ file: { size: protocol.maxFileBytes + 1, arrayBuffer: () => { throw new Error('must_not_read'); } } });
  await f.run(async () => 1); assert.equal(f.imports.length, 0);
});

test('an explicit upload-copy choice does not import, hash or query the library', async () => {
  const f = fixture({ context: { checkForReusableLibraryFile: false } });
  let uploads = 0;
  const result = await f.run(async () => { uploads++; await tick(); return 'fresh'; });
  assert.deepEqual(result, { kind: 'uploaded', result: 'fresh' });
  assert.equal(uploads, 1); assert.equal(f.imports.length, 0);
  assert.equal(f.calls.length, 0); assert.equal(f.timers.size, 0);
});

test('only the recognized current direct gate or enabled experiment permits reuse', async () => {
  const f = fixture({ gate: name => good(name, name !== '1342446482') });
  assert.equal((await f.run(cancelable)).kind, 'reused');
  for (const options of [{ loaded: false }, { loading: 'Loading' }, { gate: name => good(name, false) },
    { gate: name => ({ ...good(name, true), details: { reason: 'Uninitialized' } }) },
    { gate: name => ({ ...good(name, true), details: { reason: 'Network:Recognized', warnings: ['stale'] } }) },
    { gate: name => good(name, name !== '1342446482'), experiment: { get: () => false } }]) {
    const f = fixture(options);
    await f.run(async () => { await tick(); return 1; });
    assert.equal(f.calls.length, 0);
  }
});

test('a hit cannot mask a genuine byte failure or an upload that completed despite cancellation', async () => {
  const f = fixture();
  await assert.rejects(f.run(async signal => { try { await cancelable(signal); } catch (_) { throw new Error('timeout'); } }), /timeout/);
  const g = fixture();
  const result = await g.run(async signal => { try { await cancelable(signal); } catch (_) { return 'accepted'; } });
  assert.deepEqual(result, { kind: 'uploaded', result: 'accepted' });
});

test('account change and user cancellation cannot associate a late hit', async () => {
  for (const mode of ['account', 'cancel']) {
    const f = fixture({ request: async () => {
      if (mode === 'account') f.setCurrent(false); else f.controller.abort();
      return { payload: reused() };
    } });
    await assert.rejects(f.run(async signal => {
      if (mode === 'account') { await new Promise(resolve => setTimeout(resolve, 20)); throw new Error('cancelled'); }
      await cancelable(signal);
    }), /cancelled/);
    assert.equal(f.timers.size, 0);
  }
});
