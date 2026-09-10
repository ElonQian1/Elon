'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, flush, id } = require('./fixtures/chatgpt-runtime-regeneration');

function setup(options) {
  const f = fixture(options), listeners = new Set();
  f.modules.shared.conversationStore = {
    getState: () => ({}),
    subscribe(fn) { listeners.add(fn); return () => listeners.delete(fn); }
  };
  f.commit = (change = () => {}) => {
    const message = { id: id(5), author: { role: 'assistant' }, channel: 'final', recipient: 'all',
      status: 'in_progress', content: { content_type: 'text', parts: ['synthetic runtime answer'] } };
    const node = { message, parent: f.parent };
    change(message, node);
    f.tree.leaf = message.id;
    f.tree.nodes.set(message.id, node);
    for (const listener of listeners) listener();
  };
  f.storeListeners = listeners;
  return f;
}

for (const state of ['in_progress', 'finished_successfully', 'finished_partial_completion']) {
  test('official store confirms a new visible reply without a captured stream: ' + state, async () => {
    const f = setup(), result = f.api.regenerate(f.command);
    await flush();
    assert.equal(f.storeListeners.size, 1);
    f.modelButton.isConnected = false;
    f.commit(message => { message.status = state; });
    f.runTimer(15000);
    assert.deepEqual(await result.completion, { status: 'accepted', code: 'regenerate_observed' });
    assert.equal(f.calls.length, 1);
    assert.equal(f.api.state().pending, false);
    assert.equal(f.storeListeners.size, 0);
    assert.equal(f.listeners.size, 0);
    assert.equal(f.timers.size, 0);
  });
}

for (const [name, change] of Object.entries({
  analysis: m => { m.channel = 'analysis'; },
  tool: m => { m.recipient = 'synthetic-tool'; },
  other_role: m => { m.author.role = 'user'; },
  hidden: m => { m.metadata = { is_visually_hidden_from_conversation: true }; },
  hidden_reasoning: m => { m.metadata = { is_visually_hidden_reasoning_group: true }; },
  internal: m => { m.metadata = { debug_internal_only: true }; },
  non_text: m => { m.content.content_type = 'image'; },
  empty: m => { m.content.parts = ['   ']; },
  missing_parts: m => { m.content.parts = null; },
  unknown_part: m => { m.content.parts = [{ text: 'must not coerce' }]; },
  oversized: m => { m.content.parts = ['a'.repeat(40001)]; },
  unknown_status: m => { m.status = 'future_status'; },
  old_variant: m => { m.id = id(4); },
  wrong_parent: (m, node) => { node.parent = { id: id(99), message: { author: { role: 'user' } } }; }
})) {
  test('runtime state cannot confirm an unowned or invisible reply: ' + name, async () => {
    const f = setup(), result = f.api.regenerate(f.command);
    await flush(); f.commit(change); f.runTimer(15000);
    assert.equal((await result.completion).status, 'unknown');
    assert.equal(f.api.state().pending, true);
    assert.equal(f.calls.length, 1);
    f.page.__elonChatGptDocumentToken = 'doc_replaced_test';
    f.api.state();
    assert.equal(f.storeListeners.size, 0);
  });
}

for (const [name, change] of Object.entries({
  account: f => { f.identity.accountId = id(99); },
  user: f => { f.identity.userId = 'another-user'; },
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_another_test'; },
  route: f => { f.page.location.href = 'https://chatgpt.com/c/' + id(99); }
})) {
  test('store observation retains ownership: ' + name, async () => {
    const f = setup(), result = f.api.regenerate(f.command);
    await flush(); change(f); f.commit(); f.runTimer(15000);
    assert.equal((await result.completion).status, 'unknown');
    assert.equal(f.calls.length, 1);
  });
}

test('late owned store commit releases the fence without replaying an unknown request', async () => {
  const f = setup(), result = f.api.regenerate(f.command);
  await flush(); f.runTimer(15000);
  assert.equal((await result.completion).status, 'unknown');
  f.commit();
  assert.equal(f.api.state().pending, false);
  assert.equal(f.storeListeners.size, 0);
  assert.equal(f.calls.length, 1);
});

test('existing private stream remains sufficient and releases both subscriptions', async () => {
  const f = setup(), result = f.api.regenerate(f.command);
  await flush(); f.publish();
  assert.equal((await result.completion).status, 'accepted');
  assert.equal(f.storeListeners.size, 0);
  assert.equal(f.calls.length, 1);
});

test('a progress-only captured stream does not hide a committed runtime answer', async () => {
  const f = setup(), result = f.api.regenerate(f.command);
  await flush(); f.publish({ text: '' }, false); f.commit();
  assert.equal((await result.completion).status, 'accepted');
  assert.equal(f.calls.length, 1);
});

test('account changes inside subscription setup prevent the write', async () => {
  const f = setup(), subscribe = f.modules.shared.conversationStore.subscribe;
  f.modules.shared.conversationStore.subscribe = fn => {
    f.identity.accountId = id(99);
    return subscribe(fn);
  };
  assert.equal((await f.api.regenerate(f.command).completion).status, 'rejected');
  assert.equal(f.calls.length, 0);
  assert.equal(f.storeListeners.size, 0);
  assert.equal(f.listeners.size, 0);
});

test('verified profile exposes the exact store to the actual regeneration consumer', async () => {
  const { attach } = require('./fixtures/chatgpt-runtime-bindings');
  const profile = require('./fixtures/chatgpt-runtime-bindings-sep10b');
  const f = setup();
  attach(f.page, f.modules, { ...profile, expectedExports: {
    ...profile.expectedExports, shared: { ...profile.expectedExports.shared, ...profile.extraExports.shared }
  } });
  const runtime = require('../android/app/src/main/assets/chatgpt_web_private_regenerate_runtime');
  const contract = require('../android/app/src/main/assets/chatgpt_web_private_regenerate_contract');
  const api = runtime.create(f.page, { contract }), result = api.regenerate(f.command);
  for (let i = 0; i < 3; i++) await flush();
  assert.equal(f.storeListeners.size, 1);
  f.commit();
  assert.equal((await result.completion).status, 'accepted');
  assert.equal(f.calls.length, 1);
  assert.equal(f.storeListeners.size, 0);
});

for (const unavailable of ['missing', 'throws', 'different_store']) {
  test('unsupported store leaves the existing stream path usable: ' + unavailable, async () => {
    const f = setup();
    if (unavailable === 'missing') delete f.modules.shared.conversationStore;
    if (unavailable === 'throws') f.modules.shared.conversationStore.subscribe = () => { throw Error('synthetic'); };
    if (unavailable === 'different_store') f.modules.shared.XM = (cid, state) => state ? null : f.tree;
    const result = f.api.regenerate(f.command);
    await flush(); f.publish();
    assert.equal((await result.completion).status, 'accepted');
    assert.equal(f.storeListeners.size, 0);
    assert.equal(f.calls.length, 1);
  });
}
