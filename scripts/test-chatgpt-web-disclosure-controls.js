'use strict';

const assert = require('node:assert/strict');
global.window = {
  innerWidth: 1000,
  innerHeight: 1000,
  setTimeout(callback) { callback(); }
};
const disclosures = require('../android/app/src/main/assets/chatgpt_web_adapter_disclosure_controls.js');

function node(attributes, options = {}) {
  const state = { ...attributes };
  return {
    disabled: !!options.disabled,
    isConnected: options.isConnected !== false,
    getAttribute(name) { return state[name] || ''; },
    setAttribute(name, value) { state[name] = String(value); },
    getBoundingClientRect() {
      return options.rect || { left: 100, top: 100, right: 220, bottom: 140, width: 120, height: 40 };
    },
    scrollIntoView() {}
  };
}

function invoke(target, desired, options = {}) {
  let touch = null;
  let receipt = null;
  let snapshots = 0;
  disclosures.setExpanded(
    target,
    'control_menu_demo',
    desired,
    (event) => {
      touch = event;
      if (options.applyTouch !== false) {
        target.setAttribute('aria-expanded', desired ? 'true' : 'false');
      }
    },
    (action, ok, detail) => { receipt = { action, ok, detail }; },
    () => { snapshots += 1; }
  );
  return { touch, receipt, snapshots };
}

const collapsed = node({ 'aria-expanded': 'false' });
assert.deepEqual(disclosures.describe(collapsed), { expanded: false, expandable: true });
const opened = invoke(collapsed, true);
assert.deepEqual(opened.receipt, { action: 'set_ui_control_expanded', ok: true, detail: '' });
assert.equal(opened.touch.purpose, 'invoke_ui_control');
assert.equal(opened.touch.controlId, 'control_menu_demo');
assert.equal(opened.snapshots, 1);

const expanded = node({ 'aria-expanded': 'true' });
const unchanged = invoke(expanded, true);
assert.equal(unchanged.touch, null);
assert.equal(unchanged.receipt.ok, true);
assert.equal(unchanged.snapshots, 1);

const unsupported = invoke(node({}), true);
assert.equal(unsupported.touch, null);
assert.equal(unsupported.receipt.ok, false);

const disabled = invoke(node({ 'aria-expanded': 'false', 'aria-disabled': 'true' }), true);
assert.equal(disabled.touch, null);
assert.equal(disabled.receipt.ok, false);

const unchangedAfterTouch = invoke(node({ 'aria-expanded': 'false' }), true, { applyTouch: false });
assert.equal(unchangedAfterTouch.touch.purpose, 'invoke_ui_control');
assert.equal(unchangedAfterTouch.receipt.ok, false);
assert.match(unchangedAfterTouch.receipt.detail, /未达到请求的展开状态/);
assert.equal(unchangedAfterTouch.snapshots, 1);

process.stdout.write('CHATGPT_DISCLOSURE_CONTROLS=passed\n');
