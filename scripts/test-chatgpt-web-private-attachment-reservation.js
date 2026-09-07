'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const reservation = require('../android/app/src/main/assets/chatgpt_web_private_attachment_reservation.js');
const protocol = require('../android/app/src/main/assets/chatgpt_web_private_attachment_protocol.js');
const bytes = require('../android/app/src/main/assets/chatgpt_web_private_attachment_bytes.js');
const tick = () => new Promise(resolve => setImmediate(resolve));
const context = () => ({ useCase: 'ace_upload', storeInLibrary: false,
  libraryPersistenceMode: 'required', indexForRetrieval: false, isTemporaryChat: false });
const file = () => new File(['synthetic file'], 'fixture.txt', { type: 'text/plain' });
const stamp = Date.parse('2026-09-07T00:00:00Z');
const response = () => ({ eligible: true, reservation_id: 'reservation-synthetic',
  upload_url: 'https://uploads.oaiusercontent.com/fixture?sig=synthetic',
  upload_url_expires_at: new Date(stamp + 120000).toISOString(),
  reservation_expires_at: new Date(stamp + 180000).toISOString() });

function fixture(options = {}) {
  const requests = [], imports = [], experiments = [], timers = new Map();
  let current = true, time = stamp, serial = 0;
  const binding = {};
  const controller = new AbortController();
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' },
    AbortController, performance: { getEntriesByName: () => options.loaded === false ? [] : [{}] },
    setTimeout: callback => { timers.set(++serial, callback); return serial; },
    clearTimeout: id => timers.delete(id) };
  const experiment = { name: '3119290944', groupName: 'treatment', details: { reason: 'Network:Recognized' },
    get: (key, fallback) => { assert.equal(key, 'enable'); assert.equal(fallback, false); return true; },
    ...options.experiment };
  const client = { loadingStatus: 'Ready', getExperiment: (id, flags) => {
    experiments.push({ id, flags }); return experiment;
  }, getFeatureGate: () => { throw new Error('wrong_experiment_api'); }, ...options.client };
  const headers = { authorization: 'Bearer synthetic-page-token', 'Content-Type': 'application/json' };
  const instance = reservation.create(root, { protocol, bytes, now: () => time,
    isCurrent: candidate => current && candidate === binding,
    acquireHeaders: options.acquireHeaders || (async () => headers),
    loadRuntime: async url => { imports.push(url); return options.loadRuntime ? options.loadRuntime() : { t6: () => client }; },
    request: async (_, url, init, limits) => {
      requests.push({ url, init, limits });
      return options.request ? options.request(url, init, limits) : { payload: { ...response(), ...options.payload } };
    } });
  return { instance, root, binding, controller, requests, imports, experiments, timers,
    context: context(), file: file(), setCurrent: next => { current = next; }, setTime: next => { time = next; },
    start(value) { return instance.start(value || this.context, binding, controller.signal); },
    take(value, candidate) { return instance.take(this.file, value || this.context, candidate || binding); } };
}

test('reservation uses the current recognized experiment and the exact same-origin allocation contract', async () => {
  const f = fixture();
  assert.equal(f.start(), undefined, 'starting allocation must not return a promise to await');
  await tick();
  assert.deepEqual(f.experiments, [{ id: '3119290944', flags: { disableExposureLog: true } }]);
  assert.equal(f.imports.length, 1);
  assert.match(f.imports[0], /4813494d-hrplraurzfyvxb10\.js$/);
  assert.equal(f.requests.length, 1);
  const request = f.requests[0];
  assert.equal(request.url, '/backend-api/files/upload_reservations');
  assert.equal(request.init.method, 'POST');
  assert.equal(request.init.credentials, 'include');
  assert.equal(request.init.redirect, 'error');
  assert.deepEqual(JSON.parse(request.init.body), { intended_use_case: 'ace_upload', entry_surface: 'chat_composer',
    requires_gizmo_id: false, store_in_library: false, library_persistence_mode: 'required' });
  assert.equal(request.limits.timeoutMs, 3000);
  assert.equal(request.limits.maxBytes, 16384);
  const result = f.take();
  assert.equal(result.entry.file_id, 'reservation-synthetic');
  assert.equal(result.claim.url, '/backend-api/files/upload_reservations/reservation-synthetic/claim_and_finish');
  assert.deepEqual(JSON.parse(result.claim.body), { file_name: f.file.name, file_size: f.file.size,
    use_case: 'ace_upload', index_for_retrieval: false, store_in_library: false, library_persistence_mode: 'required',
    mime_type: 'text/plain', entry_surface: 'chat_composer',
    metadata: { store_in_library: false, is_temporary_chat: false, is_project_thread: false } });
  assert.equal(f.take(), null, 'a reservation cannot be claimed twice');
  assert.equal(f.timers.size, 0);
});

test('an unready slot is abandoned synchronously and a late response cannot change the chosen route', async () => {
  let resolve;
  const f = fixture({ request: () => new Promise(done => { resolve = done; }) });
  f.start(); await tick();
  assert.equal(f.requests.length, 1);
  assert.equal(f.take(), null);
  assert.equal(f.requests[0].init.signal.aborted, true);
  resolve({ payload: response() }); await tick();
  assert.equal(f.take(), null);
  assert.equal(f.requests.length, 1);
});

test('unknown, control or unrecognized experiments do not allocate or infer missing functionality', async () => {
  for (const patch of [{ loaded: false }, { client: { loadingStatus: 'Loading' } },
    { experiment: { name: 'other' } }, { experiment: { groupName: null } }, { experiment: { get: () => false } },
    { experiment: { get: () => 'true' } }, { experiment: { details: { reason: 'Uninitialized' } } },
    { experiment: { details: { reason: 'Network:Recognized', warnings: ['stale'] } } }]) {
    const f = fixture(patch); f.start(); await tick();
    assert.equal(f.requests.length, 0);
    assert.equal(f.take(), null);
  }
});

test('project, mismatched temporary persistence, library and source scopes cannot allocate slots', async () => {
  for (const patch of [{ isProjectThread: true }, { isTemporaryChat: true }, { isTemporaryChat: 'false' },
    { projectScopeId: 'g-p-synthetic' }, { gizmoId: 'g-p-synthetic' }, { libraryFileInfo: {} },
    { directoryId: 'directory' }, { uploadSource: 'connector' }, { libraryPersistenceMode: undefined },
    { useCase: 'gizmo' }, { storeInLibrary: undefined }]) {
    const f = fixture(); f.start({ ...f.context, ...patch }); await tick();
    assert.equal(f.requests.length, 0);
    assert.equal(f.take(), null);
    assert.equal(f.imports.length, 0);
  }
});

test('temporary reservations default only their slot persistence while retaining non-library processing', async () => {
  const f = fixture();
  f.context = { ...context(), isTemporaryChat: true, libraryPersistenceMode: undefined };
  f.start(); await tick();
  assert.equal(f.requests.length, 1);
  assert.deepEqual(JSON.parse(f.requests[0].init.body), {
    intended_use_case: 'ace_upload', entry_surface: 'chat_composer', requires_gizmo_id: false,
    store_in_library: false, library_persistence_mode: 'required',
  });
  const result = f.take();
  assert.ok(result);
  const claim = JSON.parse(result.claim.body);
  assert.equal(claim.library_persistence_mode, 'required');
  assert.equal(claim.store_in_library, false);
  assert.equal(claim.metadata.is_temporary_chat, true);
  assert.equal(claim.metadata.store_in_library, false);
  assert.equal(claim.index_for_retrieval, false);
  assert.equal(f.context.libraryPersistenceMode, undefined, 'do not rewrite the legacy create/process contract');
  assert.equal(Object.hasOwn(protocol.prepare(f.file, f.context), 'library_persistence_mode'), false);
  assert.equal(f.take(), null);
});

test('temporary slots reject library persistence or indexing and cannot transfer to ordinary chats', async () => {
  for (const patch of [{ storeInLibrary: true }, { indexForRetrieval: true },
    { libraryPersistenceMode: 'opportunistic' }, { libraryPersistenceMode: 'required' }]) {
    const f = fixture();
    f.context = { ...context(), isTemporaryChat: true, libraryPersistenceMode: undefined, ...patch };
    f.start(); await tick();
    assert.equal(f.requests.length, 0);
    assert.equal(f.take(), null);
  }
  const f = fixture();
  f.context = { ...context(), isTemporaryChat: true, libraryPersistenceMode: undefined };
  f.start(); await tick();
  assert.equal(f.take(context()), null, 'same normalized persistence does not make the scopes interchangeable');
});

test('both expiry clocks must leave at least sixty seconds at consumption', async () => {
  for (const field of ['upload_url_expires_at', 'reservation_expires_at']) {
    for (const remaining of [-1, 59999, 60000]) {
      const f = fixture({ payload: { [field]: new Date(stamp + remaining).toISOString() } });
      f.start(); await tick();
      assert.equal(!!f.take(), remaining >= 60000);
    }
  }
  const f = fixture(); f.start(); await tick(); f.setTime(stamp + 61000);
  assert.equal(f.take(), null, 'a formerly usable response is not a permanent cache');
});

test('malformed responses and untrusted destinations are discarded without exposing a URL', async () => {
  for (const payload of [{ eligible: false, reason: 'gate_disabled' }, { reservation_id: '../../x' },
    { upload_url_expires_at: 'invalid' }, { reservation_expires_at: stamp + 120000 }, { upload_url: '' },
    { upload_url: 'https://localhost/secret' }, { upload_url: 'http://uploads.oaiusercontent.com/a?sig=x' },
    { upload_url: 'https://chatgpt.com/backend-api/arbitrary' }, { upload_url: 'https://user:pass@chatgpt.com/api/estuary/upload_content_bytes' },
    { upload_headers: { authorization: 'unexpected' } }, { direct_library_upload_strategy: {} }]) {
    const f = fixture({ payload }); f.start(); await tick();
    assert.equal(f.take(), null);
  }
});

test('same-origin Estuary reservations retain the exact returned relative forwarding value', async () => {
  const path = '/backend-api/estuary/upload_content_bytes';
  const f = fixture({ payload: { upload_url: path } }); f.start(); await tick();
  const selected = f.take();
  assert.equal(selected.entry.upload_url, path);
  const plan = bytes.plan(selected.entry, f.file, protocol);
  assert.equal(plan.kind, 'estuary_bytes');
  assert.equal(plan.forwardUrl, path);
});

test('identity, page, model, persistence and owner changes invalidate the slot', async () => {
  for (const change of [f => f.setCurrent(false), f => { f.root.location.href += 'c/other'; },
    f => { f.context.modelSlug = 'other'; }, f => { f.context.storeInLibrary = true; },
    f => { f.context.useCase = 'my_files'; }, f => { f.context.isTemporaryChat = true; }]) {
    const f = fixture(); f.start(); await tick(); change(f);
    assert.equal(f.take(), null);
    assert.equal(f.take(), null);
  }
  const f = fixture(); f.start(); await tick(); assert.equal(f.take(f.context, {}), null);
});

test('cancel and total deadline prevent dispatch after delayed runtime or identity resolution', async () => {
  for (const stage of ['runtime', 'identity']) {
    let release;
    const wait = () => new Promise(resolve => { release = resolve; });
    const f = fixture(stage === 'runtime' ? { loadRuntime: wait } : { acquireHeaders: wait });
    f.start(); await tick();
    for (const callback of [...f.timers.values()]) callback();
    release(stage === 'runtime' ? { t6: () => ({ loadingStatus: 'Ready' }) } : { authorization: 'Bearer late' });
    await tick();
    assert.equal(f.requests.length, 0);
    assert.equal(f.take(), null);
  }
});

test('external cancellation aborts allocation and old completions cannot overwrite a replacement slot', async () => {
  const pending = [];
  const f = fixture({ request: () => new Promise(resolve => pending.push(resolve)) });
  f.start(); await tick(); f.start(); await tick();
  assert.equal(f.requests[0].init.signal.aborted, true);
  pending[1]({ payload: { ...response(), reservation_id: 'reservation-new' } }); await tick();
  pending[0]({ payload: response() }); await tick();
  assert.equal(f.take().entry.file_id, 'reservation-new');
  f.start(); await tick(); f.controller.abort();
  assert.equal(f.requests.at(-1).init.signal.aborted, true);
  pending[2]({ payload: response() }); await tick();
  assert.equal(f.take(), null);
  assert.equal(f.timers.size, 0);
});

test('allocation rejection is contained without an automatic retry', async () => {
  const f = fixture({ request: async () => { throw new Error('synthetic secret server text'); } });
  f.start(); await tick();
  assert.equal(f.take(), null);
  assert.equal(f.requests.length, 1);
});

test('claim metadata preserves image geometry, document MIME and filename use-case conversion', async () => {
  for (const value of [{ name: 'image.png', type: 'image/png', useCase: 'multimodal', imageDimensions: { width: 30, height: 20 } },
    { name: 'fixture.pdf', type: 'application/pdf', useCase: 'ace_upload' },
    { name: 'fixture.hwpx', type: 'application/vnd.hancom.hwpx', useCase: 'ace_upload' }]) {
    const f = fixture();
    f.context = { ...context(), useCase: value.useCase };
    f.file = new File(['synthetic data'], value.name, { type: value.type });
    f.start(); await tick();
    const claim = JSON.parse(f.take({ ...f.context, imageDimensions: value.imageDimensions }).claim.body);
    assert.equal(claim.mime_type, value.type);
    assert.equal(claim.file_size, f.file.size);
    assert.equal(claim.use_case, value.name.endsWith('hwpx') ? 'my_files' : value.useCase);
    assert.equal(claim.index_for_retrieval, value.name.endsWith('hwpx'));
    if (value.imageDimensions) {
      assert.equal(claim.width, 30); assert.equal(claim.height, 20);
    }
  }
});

test('large direct-library uploads retain their existing multipart route', async () => {
  const f = fixture(); f.context.storeInLibrary = true;
  f.file = new File([new Uint8Array(2 * 1024 * 1024 + 1)], 'large.txt', { type: 'text/plain' });
  f.start(); await tick(); assert.equal(f.take(), null);
});

test('enabled library intent preserves opportunistic persistence in allocation and claim', async () => {
  const f = fixture();
  f.context = { ...context(), storeInLibrary: true, libraryPersistenceMode: 'opportunistic' };
  f.start(); await tick();
  const allocation = JSON.parse(f.requests[0].init.body);
  const claim = JSON.parse(f.take().claim.body);
  assert.equal(allocation.store_in_library, true);
  assert.equal(allocation.library_persistence_mode, 'opportunistic');
  assert.equal(claim.store_in_library, true);
  assert.equal(claim.library_persistence_mode, 'opportunistic');
  assert.equal(claim.metadata.store_in_library, true);
});
