'use strict';

const assert = require('assert');
const pagePolicy = require('../android/app/src/main/assets/chatgpt_web_adapter_page_semantic_policy.js');
const adapter = require('../android/app/src/main/assets/chatgpt_web_adapter_temporary_chat.js');

global.window = {
  innerWidth: 400,
  innerHeight: 800,
  setTimeout(callback) { callback(); }
};

function node(hasDomClick = true) {
  let clicks = 0;
  const target = {
    isConnected: true,
    getBoundingClientRect: () => ({ left: 20, top: 40, width: 80, height: 40, right: 100, bottom: 80 }),
    scrollIntoView() {},
    clickCount() { return clicks; }
  };
  if (hasDomClick) target.click = () => { clicks += 1; };
  return target;
}

function execute(currentSelected, desiredSelected, hasDomClick = true) {
  const events = [];
  const results = [];
  let snapshots = 0;
  const target = node(hasDomClick);
  const handled = adapter.setSelected({
    node: target,
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
  return { handled, events, results, snapshots, clicks: target.clickCount() };
}

const unchanged = execute(false, false);
assert.strictEqual(unchanged.handled, true);
assert.strictEqual(unchanged.events.length, 0);
assert.deepStrictEqual(unchanged.results, [{ action: 'set_ui_control_selected', ok: true }]);
assert.strictEqual(unchanged.snapshots, 1);

const changed = execute(false, true);
assert.strictEqual(changed.handled, true);
assert.strictEqual(changed.clicks, 1);
assert.strictEqual(changed.events.length, 0);
assert.deepStrictEqual(changed.results, [{ action: 'set_ui_control_selected', ok: true }]);
assert.strictEqual(changed.snapshots, 1);

const nativeTouchFallback = execute(false, true, false);
assert.strictEqual(nativeTouchFallback.clicks, 0);
assert.strictEqual(nativeTouchFallback.events.length, 1);
assert.strictEqual(nativeTouchFallback.events[0].type, 'web_touch_request');
assert.strictEqual(nativeTouchFallback.events[0].purpose, 'invoke_ui_control');
assert.strictEqual(nativeTouchFallback.events[0].controlId, 'control_temporary_chat');

assert.strictEqual(adapter.setSelected({ control: { semantic: 'close' } }), false);
assert.strictEqual(
  adapter.describe(pagePolicy, { signal: '关闭临时聊天' }).selected,
  true
);

console.log('CHATGPT_WEB_TEMPORARY_CHAT_ADAPTER=passed');
