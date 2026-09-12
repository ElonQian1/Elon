'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const actions = require('../android/app/src/main/assets/chatgpt_web_canvas_document_actions.js');
const { fixture, PATH, ID } = require('./fixtures/chatgpt-web-private-canvas-documents.cjs');

function setup() {
  const f = fixture(), events = [];
  f.page.__elonChatGptPrivateCanvasDocuments = { create: () => f.api };
  const api = actions.create(f.page);
  const call = (input, confirmed = false, overrides = {}) => new Promise(resolve => {
    const command = { value: JSON.stringify({ path: PATH, ...input }), selected: confirmed, requestId: 'mcp_test', ...overrides };
    assert.equal(api.handle('canvas_document', command, (action, ok, detail) => resolve({ action, ok, detail }),
      () => f.snapshot, value => events.push(value)), true);
  });
  return { ...f, events, api: actions.create(f.page), call };
}

test('canonical list emits complete typed display data without composer readiness', async () => {
  const f = setup();
  f.rows[0].content = '<script>inert</script>\n' + '正文\u{1f600}'.repeat(4000);
  assert.equal((await f.call({ operation: 'list', force: false })).detail, 'canvas_ready');
  const event = f.events[0];
  assert.deepEqual(Object.keys(event).sort(), ['documents', 'path', 'requestId', 'scope', 'ticket', 'type', 'unconfirmedWrite', 'version'].sort());
  assert.equal(event.documents[0].content, f.rows[0].content);
  assert.equal(event.type, 'canvas_documents');
  assert.equal(event.requestId, 'mcp_test');
  assert.match(event.scope, /^cd_[a-f0-9]{32}_[a-z0-9]+$/);
  assert.equal(f.imports(), 0);
});

test('canonical save is explicit, emits version readback and submits only once', async () => {
  const f = setup();
  await f.call({ operation: 'list', force: false });
  const { ticket, scope } = f.events[0];
  const input = { operation: 'save', ticket, scope, id: ID, content: '完整正文\u{1f600}', comments: [] };
  assert.equal((await f.call(input)).detail, 'canvas_confirmation_required');
  assert.equal(f.requests.filter(x => x.init.method === 'POST').length, 0);
  assert.equal((await f.call(input, true)).detail, 'canvas_saved');
  assert.equal(f.events.at(-1).documents[0].content, input.content);
  assert.equal(f.events.at(-1).documents[0].documentVersion, 5);
  assert.equal((await f.call(input, true)).detail, 'canvas_selection_expired');
  assert.equal(f.requests.filter(x => x.init.method === 'POST').length, 1);
});
test('canonical rename requires confirmation and emits the unchanged source with the new title', async () => {
  const f = setup(); await f.call({ operation: 'list', force: true });
  const index = f.events[0], input = { operation: 'rename', ticket: index.ticket, scope: index.scope, id: ID, title: 'New title' };
  assert.equal((await f.call(input, false)).detail, 'canvas_confirmation_required');
  assert.equal(f.requests.filter(value => value.init.method === 'POST').length, 0);
  assert.equal((await f.call(input, true)).detail, 'canvas_renamed');
  assert.equal(f.events.at(-1).documents[0].title, 'New title');
  assert.equal(f.events.at(-1).documents[0].content, index.documents[0].content);
  assert.equal((await f.call({ ...input, extra: 'not_allowed' }, true)).detail, 'canvas_request_invalid');
});

test('malformed commands and missing display channel cannot dispatch', async () => {
  const f = setup();
  for (const input of [null, [], { operation: 'list', force: 'true' }, { operation: 'list', force: false, headers: {} },
    { operation: 'save', ticket: 'expired', scope: 'expired', id: ID, content: '', comments: [] }]) {
    assert.equal((await f.call(input)).ok, false);
  }
  assert.equal((await f.call({ operation: 'list', force: false }, false, { requestId: 'foreign' })).ok, false);
  let result;
  f.api.handle('canvas_document', { value: JSON.stringify({ operation: 'list', path: PATH, force: false }), requestId: 'mcp_test' },
    (_, ok, detail) => { result = { ok, detail }; }, () => f.snapshot, null);
  assert.equal(result.ok, false);
  assert.equal(f.requests.length, 0);
  assert.equal(f.api.handle('other', {}, () => {}, () => f.snapshot, () => {}), false);
});

test('scope is stable across refresh but changes with identity or document', async () => {
  const f = setup();
  await f.call({ operation: 'list', force: false });
  const first = f.events.at(-1);
  await f.call({ operation: 'list', force: true });
  assert.equal(f.events.at(-1).scope, first.scope);
  const current = f.events.at(-1);
  const wrong = current.scope.replace(/^cd_./, 'cd_f') === current.scope ? current.scope.replace(/^cd_./, 'cd_0') : current.scope.replace(/^cd_./, 'cd_f');
  assert.equal((await f.call({ operation: 'save', ticket: current.ticket, scope: wrong, id: ID, content: 'edit', comments: [] }, true)).detail, 'canvas_selection_expired');
  f.headers.Authorization = 'Bearer synthetic-second';
  await f.call({ operation: 'list', force: true });
  const second = f.events.at(-1);
  assert.notEqual(second.scope, first.scope);
  f.page.document = {};
  f.page.__elonChatGptDocumentToken = 'doc_replaced';
  await f.call({ operation: 'list', force: true });
  assert.notEqual(f.events.at(-1).scope, second.scope);
  assert.equal(f.requests.filter(x => x.init.method === 'POST').length, 0);
});

test('unknown write remains guarded and verify never repeats POST', async () => {
  const f = setup();
  await f.call({ operation: 'list', force: false });
  const first = f.events.at(-1);
  f.setHook(async ({ init }) => { if (init.method === 'POST') throw Error('network'); });
  const edit = { operation: 'save', ticket: first.ticket, scope: first.scope, id: ID, content: 'edit', comments: [] };
  assert.equal((await f.call(edit, true)).detail, 'canvas_write_unconfirmed');
  assert.equal((await f.call(edit, true)).detail, 'canvas_write_unconfirmed');
  await f.call({ operation: 'list', force: true });
  const selection = () => ({ operation: 'verify', ticket: f.events.at(-1).ticket, scope: first.scope, id: ID });
  assert.equal((await f.call(selection())).detail, 'canvas_verification_pending');
  assert.equal(f.events.at(-1).unconfirmedWrite, true);
  assert.equal((await f.call(selection(), true)).detail, 'canvas_result_acknowledged');
  assert.equal(f.events.at(-1).unconfirmedWrite, false);
  assert.equal(f.requests.filter(x => x.init.method === 'POST').length, 1);
});

test('production menu and canonical assets route to native editor rather than shared snapshot', () => {
  const root = path.resolve(__dirname, '..');
  const read = name => fs.readFileSync(path.join(root, name), 'utf8');
  const dir = 'android/app/src/main/kotlin/com/elon/app/';
  assert.match(read(dir + 'WebChatProductionConversationActions.kt'), /ACTION_CANVAS -> canvas\.show\(conversation\)/);
  assert.match(read(dir + 'WebChatProductionConversationActions.kt'), /web-chat-conversation-action-canvas/);
  assert.match(read(dir + 'WebChatCanvasEditorView.kt'), /web-chat-canvas-editor-save/);
  assert.match(read(dir + 'WebChatCanvasEditorView.kt'), /web-chat-canvas-version-comparison/);
  assert.doesNotMatch(read(dir + 'WebChatCanvasEditorView.kt'), /loadUrl|evaluateJavascript|android\.webkit/);
  const assets = read(dir + 'chatgptweb/ChatGptWebAdapterAssets.kt');
  for (const name of ['document_policy', 'edit_context', 'document_sharing', 'documents']) assert.match(assets, new RegExp('chatgpt_web_private_canvas_' + name + '\\.js'));
  assert.match(assets, /chatgpt_web_canvas_document_actions\.js/);
  assert.match(read('android/app/src/main/assets/chatgpt_web_adapter.js'), /__elonChatGptCanvasDocumentActions\?\.handle\(action, command, respond, snapshot, emitEvent\)/);
});

test('history and sharing use the same canonical display channel and explicit mutation confirmation', async () => {
  const f = setup();
  f.page.__elonChatGptPrivateCanvasDocumentSharing = require('../android/app/src/main/assets/chatgpt_web_private_canvas_document_sharing.js');
  f.page.__elonChatGptPrivateCanvasContent = require('../android/app/src/main/assets/chatgpt_web_private_canvas_content.js');
  await f.call({ operation: 'list', force: false });
  const { ticket, scope } = f.events[0];
  f.setHook(({ url }) => url.endsWith('/share') ? { payload: { shared_textdoc: null } } :
    { payload: { previous_doc_states: [{ id: ID, title: 'Synthetic', content: 'Full source', version: 3, textdoc_type: 'document', comments: [] }] } });
  assert.equal((await f.call({ operation: 'history', ticket, scope, id: ID, beforeVersion: 4 })).ok, true);
  const page = f.events.at(-1).history;
  assert.equal(page.documentId, ID); assert.equal(page.versions[0].content, 'Full source');
  const request = { operation: 'restore', ticket, scope, id: ID, historyTicket: page.ticket, restoreVersion: 3 };
  assert.equal((await f.call(request)).detail, 'canvas_confirmation_required');
  assert.equal((await f.call({ ...request, historyTicket: 'bad' }, true)).detail, 'canvas_request_invalid');
  assert.equal((await f.call({ operation: 'share_lookup', ticket, scope, id: ID })).ok, true);
  assert.equal(f.events.at(-1).share.state, 'missing');
  for (const operation of ['share_create', 'share_ack'])
    assert.equal((await f.call({ operation, ticket, scope, id: ID })).detail, 'canvas_confirmation_required');
  assert.equal(f.requests.filter(x => x.init.method === 'POST').length, 0);
});

test('management entries are native editor icons and parent close cancels child generation', () => {
  const dir = path.resolve(__dirname, '../android/app/src/main/kotlin/com/elon/app');
  const read = name => fs.readFileSync(path.join(dir, name + '.kt'), 'utf8');
  const view = read('WebChatCanvasEditorView'), owner = read('WebChatCanvasDocumentsCoordinator');
  const management = read('WebChatCanvasManagementCoordinator');
  for (const id of ['history', 'share']) assert.ok(view.includes('web-chat-canvas-editor-' + id));
  assert.match(owner, /history = \{ management\?\.showHistory\(\) \}, share = \{ management\?\.showShare\(\) \}/);
  assert.match(owner, /fun cancel\(\) \{[^}]+management\?\.cancel\(\); management = null/s);
  for (const id of ['web-chat-canvas-history-restore-confirm', 'web-chat-canvas-original-share-confirm', 'web-chat-canvas-history-content'])
    assert.ok(management.includes(id));
  assert.match(management, /!draft\.changed && draft\.matches\(value\)/);
  assert.match(management, /if \(active\(run\)\) \{ state\("读取完成", false\); done\(value\) \}/);
  assert.doesNotMatch(management + view, /loadUrl|evaluateJavascript|android\.webkit|WebView/);
});
