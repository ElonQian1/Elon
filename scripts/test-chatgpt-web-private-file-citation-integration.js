'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { webcrypto } = require('node:crypto');
const projectionModule = require('../android/app/src/main/assets/chatgpt_web_private_history_projection.js');
const download = require('../android/app/src/main/assets/chatgpt_web_private_file_download.js');
const request = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const projection = projectionModule.create({});
const PROJECT = 'g-p-0123456789abcdef0123456789abcdef';
const ref = (id = 'file-cited') => ({ type: 'file', id, name: 'cited.txt', content_type: 'text/plain',
  cloud_doc_url: 'https://cloud.example.test/attribution?token=synthetic', snippet: 'not a native file field' });
const message = (id = 'answer', refs = [ref()]) => ({ id, author: { role: 'assistant' },
  content: { parts: ['Answer'] }, metadata: { content_references: refs } });

function fixture() {
  const calls = [], queued = [], receipts = [];
  let account = 'Bearer synthetic-citation-test';
  const payload = { messages: [message()] };
  const bridge = { onmessage: null, postMessage(raw) {
    const value = JSON.parse(raw);
    if (value.cancel === true) return;
    queued.push(value);
    queueMicrotask(() => bridge.onmessage?.({ data: JSON.stringify({ leaseId: value.leaseId, state: 'queued' }) }));
  } };
  const root = {
    location: { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/c/visible' },
    __elonChatGptDocumentToken: 'doc_citation_1',
    __elonChatGptPrivateHistoryProjection: projectionModule,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => ({ Authorization: account }) },
    __elonChatGptPrivateJsonRequest: request,
    elonChatGptFileDownload: bridge, AbortController, crypto: webcrypto, setTimeout, clearTimeout,
    get document() { throw new Error('The download must not wait for or inspect DOM'); },
    fetch: async (url, init) => {
      calls.push({ url, init });
      if (new URL(url).pathname.endsWith('/simple')) {
        return Response.json({ file_id: 'file-cited', library_file_id: 'libfile_cited',
          is_library_file: true, is_project: true, gizmo_id: PROJECT });
      }
      return Response.json({ status: 'success', file_id: 'file-cited',
        download_url: 'https://files.oaiusercontent.com/synthetic-download' });
    },
  };
  const api = download.create(root);
  const rows = () => api.register('/c/source', payload, projection.files(payload));
  const descriptor = row => JSON.stringify({ version: 1,
    leaseId: '00000000-0000-4000-8000-000000000001', documentToken: root.__elonChatGptDocumentToken,
    href: root.location.href, path: '/c/source', name: row.name, downloadHandle: row.downloadHandle });
  const run = row => api.start(descriptor(row || rows()[0]), (...args) => receipts.push(args));
  return { root, payload, api, calls, queued, receipts, rows, run, setAccount(value) { account = value; } };
}

test('a reply containing only a file citation enters the native index with an opaque download handle', async () => {
  const f = fixture();
  const rows = f.rows();
  assert.equal(rows[0].name, 'cited.txt');
  assert.equal(rows[0].mediaType, 'text/plain');
  assert.match(rows[0].downloadHandle, /^download_[a-f0-9]{32}$/);
  assert.doesNotMatch(JSON.stringify({ rows, messages: projection.project(f.payload) }), /file-cited|https|token|snippet/);
  assert.equal(projection.fileSource(f.payload, rows[0].id).fileCitationReference.id, 'file-cited');
  await f.run(rows[0]);
  assert.equal(f.calls.length, 1);
  const url = new URL(f.calls[0].url);
  assert.equal(url.pathname, '/backend-api/files/download/file-cited');
  assert.equal(url.searchParams.get('conversation_id'), 'source');
  assert.equal(url.searchParams.get('download_intent'), 'true');
  assert.equal(f.calls[0].init.method, 'GET');
  assert.equal(f.queued.length, 1);
  assert.deepEqual(f.receipts, [['download_conversation_file', true, 'download_queued']]);
  assert.equal(f.root.location.href, 'https://chatgpt.com/c/visible');
});

test('library citations verify effective project metadata before authorizing any byte transfer', async () => {
  const f = fixture();
  f.payload.messages[0].metadata.content_references[0].library_file_id = 'libfile_cited';
  await f.run();
  assert.equal(f.calls.length, 2);
  assert.equal(new URL(f.calls[0].url).pathname, '/backend-api/files/file-cited/simple');
  const url = new URL(f.calls[1].url);
  assert.equal(url.searchParams.get('gizmo_id'), PROJECT);
  assert.equal(url.searchParams.get('check_context_scopes_for_conversation_id'), 'source');
  assert.equal(f.queued.length, 1);
});

test('ambiguous library scope and project mismatches never drop into unscoped authorization', async () => {
  const f = fixture();
  f.payload.messages[0].metadata.content_references[0].library_file_id = 'libfile_cited';
  f.root.fetch = async () => Response.json({ is_library_file: true, library_file_id: 'libfile_other' });
  await f.run();
  assert.equal(f.queued.length, 0);
  assert.deepEqual(f.receipts, [['download_conversation_file', false, 'download_prepare_failed']]);
  const g = fixture();
  g.payload.gizmo_id = PROJECT;
  g.payload.messages[0].metadata.content_references[0].gizmo_id = 'g-p-fedcba9876543210fedcba9876543210';
  assert.equal(g.rows()[0].downloadHandle, undefined);
});

test('citation appends preserve prior row IDs and deduplicate ordinary and shared files', () => {
  const m = message('answer', [ref('file-original'), ref(), ref()]);
  m.metadata.attachments = [{ id: 'file-original', name: 'original.txt' }];
  m.metadata.shared_library_file_references = [{ library_file_id: 'libfile_shared', name: 'shared.txt' }];
  m.metadata.content_references.push({ ...ref('file-shared'), library_file_id: 'libfile_shared' });
  const payload = { messages: [m] }, rows = projection.files(payload).files;
  assert.deepEqual(rows.map(row => [row.id, row.name]), [
    ['answer:0', 'original.txt'], ['answer:1', 'shared.txt'], ['answer:2', 'cited.txt'],
  ]);
  assert.ok(projection.fileSource(payload, 'answer:0').attachment);
  assert.ok(projection.fileSource(payload, 'answer:1').sharedLibraryReference);
  assert.ok(projection.fileSource(payload, 'answer:2').fileCitationReference);
});

test('file citations remain bound to the selected branch and message visibility', () => {
  const hidden = message('root');
  hidden.metadata.is_visually_hidden_from_conversation = true;
  const payload = { current_node: 'selected', mapping: {
    root: { parent: null, message: hidden },
    selected: { parent: 'root', message: message('selected') },
    alternate: { parent: 'root', message: message('alternate') },
  } };
  assert.deepEqual(projection.files(payload).files.map(row => row.messageId), ['selected']);
  assert.equal(projection.fileSource(payload, 'alternate:0'), null);
  delete payload.current_node;
  assert.equal(projection.files(payload), null);
  for (const fields of [{ channel: 'analysis' }, { recipient: 'python' }, { author: { role: 'tool' } }]) {
    assert.equal(projection.files({ messages: [{ ...message(), ...fields }] }).files.length, 0);
  }
});

test('reference/index limits and source row positions agree without claiming completeness', () => {
  const refs = Array.from({ length: 21 }, (_, i) => ref('file-' + i));
  const payload = { messages: [message('many', refs)] };
  assert.equal(projection.files(payload).files.length, 20);
  assert.equal(projection.files(payload).truncated, true);
  assert.equal(projection.fileSource(payload, 'many:19').fileCitationReference.id, 'file-19');
  assert.equal(projection.fileSource(payload, 'many:20'), null);
  assert.equal(projection.files({ messages: Array.from({ length: 6 }, (_, i) => message('m' + i, refs)) }).files.length, 100);
});

test('account or document changes revoke citation downloads before an HTTP request', async () => {
  for (const change of [f => f.setAccount('Bearer other-synthetic-account'),
    f => { f.root.__elonChatGptDocumentToken = 'doc_other_2'; }]) {
    const f = fixture(), rows = f.rows();
    change(f);
    await f.run(rows[0]);
    assert.equal(f.calls.length, 0);
    assert.equal(f.queued.length, 0);
  }
});

test('Android loads the citation module before its owners and reinjection preserves transport identity', () => {
  const assets = fs.readFileSync('android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt', 'utf8');
  const names = [...assets.matchAll(/"(chatgpt_web_[a-z0-9_]+\.js)"/g)].map(match => match[1]);
  assert.equal(new Set(names).size, names.length);
  const assembled = names.map(name => fs.readFileSync('android/app/src/main/assets/' + name, 'utf8')).join('\n');
  assert.doesNotThrow(() => new vm.Script(assembled), 'the production asset bundle must compile in catalog order');
  for (const owner of ['history_projection', 'file_download']) {
    assert.ok(names.indexOf('chatgpt_web_private_file_citation.js') < names.indexOf('chatgpt_web_private_' + owner + '.js'));
  }
  let retired = 0;
  const transport = {}, window = { location: { origin: 'https://chatgpt.com' },
    __elonChatGptPrivateTransport: transport,
    __elonChatGptPrivateFileDownload: { version: 14, dispose() { retired++; } } };
  const context = vm.createContext({ window, Map, Set, URL });
  for (let i = 0; i < 2; i++) {
    for (const name of ['file_citation', 'history_projection', 'file_download']) {
      vm.runInContext(fs.readFileSync('android/app/src/main/assets/chatgpt_web_private_' + name + '.js', 'utf8'), context);
    }
  }
  assert.equal(retired, 1);
  assert.equal(window.__elonChatGptPrivateTransport, transport);
  assert.equal(window.__elonChatGptPrivateHistoryProjection.create({}).files({ messages: [message()] }).files.length, 1);
  assert.equal(window.__elonChatGptPrivateFileDownload.version, download.version);
});
