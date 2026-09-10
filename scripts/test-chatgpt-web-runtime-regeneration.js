'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const runtime = require('../android/app/src/main/assets/chatgpt_web_private_regenerate_runtime.js');
const contract = require('../android/app/src/main/assets/chatgpt_web_private_regenerate_contract.js');
const { fixture, id, flush } = require('./fixtures/chatgpt-runtime-regeneration.js');

test('regeneration has a versioned official-runtime transaction', () => {
  assert.equal(runtime.version, 5);
  assert.equal(contract.version, 5);
  assert.equal(typeof runtime.create, 'function');
});

test('closed portal callback is called once without menu opening and waits for a new reply', async () => {
  const f = fixture(), result = f.api.regenerate(f.command);
  await flush();
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0].sourceEvent.type, 'click');
  assert.equal(f.calls[0].requestedModelId, 'synthetic-model');
  assert.deepEqual(Object.keys(f.calls[0]).sort(), ['requestedModelId', 'sourceEvent']);
  assert.equal(f.api.state().pending, true);
  assert.equal(f.prepared(), 1);
  assert.equal((await f.api.regenerate(f.command).completion).code, 'busy');
  assert.equal(f.prepared(), 1, 'duplicate commands must not clear the in-flight response');
  f.publish();
  assert.deepEqual(await result.completion, { status: 'accepted', code: 'regenerate_observed' });
  assert.equal(f.api.state().pending, false);
  assert.equal(f.listeners.size, 0);
  assert.equal(f.timers.size, 0);
});

for (const reason of ['user_stop', 'server_error']) {
  test('an officially retryable partial completion retains its original parent: ' + reason, async () => {
    const f = fixture();
    f.message.status = 'finished_partial_completion';
    if (reason === 'server_error') {
      f.message.metadata = { finish_details: { type: 'interrupted', reason } };
    }
    assert.equal(f.api.available(f.turn, f.command.getModelTrigger), true);
    const result = f.api.regenerate(f.command);
    await flush();
    assert.equal(f.calls.length, 1);
    assert.equal(f.tree.leaf, f.message.id, 'no synthetic user turn or replacement branch');
    f.publish();
    assert.equal((await result.completion).status, 'accepted');
    assert.equal(f.tree.nodes.get(id(5)).parent, f.parent);
    assert.equal(f.calls.length, 1);
  });
}

for (const state of ['streaming', 'completed']) {
  test('observes an owned retry while the model picker is disabled: ' + state, async () => {
    const f = fixture(), result = f.api.regenerate(f.command);
    await flush();
    f.modelButton.__reactFiber$test.return.memoizedProps.ariaDisabled = true;
    assert.equal(f.api.available(f.turn, f.command.getModelTrigger), false,
      'a disabled picker must still block starting a new mutation');
    f.publish({ state });
    assert.equal(f.api.state().pending, false,
      'temporary picker disablement must not block observing the already dispatched reply');
    assert.equal((await result.completion).status, 'accepted');
    assert.equal(f.calls.length, 1);
  });
}

for (const [reason, change] of Object.entries({
  identity: f => { f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => ({ authorization: 'Bearer different-synthetic-identity' }); },
  conversation: f => { f.modelMenu.conversation = { id: 'other', serverId$: () => id(1) }; },
  unknown_disabled_state: f => { f.modelButton.__reactFiber$test.return.memoizedProps.ariaDisabled = undefined; },
  parent: f => { f.parent.id = id(99); }
})) {
  test('disabled-picker observation still rejects changed ownership: ' + reason, async () => {
    const f = fixture(), result = f.api.regenerate(f.command);
    await flush();
    f.modelButton.__reactFiber$test.return.memoizedProps.ariaDisabled = true;
    change(f);
    f.publish();
    assert.equal(f.api.state().pending, true);
    f.runTimer(15000);
    assert.deepEqual(await result.completion, { status: 'unknown', code: 'timeout' });
    assert.equal(f.calls.length, 1, 'an unknown result must not invoke a fallback or replay');
  });
}

for (const [name, change] of Object.entries({
  disabled: f => { f.page.__elonChatGptPrivateTextTransactionsEnabled = false; },
  detached: f => { f.turn.isConnected = false; },
  old_fiber: f => { f.root.stateNode.current = {}; },
  denied: f => { f.menu.canRegenerateResponse = false; },
  upsell: f => { f.menu.retryOption.shouldShowUpsell = true; },
  image: f => { f.menu.hasImageGenMessage = true; },
  incomplete_message: f => { f.message.status = 'in_progress'; },
  unknown_message_status: f => { f.message.status = 'unknown_synthetic_status'; },
  unknown_module: f => { f.page.performance.getEntriesByName = () => []; },
  unknown_account: f => { f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => ({}); },
  another_conversation: f => { f.menu.conversation = {}; },
  ambiguous: f => { f.menuRoot.memoizedProps.children.push({ $$typeof: Symbol.for('react.transitional.element'), props: { ...f.menu } }); },
  missing_model_picker: f => { f.modelButton.isConnected = false; }
})) {
  test(name + ' is not a permission to invoke a runtime callback', async () => {
    const f = fixture(); change(f);
    const result = f.api.regenerate(f.command);
    assert.equal(result.handled, false);
    await flush(); assert.equal(f.calls.length, 0);
  });
}

test('committed alternate is recognized but abandoned work-in-progress is not used', async () => {
  const f = fixture(), committed = f.button.__reactFiber$test;
  f.button.__reactFiber$test = { return: { stateNode: { current: {} } }, alternate: committed };
  const result = f.api.regenerate(f.command);
  await flush(); assert.equal(f.calls.length, 1);
  f.publish(); assert.equal((await result.completion).status, 'accepted');
});

for (const [name, change] of Object.entries({
  rate_limit: f => { f.modelMenu.modelSwitcherDenialsBySlug['synthetic-model'] = true; },
  another_branch: f => { f.tree.leaf = id(9); },
  invalid_parent: f => { f.parent.message.author.role = 'assistant'; },
  variants_unknown: f => { f.tree.variants = undefined; },
  missing_runtime_export: f => { f.modules.shared.HM.getNode = undefined; }
})) {
  test(name + ' fails before any write', async () => {
    const f = fixture(); change(f);
    const result = f.api.regenerate(f.command);
    assert.equal((await result.completion).status, name === 'missing_runtime_export' ? 'unavailable' : 'rejected');
    assert.equal(f.calls.length, 0);
    assert.equal(f.api.state().pending, false);
  });
}

test('context changes during module loading cannot fall back into another conversation', async () => {
  const f = fixture();
  const result = f.api.regenerate(f.command);
  f.page.location.href = 'https://chatgpt.com/c/' + id(9);
  assert.equal((await result.completion).status, 'rejected');
  assert.equal(f.calls.length, 0);
});

test('pre-submit reentrancy cannot switch targets or bypass the final state check', async () => {
  const f = fixture();
  f.command.beforeSubmit = () => { f.menu.retryOption.value = 'changed-model'; };
  assert.equal((await f.api.regenerate(f.command).completion).status, 'rejected');
  assert.equal(f.calls.length, 0);
  assert.equal(f.prepared(), 0, 'a rejected pre-submit hook must retain the prior stream');
});

test('rate-limit changes in the pre-submit hook are rechecked before invocation', async () => {
  const f = fixture();
  f.command.beforeSubmit = () => { f.modelMenu.modelSwitcherDenialsBySlug['synthetic-model'] = true; };
  assert.equal((await f.api.regenerate(f.command).completion).status, 'rejected');
  assert.equal(f.calls.length, 0);
  assert.equal(f.prepared(), 0, 'a rejected rate-limit check must retain the prior stream');
});

test('stream reset notifications still cannot retarget the captured callback', async () => {
  const f = fixture(), stream = f.page.__elonChatGptPrivateStreamTransport;
  const prepare = stream.prepareSend;
  stream.prepareSend = () => { prepare(); f.menu.retryOption.value = 'changed-by-stream-listener'; };
  assert.equal((await f.api.regenerate(f.command).completion).status, 'rejected');
  assert.equal(f.calls.length, 0);
  assert.equal(f.api.state().pending, false);
});

test('partial completion still obeys official eligibility and model restrictions', async () => {
  const denied = fixture(); denied.message.status = 'finished_partial_completion';
  denied.menu.canRegenerateResponse = false;
  assert.equal(denied.api.regenerate(denied.command).handled, false);
  assert.equal(denied.calls.length, 0);
  const limited = fixture(); limited.message.status = 'finished_partial_completion';
  limited.modelMenu.modelSwitcherDenialsBySlug['synthetic-model'] = true;
  const result = limited.api.regenerate(limited.command);
  assert.equal(result.handled, true);
  assert.equal((await result.completion).status, 'rejected');
  assert.equal(limited.calls.length, 0);
  assert.equal(limited.prepared(), 0);
});

test('runtime import timeout only permits fallback while the original binding still matches', async () => {
  const f = fixture({ loadRuntime: () => new Promise(() => {}) });
  const result = f.api.regenerate(f.command); f.runTimer(1500);
  assert.equal((await result.completion).status, 'unavailable');
  assert.equal(f.calls.length, 0); assert.equal(f.api.state().pending, false);
});

test('unconfirmed tree visibility never starts an idle DOM polling loop', async () => {
  const f = fixture(), result = f.api.regenerate(f.command); await flush();
  assert.equal(f.runTimer(100), false, 'no polling before a real stream event');
  f.publish({}, false);
  for (let count = 0; count < 30; count++) f.runTimer(100);
  assert.equal(f.runTimer(100), false);
  f.runTimer(15000); assert.equal((await result.completion).code, 'timeout');
  f.page.__elonChatGptDocumentToken = 'doc_finished'; f.api.state();
  assert.equal(f.listeners.size, 0); assert.equal(f.timers.size, 0);
});

test('current stream plus matching official tree confirms the original parent, not old variants', async () => {
  const f = fixture(), c = contract.create(f.page);
  const binding = c.prepare(c.capture(f.turn, f.command.getModelTrigger), f.modules);
  for (const value of [{ id: f.message.id }, { id: id(4) }, { conversationId: id(9) }, { text: '' }]) {
    f.publish(value);
    assert.equal(c.observed(binding, f.modules, f.page.__elonChatGptPrivateStreamTransport.current()), false);
  }
  f.publish();
  const stream = f.page.__elonChatGptPrivateStreamTransport.current();
  assert.equal(c.observed(binding, f.modules, stream), true);
  f.tree.nodes.get(id(5)).parent = { id: id(8), message: { author: { role: 'user' } } };
  assert.equal(c.observed(binding, f.modules, stream), false);
});

test('a frame that arrives before the official tree commit is confirmed with bounded memory-only retries', async () => {
  const f = fixture(), result = f.api.regenerate(f.command); await flush();
  f.publish({}, false);
  assert.equal(f.api.state().pending, true);
  assert.equal(f.runTimer(100), true);
  f.tree.leaf = id(5); f.tree.nodes.set(id(5), { message: { author: { role: 'assistant' } }, parent: f.parent });
  f.runTimer(100);
  assert.equal((await result.completion).status, 'accepted');
  assert.equal(f.timers.size, 0);
});

test('timeout retains ownership; a late actual reply releases it without a second write', async () => {
  const f = fixture(), result = f.api.regenerate(f.command); await flush();
  assert.equal(f.runTimer(15000), true);
  assert.equal((await result.completion).code, 'timeout');
  assert.equal((await f.api.regenerate(f.command).completion).code, 'busy');
  assert.equal(f.calls.length, 1);
  f.publish(); assert.equal(f.api.state().pending, false);
});

test('the official missing-node exception still permits bounded stream-before-tree confirmation', async () => {
  const f = fixture(), result = f.api.regenerate(f.command); await flush();
  const getParent = f.modules.shared.HM.getParentPromptNode;
  f.modules.shared.HM.getParentPromptNode = (tree, key) => {
    if (!tree.nodes.has(key)) throw Error('synthetic missing node');
    return getParent(tree, key);
  };
  f.publish({}, false);
  assert.equal(f.runTimer(100), true, 'missing nodes must not discard the only response event');
  f.tree.leaf = id(5);
  f.tree.nodes.set(id(5), { message: { author: { role: 'assistant' } }, parent: f.parent });
  f.runTimer(100);
  assert.equal((await result.completion).status, 'accepted');
  assert.equal(f.calls.length, 1);
  assert.equal(f.timers.size, 0);
  assert.equal(f.listeners.size, 0);
});

test('callback exceptions are indeterminate and do not enable another route', async () => {
  const f = fixture({ invoke: () => { throw Error('synthetic'); } });
  assert.equal((await f.api.regenerate(f.command).completion).code, 'invocation_failed');
  assert.equal(f.api.state().pending, true);
  assert.equal((await f.api.regenerate(f.command).completion).code, 'busy');
  f.publish(); assert.equal(f.api.state().pending, false);
});

test('document replacement cleans up the old observer without implying success', async () => {
  const f = fixture(), result = f.api.regenerate(f.command); await flush();
  f.page.__elonChatGptDocumentToken = 'doc_new_document';
  assert.equal(f.api.state().pending, false);
  assert.equal((await result.completion).code, 'document_changed');
  assert.equal(f.listeners.size, 0); assert.equal(f.timers.size, 0);
});

test('text runtime and private relay ownership both block regeneration', async () => {
  for (const key of ['__elonChatGptPrivateTextRuntimeSubmit', '__elonChatGptPrivateTextTransactionRelay']) {
    const f = fixture(); f.page[key] = { state: () => ({ pending: true, active: true }) };
    assert.equal((await f.api.regenerate(f.command).completion).code, 'busy');
    assert.equal(f.calls.length, 0); assert.equal(f.prepared(), 0);
  }
});
