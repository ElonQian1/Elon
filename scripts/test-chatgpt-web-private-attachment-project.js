'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const assets = '../android/app/src/main/assets/';
const projectModule = require(assets + 'chatgpt_web_private_attachment_project.js');
const composerModule = require(assets + 'chatgpt_web_private_attachment_composer.js');
const sendModule = require(assets + 'chatgpt_web_private_attachment_send.js');
const transportModule = require(assets + 'chatgpt_web_private_attachment_transport.js');
const protocol = require(assets + 'chatgpt_web_private_attachment_protocol.js');
const jsonRequest = require(assets + 'chatgpt_web_private_json_request.js');
const PROJECT = 'g-p-0123456789abcdef0123456789abcdef';
const OTHER = 'g-p-fedcba9876543210fedcba9876543210';

function fixture(options = {}) {
  let values = [], reads = 0, fallbacks = 0, account = 'Bearer synthetic-page-token';
  const calls = [], receipts = [];
  const files$ = () => values;
  files$.set = value => { values = value; };
  const store = { files$, readyFiles$: () => values.filter(file => file.status === 'ready'),
    hasUploadInProgress$: () => values.some(file => file.status === 'uploading') };
  const input = { isConnected: true, __reactFiber$fixture: { memoizedProps: { value: store } } };
  const headers = () => ({ authorization: account, 'chatgpt-account-id': 'synthetic-account' });
  const payload = { gizmo: { id: PROJECT, current_user_permission: { can_write: true }, use_injest_path: false } };
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/g/' + PROJECT + '-synthetic/project' },
    document: { querySelector: selector => selector === '#upload-files' ? input : {} },
    __elonChatGptDocumentToken: 'doc_synthetic_project',
    __elonChatGptComposer: { currentModel: () => 'synthetic-model' },
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: headers, acquireSameOriginRequestHeaders: async () => headers() },
    __elonChatGptPrivateAttachmentProject: projectModule,
    __elonChatGptPrivateAttachmentTransport: transportModule,
    __elonChatGptPrivateAttachmentProtocol: protocol,
    __elonChatGptPrivateJsonRequest: jsonRequest,
    AbortController, setTimeout, clearTimeout, setInterval, clearInterval,
    fetch: async (url, init) => {
      calls.push({ url, init });
      if (options.fetch) return options.fetch(url, init);
      if (init.method === 'GET') {
        options.beforePermission?.(root);
        return Response.json(payload);
      }
      if (url === '/backend-api/files') return Response.json({ status: 'success', file_id: 'file-synthetic',
        upload_url: 'https://uploads.oaiusercontent.com/fixture?sig=synthetic' });
      if (init.method === 'PUT') return new Response('');
      if (url === '/backend-api/files/process_upload_stream') return new Response(JSON.stringify({
        event: 'file.processing.completed', file_id: 'file-synthetic', progress: 100,
        extra: { total_tokens: 6, metadata_object_id: 'library-synthetic', library_persistence_result: 'library' },
      }));
      throw new Error('unexpected_dispatch');
    },
  };
  const file = options.file || new File(['synthetic project fixture'], 'fixture.txt', { type: 'text/plain' });
  const descriptor = { version: 1, leaseId: '00000000-0000-4000-8000-000000000000',
    documentToken: root.__elonChatGptDocumentToken, href: root.location.href,
    name: file.name, size: file.size, type: file.type };
  const composer = composerModule.create(root);
  const send = sendModule.create(root, { composer,
    source: { read: async () => { reads++; return file; } },
    image: { available: () => true, prepare: async () => ({ file, dimensions: { width: 12, height: 8 } }) },
  });
  return { root, file, descriptor, payload, composer, send, store, calls, receipts,
    start: () => send.start(JSON.stringify(descriptor), (...args) => receipts.push(args), () => {}, () => { fallbacks++; }),
    reads: () => reads, fallbacks: () => fallbacks, setAccount: value => { account = value; } };
}

test('new project private upload reads permission then binds create/process/ready-store to one project', async () => {
  const f = fixture();
  await f.start();
  assert.equal(f.fallbacks(), 0);
  assert.equal(f.receipts[0][1], true);
  assert.equal(f.reads(), 1);
  assert.equal(f.calls.length, 4);
  const read = f.calls[0];
  assert.equal(read.url, '/backend-api/gizmos/' + PROJECT);
  assert.equal(read.init.cache, 'no-store');
  assert.equal(read.init.redirect, 'error');
  const create = JSON.parse(f.calls[1].init.body), process = JSON.parse(f.calls[3].init.body);
  assert.equal(create.gizmo_id, PROJECT);
  assert.equal(create.use_case, 'gizmo');
  assert.equal(process.gizmo_id, PROJECT);
  assert.equal(process.index_for_retrieval, false);
  assert.equal(process.metadata.is_project_thread, true);
  assert.deepEqual(process.metadata.library_file_info, { gizmo_id: PROJECT, is_project: true, should_upload_to_project: true });
  assert.equal(f.calls[2].init.credentials, 'omit');
  assert.equal(f.calls[2].init.headers.authorization, undefined);
  const ready = f.store.readyFiles$()[0];
  assert.equal(ready.projectGizmoId, PROJECT);
  assert.equal(ready.isProjectThread, true);
  assert.equal(ready.libraryFileId, 'library-synthetic');
  assert.equal(ready.fileSpec.libraryFileId, 'library-synthetic');
  assert.equal(ready.fileSpec.libraryPersistenceResult, 'library');
  assert.equal(f.composer.remove(f.composer.merge([])[0].id), true);
  assert.equal(f.store.files$().length, 0);
  assert.equal(f.calls.length, 4, 'no implicit text send, duplicate upload or remote delete');
});

test('read-only, unknown or mismatched project evidence selects compatibility before reading bytes', async () => {
  for (const gizmo of [null, {}, { id: OTHER, current_user_permission: { can_write: true } },
    { id: PROJECT }, { id: PROJECT, current_user_permission: { can_write: false } },
    { id: PROJECT, current_user_permission: { can_write: 'true' } },
    { id: PROJECT, current_user_permission: { can_write: true }, use_injest_path: 'true' }]) {
    const f = fixture();
    f.payload.gizmo = gizmo;
    await f.start();
    assert.equal(f.calls.length, 1);
    assert.equal(f.reads(), 0);
    assert.equal(f.fallbacks(), 1);
    assert.equal(f.store.files$().length, 0);
  }
});

test('changing project, identity, document or cancelling during permission read cannot upload or replay', async () => {
  for (const change of ['project', 'account', 'document', 'cancel']) {
    let f;
    f = fixture({ beforePermission: root => {
      if (change === 'project') root.location.href = root.location.href.replace(PROJECT, OTHER);
      if (change === 'account') f.setAccount('Bearer another-synthetic-token');
      if (change === 'document') root.__elonChatGptDocumentToken = 'doc_another';
      if (change === 'cancel') f.send.cancel();
    } });
    await f.start();
    assert.equal(f.calls.length, 1);
    assert.equal(f.reads(), 0);
    assert.equal(f.fallbacks(), 0);
    assert.equal(f.receipts[0][1], false);
  }
});

test('project routes exclude existing branches, unknown IDs and combined temporary mode', () => {
  for (const path of ['/g/' + PROJECT + '/c/00000000-0000-4000-8000-000000000000',
    '/g/g-p-unconfirmed/project', '/g/' + PROJECT + '/project?temporary-chat=true',
    '/g/' + PROJECT + '/project?model=unknown']) {
    const f = fixture();
    f.root.location.href = 'https://chatgpt.com' + path;
    assert.equal(f.composer.available(), false);
  }
});

test('project image keeps multimodal dimensions without inventing an unobserved retrieval flag', async () => {
  const file = new File(['synthetic image bytes'], 'fixture.png', { type: 'image/png' });
  for (const ingest of [false, true]) {
    const f = fixture({ file });
    f.payload.gizmo.use_injest_path = ingest;
    await f.start();
    assert.equal(f.fallbacks(), ingest ? 1 : 0);
    assert.equal(f.reads(), ingest ? 0 : 1);
    if (ingest) { assert.equal(f.calls.length, 1); continue; }
    const create = JSON.parse(f.calls[1].init.body), process = JSON.parse(f.calls[3].init.body);
    assert.equal(create.use_case, 'multimodal');
    assert.equal(Object.hasOwn(create, 'gizmo_id'), false);
    assert.equal(Object.hasOwn(process, 'gizmo_id'), false);
    assert.equal(process.metadata.library_file_info.gizmo_id, PROJECT);
    assert.equal(f.store.readyFiles$()[0].fileSpec.width, 12);
  }
});

test('project retrieval follows observed spreadsheet suffix rules and exact permission is refreshed per upload', async () => {
  const scope = { projectId: PROJECT, usesInjestPath: true }, project = projectModule.create({});
  for (const [name, indexed] of [['fixture.txt', false], ['fixture.CSV', true], ['fixture.xlsx', true], ['fixture.pdf', false]]) {
    const options = project.uploadContext(scope, { name, type: 'text/plain' });
    assert.equal(options.indexForRetrieval, indexed);
  }
  const f = fixture();
  await f.start();
  f.composer.remove(f.composer.merge([])[0].id);
  f.payload.gizmo.current_user_permission.can_write = false;
  await f.start();
  assert.equal(f.calls.filter(call => call.init.method === 'GET').length, 2);
  assert.equal(f.calls.filter(call => call.init.method === 'POST').length, 2);
  assert.equal(f.fallbacks(), 1);
});

test('project protocol rejects cross-project and incompatible privacy metadata before any write', () => {
  const f = fixture(), project = projectModule.create(f.root);
  const options = project.uploadContext({ projectId: PROJECT, usesInjestPath: false }, f.file);
  for (const override of [{ gizmoId: OTHER }, { isTemporaryChat: true }, { useCase: 'ace_upload' },
    { isProjectThread: false }, { libraryFileInfo: { ...options.libraryFileInfo, origination_thread_id: 'unconfirmed' } }]) {
    assert.throws(() => protocol.prepare(f.file, { ...options, ...override }), /unsupported_upload_context/);
  }
});

test('pending transport snapshots project metadata and association rejects a different project receipt', async () => {
  const f = fixture(), binding = f.composer.capture();
  assert.equal(await f.composer.prepare(binding, undefined, f.descriptor), true);
  const options = { ...f.composer.uploadContext(binding, f.file), libraryFileInfo: {
    gizmo_id: PROJECT, is_project: true, should_upload_to_project: true } };
  const transport = transportModule.create(f.root, { isCurrent: () => f.composer.current(binding),
    acquireHeaders: async () => {
      options.libraryFileInfo.gizmo_id = OTHER;
      return f.root.__elonChatGptPrivateTransport.copySameOriginRequestHeaders();
    } });
  const result = await transport.upload(f.file, options, binding);
  assert.equal(result.ok, true);
  assert.equal(result.projectId, PROJECT);
  assert.equal(JSON.parse(f.calls.at(-1).init.body).metadata.library_file_info.gizmo_id, PROJECT);
  assert.throws(() => f.composer.associate(binding, f.file, { ...result, projectId: OTHER }, f.descriptor.leaseId), /association_invalid/);
  assert.equal(f.store.files$().length, 0);
});
