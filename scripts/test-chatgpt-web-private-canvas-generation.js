'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const generation = require('../android/app/src/main/assets/chatgpt_web_private_canvas_generation.js');
const policy = require('../android/app/src/main/assets/chatgpt_web_private_canvas_document_policy.js');
const { fixture, ID } = require('./fixtures/chatgpt-web-private-canvas-documents.cjs');

function runtimeFixture() {
  let rendering = false, effects = [], unmounts = 0, mounted = 0, denied = false, account = 'synthetic-account';
  let leaf = 'original', invoked = 0, captured, document;
  const conversation = { id: 'conversation', serverId$: () => 'cid' }, rows = [conversation];
  const nodes = { original: { id: 'original', message: { author: { role: 'assistant' } } } };
  const shared = { canvasConversations: () => rows, useCanvasSendBlocked: () => denied,
    XM: () => nodes, HM: { getCurrentLeafId: () => leaf, getNode: (_, id) => nodes[id],
      getParentPromptNode: () => nodes.user } };
  const react = { createElement: (type, props, child) => ({ type, props: { ...props, children: child } }),
    useLayoutEffect: callback => { assert.equal(rendering, true); effects.push(callback); } };
  const runtime = { reactApi: () => react, reactDom: () => ({ flushSync: fn => { fn(); effects.splice(0).forEach(fn => fn()); } }),
    reactRoot: () => ({ createRoot: () => { mounted++; return {
      render: value => { rendering = true; const inner = value.type(value.props); inner.type(inner.props); rendering = false; },
      unmount: () => { unmounts++; }
    }; } }), intlInit() {}, intlProvider: props => props.children };
  const module = { i() {}, a(owner, doc) {
    assert.equal(rendering, true); assert.equal(owner, conversation); document = doc;
    return async value => { invoked++; captured = value; };
  } };
  const page = { document: { createElement: () => ({}), documentElement: { lang: 'zh-CN' } }, Event,
    setTimeout, clearTimeout, __elonChatGptPrivateTextTransactionsEnabled: true,
    __elonChatGptPrivateCanvasDocumentPolicy: policy,
    __elonChatGptPrivateModelContract: { create: () => ({ withRuntimeIdentity: () => ({ account }) }) },
    __elonChatGptPrivateRuntimeBindings: { state: () => ({ profile_id: 'web_20260912' }),
      load: async role => role === 'shared' ? shared : runtime } };
  const doc = { id: ID, content: 'A\u{1f600}BC', documentVersion: 4, documentType: 'document' };
  const input = { prompt: 'Rewrite synthetic selection', start: 1, end: 3 };
  const service = generation.create(page, { loadRuntime: async () => module, timeoutMs: 20 });
  let valid = true;
  const prepare = (override = {}) => service.prepare({ id: 'cid' }, doc, { ...input, ...override }, () => {
    if (!valid) throw Error('canvas_context_changed');
  });
  const reply = (patch = {}) => {
    nodes.user = { id: 'user', message: { author: { role: 'user' }, content: { content_type: 'text', parts: [input.prompt] },
      metadata: { canvas: { textdoc_id: ID, textdoc_type: 'document', version: 4, textdoc_content_length: doc.content.length,
        user_message_type: 'ask_chatgpt', selection_metadata: captured?.selectionMetadata, ...patch } } } };
    nodes.answer = { id: 'answer', message: { author: { role: 'assistant' }, status: 'finished_successfully' } };
    leaf = 'answer';
  };
  return { page, module, runtime, shared, rows, doc, input, nodes, prepare, reply,
    change: () => { valid = false; }, deny: () => { denied = true; }, swap: () => { account = 'other'; },
    counts: () => ({ invoked, mounted, unmounts }), captured: () => ({ value: captured, document }) };
}

test('real hook contract receives original UTF-16 selection, version and explicit edit intent once', async () => {
  const f = runtimeFixture(), ready = await f.prepare();
  assert.deepEqual(f.counts(), { invoked: 0, mounted: 0, unmounts: 0 });
  let dispatched = 0;
  await ready.invoke(() => dispatched++);
  assert.deepEqual(f.counts(), { invoked: 1, mounted: 1, unmounts: 1 });
  assert.equal(dispatched, 1); assert.equal(f.counts().invoked, 1);
  const { value, document } = f.captured();
  assert.deepEqual(value.sourceRange, { start: 1, end: 3 });
  assert.equal(value.action, 'edit'); assert.equal(value.userMessageType, 'ask_chatgpt');
  assert.equal(document.content, f.doc.content); assert.equal(document.versionInt, 4);
  assert.equal(ready.settled(), false);
  assert.throws(() => ready.invoke(() => dispatched++), /write_unconfirmed/);
  f.reply(); assert.equal(ready.settled(), true);
  f.swap(); assert.throws(ready.settled, /context_changed/);
});
test('whole-document edit omits selection metadata, never invents a range', async () => {
  const f = runtimeFixture(), ready = await f.prepare({ start: 3, end: 3 }); await ready.invoke(() => {});
  assert.equal(f.captured().value.sourceRange, undefined); assert.equal(f.captured().value.selectionMetadata, undefined);
});

test('accept-comment command derives immutable original text and UTF-16 anchor, not caller prompt', async () => {
  const f = runtimeFixture();
  f.doc.comments = [{ id: 'comment', content: 'x'.repeat(5000), start: 1, end: 3 }];
  const ready = await f.prepare({ operation: 'accept_comment', commentId: 'comment', prompt: 'Ignored', start: 0, end: 0 });
  ready.validate(); assert.equal(f.counts().invoked, 0);
  await ready.invoke(() => {});
  const value = f.captured().value;
  assert.equal(value.content, 'x'.repeat(5000)); assert.equal(value.userMessageType, 'accept_comment');
  assert.deepEqual(value.sourceRange, { start: 1, end: 3 });
  f.reply({ user_message_type: 'accept_comment' }); f.nodes.user.message.content.parts = [value.content];
  assert.equal(ready.settled(), true);
});

test('missing or invalid comment never yields an AI command', async () => {
  const f = runtimeFixture();
  await assert.rejects(f.prepare({ operation: 'accept_comment', commentId: 'missing' }), /comment_invalid/);
  f.doc.comments = [{ id: 'comment', content: 'Suggestion', start: 2, end: 3 }];
  await assert.rejects(f.prepare({ operation: 'accept_comment', commentId: 'comment' }), /selection_invalid/);
});
for (const input of [{ start: 2 }, { start: -1 }, { end: 999 }, { start: 4, end: 3 },
  { prompt: '' }, { prompt: 'x'.repeat(4001) }, { prompt: '\ud800' }]) {
  test('rejects invalid prompt or Unicode range ' + JSON.stringify(input).slice(0,55), async () => {
    const f = runtimeFixture(); await assert.rejects(f.prepare(input), /canvas_/); assert.equal(f.counts().invoked, 0);
  });
}
for (const fault of ['denied', 'profile', 'identity', 'duplicate', 'changed', 'uncommitted', 'pending']) {
  test('refuses before dispatch: ' + fault, async () => {
    const f = runtimeFixture();
    if (fault === 'denied') f.deny();
    if (fault === 'profile') f.page.__elonChatGptPrivateRuntimeBindings.state = () => ({ profile_id: 'unknown' });
    if (fault === 'identity') f.page.__elonChatGptPrivateModelContract = null;
    if (fault === 'duplicate') f.rows.push(f.rows[0]);
    if (fault === 'changed') f.change();
    if (fault === 'uncommitted') f.runtime.reactDom = () => ({ flushSync: fn => fn() });
    if (fault === 'pending') f.page.__elonChatGptPrivateTextRuntimeSubmit = { state: () => ({ pending: true }) };
    await assert.rejects(async () => { const ready = await f.prepare(); await ready.invoke(() => {}); }, /canvas_/);
    assert.equal(f.counts().invoked, 0);
  });
}
test('permission is captured at dispatch, not before asynchronous original readback', async () => {
  const f = runtimeFixture(), ready = await f.prepare(); f.deny();
  assert.throws(() => ready.invoke(() => assert.fail('must not dispatch')), /generation_blocked/);
  assert.equal(f.counts().invoked, 0);
});
for (const patch of [{ textdoc_id: 'other' }, { version: 9 }, { user_message_type: 'accept_comment' },
  { selection_metadata: undefined }, { textdoc_content_length: 999 }]) {
  test('another reply is not generation completion: ' + JSON.stringify(patch), async () => {
    const f = runtimeFixture(), ready = await f.prepare(); await ready.invoke(() => {}); f.reply(patch);
    assert.equal(ready.settled(), false);
  });
}

function integration() {
  const f = fixture(); let dispatched = 0, settled = false, rejecting = false;
  f.page.__elonChatGptPrivateCanvasGeneration = { create: () => ({ prepare: async () => ({
    invoke: async before => { before(); dispatched++; if (rejecting) throw Error('uncertain'); }, settled: () => settled
  }) }) };
  const request = value => ({ operation: 'generate', ticket: value.ticket, scope: value.scope, id: ID,
    prompt: 'Rewrite synthetic content', start: 0, end: 0 });
  return { ...f, request, dispatched: () => dispatched, finish: () => { settled = true; }, reject: () => { rejecting = true; } };
}
test('documents separate dispatch from completion; only GET verifies, never replay or erase original', async () => {
  const f = integration(), index = await f.list();
  assert.equal((await f.run(f.request(index))).code, 'canvas_confirmation_required');
  const sent = await f.run(f.request(index), true);
  assert.equal(sent.code, 'canvas_generation_dispatched'); assert.equal(sent.unconfirmedWrite, true);
  assert.equal(sent.documents[0].content, index.documents[0].content); assert.equal(f.api.generationPending(), true);
  assert.equal((await f.run(f.request(sent), true)).code, 'canvas_write_unconfirmed');
  let pending = await f.run({ ...f.request(sent), operation: 'verify' }, true);
  assert.equal(pending.code, 'canvas_generation_pending'); assert.equal(pending.unconfirmedWrite, true);
  f.rows[0].content = 'New generated content'; f.rows[0].version = 5; f.finish();
  assert.equal(f.api.generationPending(), false);
  const verified = await f.run({ ...f.request(pending), operation: 'verify' });
  assert.equal(verified.code, 'canvas_generated'); assert.equal(verified.unconfirmedWrite, false);
  assert.equal(f.dispatched(), 1); assert.ok(f.requests.every(value => value.init.method === 'GET'));
});
test('post-invocation rejection remains uncertain and cannot resend', async () => {
  const f = integration(), index = await f.list(); f.reject();
  assert.equal((await f.run(f.request(index), true)).code, 'canvas_generation_unconfirmed');
  const read = await f.list(true);
  assert.equal((await f.run(f.request(read), true)).code, 'canvas_write_unconfirmed'); assert.equal(f.dispatched(), 1);
});
test('a current original read prevents stale generation after preparation', async () => {
  const f = integration(), index = await f.list(); f.rows[0].version = 5;
  assert.equal((await f.run(f.request(index), true)).code, 'canvas_version_conflict'); assert.equal(f.dispatched(), 0);
});
test('completed no-edit reply is distinct from a new original version', async () => {
  const f = integration(), sent = await f.run(f.request(await f.list()), true); f.finish();
  const result = await f.run({ ...f.request(sent), operation: 'verify' });
  assert.equal(result.code, 'canvas_generation_no_change'); assert.equal(result.unconfirmedWrite, false);
});
