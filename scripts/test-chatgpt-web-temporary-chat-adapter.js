'use strict';

const assert = require('assert');
const pagePolicy = require('../android/app/src/main/assets/chatgpt_web_adapter_page_semantic_policy.js');
const adapter = require('../android/app/src/main/assets/chatgpt_web_adapter_temporary_chat.js');

global.window = {
  innerWidth: 400,
  innerHeight: 800,
  setTimeout(callback) { callback(); }
};

function node() {
  return {
    isConnected: true,
    getBoundingClientRect: () => ({ left: 20, top: 40, width: 80, height: 40, right: 100, bottom: 80 }),
    scrollIntoView() {}
  };
}

function execute(currentSelected, desiredSelected) {
  const events = [];
  const results = [];
  let snapshots = 0;
  const handled = adapter.setSelected({
    node: node(),
    control: { semantic: 'temporary_chat', selected: currentSelected },
    controlId: 'control_temporary_chat',
    desiredSelected,
    pageSemanticPolicy: pagePolicy,
    isVisible: () => true,
    isInViewport: () => true,
    emitEvent: (event) => events.push(event),
    result: (action, ok) => results.push({ action, ok }),
    emitSnapshot: () => { snapshots += 1; }
  });
  return { handled, events, results, snapshots };
}

const unchanged = execute(false, false);
assert.strictEqual(unchanged.handled, true);
assert.strictEqual(unchanged.events.length, 0);
assert.deepStrictEqual(unchanged.results, [{ action: 'set_ui_control_selected', ok: true }]);
assert.strictEqual(unchanged.snapshots, 1);

const changed = execute(false, true);
assert.strictEqual(changed.handled, true);
assert.strictEqual(changed.events.length, 1);
assert.strictEqual(changed.events[0].type, 'web_touch_request');
assert.strictEqual(changed.events[0].purpose, 'invoke_ui_control');
assert.strictEqual(changed.events[0].controlId, 'control_temporary_chat');
assert.deepStrictEqual(changed.results, [{ action: 'set_ui_control_selected', ok: true }]);
assert.strictEqual(changed.snapshots, 1);

assert.strictEqual(adapter.setSelected({ control: { semantic: 'close' } }), false);
assert.strictEqual(
  adapter.describe(pagePolicy, { signal: '关闭临时聊天' }).selected,
  true
);

console.log('CHATGPT_WEB_TEMPORARY_CHAT_ADAPTER=passed');
