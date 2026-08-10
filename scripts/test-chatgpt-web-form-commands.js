'use strict';

const assert = require('node:assert/strict');
global.window = {
  innerWidth: 1000,
  innerHeight: 1000,
  setTimeout(callback) { callback(); }
};
const forms = require('../android/app/src/main/assets/chatgpt_web_adapter_form_controls.js');
const commands = require('../android/app/src/main/assets/chatgpt_web_adapter_form_commands.js');

function node(attributes, options = {}) {
  const events = [];
  return {
    tagName: options.tagName || 'DIV',
    type: options.type || '',
    checked: !!options.checked,
    disabled: false,
    isConnected: true,
    options: options.options || [],
    selectedIndex: Number.isInteger(options.selectedIndex) ? options.selectedIndex : -1,
    events,
    getAttribute(name) { return attributes[name] || ''; },
    getBoundingClientRect() { return { left: 100, top: 100, right: 200, bottom: 140, width: 100, height: 40 }; },
    scrollIntoView() {},
    focus() {},
    dispatchEvent(event) { events.push(event.type); return true; }
  };
}

const toggle = node({ role: 'switch', 'aria-checked': 'false', 'aria-label': 'Memory' });
let touch = null;
let receipt = null;
let snapshots = 0;
commands.setSelected(
  toggle,
  'control_toggle_demo',
  true,
  forms,
  (event) => { touch = event; },
  (action, ok, detail) => { receipt = { action, ok, detail }; },
  () => { snapshots += 1; }
);
assert.equal(receipt.action, 'set_ui_control_selected');
assert.equal(receipt.ok, true);
assert.equal(touch.purpose, 'invoke_ui_control');
assert.equal(touch.controlId, 'control_toggle_demo');
assert.equal(snapshots, 1);

const alreadySelected = node({ role: 'checkbox', 'aria-checked': 'true' });
touch = null;
commands.setSelected(alreadySelected, 'control_toggle_done', true, forms, (event) => {
  touch = event;
}, () => {}, () => { snapshots += 1; });
assert.equal(touch, null);

const select = node({ 'aria-label': 'Model' }, {
  tagName: 'SELECT',
  selectedIndex: 0,
  options: [
    { label: 'Fast', disabled: false },
    { label: 'Thinking', disabled: false }
  ]
});
commands.selectChoice(select, 1, forms, (action, ok) => {
  receipt = { action, ok };
}, () => { snapshots += 1; });
assert.equal(receipt.action, 'select_ui_control_choice');
assert.equal(receipt.ok, true);
assert.equal(select.selectedIndex, 1);
assert.deepEqual(select.events, ['input', 'change']);

const range = node({ 'aria-label': 'Thinking effort' }, {
  tagName: 'INPUT',
  type: 'range'
});
range.min = '0';
range.max = '10';
range.step = '2';
range.value = '4';
commands.setSliderValue(range, 8, forms, (action, ok) => {
  receipt = { action, ok };
}, () => { snapshots += 1; });
assert.equal(receipt.action, 'set_ui_control_slider');
assert.equal(receipt.ok, true);
assert.equal(range.value, '8');
assert.deepEqual(range.events, ['input', 'change']);

process.stdout.write('CHATGPT_FORM_COMMANDS=passed\n');
