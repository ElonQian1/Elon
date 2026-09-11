'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { webcrypto } = require('node:crypto');
const path = require('node:path');
const fs = require('node:fs');
const vm = require('node:vm');
const asset = name => path.join(__dirname, '../android/app/src/main/assets/chatgpt_web_private_' + name + '.js');
const catalog = require(asset('library_catalog'));
const expected = require('../android/app/src/test/resources/webchat/private-library-sources-contract.json');
const file = (n, extra = {}) => ({ kind: 'file', id: 'libfile_private_' + n, file_id: 'file-private-' + n,
  name: 'private-filename-' + n + '.png', mime_type: 'image/png', file_size_bytes: 100, ...extra });

function fixture() {
  let rows = [file(1), file(2), file(3, { library_artifact_type: 'saved_entity', saved_entity: { private: 'private-content' } })];
  let auth = 'Bearer private-token-fixture', sequence = 0;
  const events = [], requests = [];
  const root = { location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/private-context' },
    __elonChatGptDocumentToken: 'doc_private_sources', crypto: webcrypto, AbortController, setTimeout, clearTimeout,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: auth }) },
    __elonChatGptPrivateJsonRequest: { async request(_, url, init) {
      requests.push({ url, init }); if (rows instanceof Error) throw rows; return { payload: { items: rows } };
    } }, elonChatGptFileDownload: { postMessage() { throw new Error('must_not_download'); } },
    __elonChatGptPrivateLibraryAttachment: require(asset('library_attachment')),
    __elonChatGptPrivateAttachmentProtocol: require(asset('attachment_protocol')),
    __elonChatGptPrivateLibraryDownload: require(asset('library_download')),
  };
  Object.defineProperty(root, 'document', { get() { throw new Error('must_not_read_DOM'); } });
  root.__elonChatGptPrivateFileDownload = require(asset('file_download')).create(root);
  root.__elonChatGptPrivateLibraryMutations = require(asset('library_mutations')).create(root);
  const service = catalog.create(root);
  const list = async (operation = 'open') => {
    let receipt;
    await service.list({ requestId: 'mcp_' + ++sequence, value: JSON.stringify({ operation }) },
      event => events.push(event), (...value) => { receipt = value; });
    return receipt;
  };
  return { root, service, list, events, requests, setRows: value => { rows = value; }, setAuth: value => { auth = value; } };
}

test('real catalog/download/attachment/mutation owners produce the shared native contract without DOM', async () => {
  const f = fixture();
  assert.equal(f.service.sourceDiagnostics(), null);
  assert.equal((await f.list())[1], true);
  const before = JSON.stringify(f.events);
  assert.deepEqual(f.service.sourceDiagnostics(), expected);
  assert.deepEqual(f.service.sourceDiagnostics(), expected);
  assert.equal(f.requests.length, 1, 'diagnostics issue no request or materialization');
  assert.equal(JSON.stringify(f.events), before, 'no extra snapshot or renewed handles');
  const json = JSON.stringify(f.service.sourceDiagnostics());
  for (const secret of ['private-', 'private_', 'private-content', 'Authorization', 'Bearer', 'https:', 'handle']) {
    assert.equal(json.includes(secret), false, secret);
  }
  f.service.sourceDiagnostics().groups[0].count = 90;
  assert.deepEqual(f.service.sourceDiagnostics(), expected, 'caller cannot alter cached metadata');
});

test('account/document/route/transport changes and disposal revoke diagnostics', async () => {
  for (const change of [f => f.setAuth('Bearer private-new-account'),
    f => { f.root.__elonChatGptDocumentToken = 'doc_replaced_sources'; },
    f => { f.root.location.href = 'https://chatgpt.com/c/elsewhere'; },
    f => { f.root.__elonChatGptPrivateTransport = { ...f.root.__elonChatGptPrivateTransport }; },
    f => f.service.dispose()]) {
    const f = fixture(); await f.list(); change(f); assert.equal(f.service.sourceDiagnostics(), null);
  }
});

test('raster artifacts expose download only while special artifacts remain unclaimed', async () => {
  const f = fixture();
  f.setRows([file(1, { library_artifact_type: 'raster_fixture' }), file(2, { library_artifact_type: 'saved_entity' })]);
  await f.list();
  const [image, unknown] = f.service.sourceDiagnostics().groups;
  assert.equal(image.artifact, 'other');
  assert.equal(image.download, true);
  assert.equal(image.attach || image.rename || image.trash, false);
  assert.equal(unknown.artifact, 'saved_entity');
  assert.equal(unknown.download, false);
});

test('explicit refresh failure retains stale evidence but expiry and mutation invalidate it', async t => {
  const f = fixture(); await f.list();
  f.setRows(new Error('timeout'));
  assert.equal((await f.list('refresh'))[1], false);
  assert.equal(f.service.sourceDiagnostics().stale, true);
  const now = Date.now(); t.mock.method(Date, 'now', () => now + 60001);
  assert.equal(f.service.sourceDiagnostics(), null); t.mock.restoreAll();
  const g = fixture(); await g.list();
  g.service.selectMutation(g.events.at(-1).items[0].handle).settle(true, 'rename', 'renamed.png');
  assert.equal(g.service.sourceDiagnostics(), null);
});

test('source classification is bounded and never interprets unknown flags as a capability', async () => {
  const f = fixture();
  const rows = Array.from({ length: 128 }, (_, index) => file(index, {
    ...(index & 1 ? { context_scopes: { private: 'private-content' } } : {}),
    ...(index & 2 ? { preview_file: {} } : {}), ...(index & 4 ? { cloud_doc_url: 'https://private.invalid' } : {}),
    ...(index & 8 ? { external_account: {} } : {}), ...(index & 16 ? { saved_entity: {} } : {}),
    ...(index & 32 ? { gizmo_id: 'private-project' } : {}), ...(index & 64 ? { library_artifact_type: 'private-artifact' } : {}),
  }));
  f.setRows(rows); await f.list();
  const result = f.service.sourceDiagnostics();
  assert.equal(result.groups.length, 16); assert.equal(result.omitted, 112); assert.equal(result.total, 128);
  assert.equal(result.groups.reduce((sum, group) => sum + group.count, result.omitted), 128);
  assert.ok(JSON.stringify(result).length < 12000);
  assert.equal(JSON.stringify(result).includes('private'), false);
  f.setRows([file(1, { id: 'external-gdrive:file:synthetic', library_artifact_type: 'private-artifact' })]);
  await f.list('refresh');
  assert.equal(f.service.sourceDiagnostics().groups[0].node, 'external');
  assert.equal(f.service.sourceDiagnostics().groups[0].artifact, 'other');
});

test('module upgrade adds diagnostics and disposes only the prior catalog owner', () => {
  const f = fixture(); let disposed = 0;
  const transport = f.root.__elonChatGptPrivateTransport, download = f.root.__elonChatGptPrivateFileDownload;
  f.root.__elonChatGptPrivateLibraryCatalog = { version: 8, dispose() { disposed++; } };
  const context = { window: f.root, URL, Date, Uint8Array };
  vm.runInNewContext(fs.readFileSync(asset('library_catalog'), 'utf8'), context);
  const fresh = f.root.__elonChatGptPrivateLibraryCatalog;
  assert.equal(disposed, 1); assert.equal(typeof fresh.sourceDiagnostics, 'function');
  vm.runInNewContext(fs.readFileSync(asset('library_catalog'), 'utf8'), context);
  assert.equal(f.root.__elonChatGptPrivateLibraryCatalog, fresh);
  assert.equal(f.root.__elonChatGptPrivateTransport, transport);
  assert.equal(f.root.__elonChatGptPrivateFileDownload, download);
});
