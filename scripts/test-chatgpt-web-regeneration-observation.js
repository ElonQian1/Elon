'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, flush, id } = require('./fixtures/chatgpt-runtime-regeneration.js');

for (const [code, change] of Object.entries({
  stream_missing: () => {},
  owner_changed: f => { f.page.location.href += '#changed'; },
  conversation_mismatch: f => f.publish({ conversationId: id(9) }),
  message_invalid: f => f.publish({ id: 'not-an-id' }),
  old_variant: f => f.publish({ id: f.message.id }),
  text_pending: f => f.publish({ text: '' }),
  state_unknown: f => f.publish({ state: 'not-a-state' }),
  leaf_pending: f => f.publish({}, false),
  node_pending: f => { f.publish({}, false); f.tree.leaf = id(5); },
  parent_mismatch: f => { f.publish({}, false); f.tree.leaf = id(5);
    f.tree.nodes.set(id(5), { message: { author: { role: 'assistant' } },
      parent: { id: id(8), message: { author: { role: 'user' } } } }); },
  tree_unavailable: f => { f.publish({}, false); f.modules.shared.XM = () => { throw Error('secret'); }; }
})) {
  test('timeout reports only the closed observation reason: ' + code, async () => {
    const f = fixture(), result = f.api.regenerate(f.command);
    await flush(); change(f); f.runTimer(15000);
    assert.deepEqual(await result.completion, { status: 'unknown', code: 'timeout_' + code });
    assert.equal(f.api.state().pending, true, 'an unknown result still holds the single-writer fence');
    assert.equal(f.calls.length, 1);
    assert.equal((await f.api.regenerate(f.command).completion).code, 'busy');
  });
}

test('a final readonly check recognizes a late tree commit without another stream event', async () => {
  const f = fixture(), result = f.api.regenerate(f.command);
  await flush(); f.publish({ state: 'completed' }, false);
  for (let i = 0; i < 20; i++) f.runTimer(100);
  assert.equal(f.runTimer(100), false, 'the existing short retry budget is unchanged');
  f.tree.leaf = id(5);
  f.tree.nodes.set(id(5), { message: { author: { role: 'assistant' } }, parent: f.parent });
  f.modelButton.isConnected = false;
  f.runTimer(15000);
  assert.deepEqual(await result.completion, { status: 'accepted', code: 'regenerate_observed' });
  assert.equal(f.calls.length, 1);
  assert.equal(f.api.state().pending, false);
  assert.equal(f.listeners.size, 0);
  assert.equal(f.timers.size, 0);
});
