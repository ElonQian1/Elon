'use strict';

const assert = require('assert');
const revealPolicy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_project_choice_reveal.js'
);
const controlLabels = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_control_labels.js'
);

assert.equal(revealPolicy.normalizedLabel('  Project\u00a0Alpha  '), 'project alpha');
assert.equal(controlLabels.defaultLabel('save_to_project'), '保存到项目');
assert.equal(controlLabels.defaultLabel('unknown'), '操作');
assert.deepEqual(revealPolicy.scrollPositions({ scrollHeight: 300, clientHeight: 100 }), [0, 80, 160, 200]);
assert.deepEqual(revealPolicy.scrollPositions({ scrollHeight: 100, clientHeight: 100 }), []);

function node(label, parentElement) {
  return {
    label,
    role: 'menuitem',
    parentElement,
    scrollIntoViewCalls: 0,
    scrollIntoView() { this.scrollIntoViewCalls += 1; }
  };
}

const body = { nodeType: 1, tagName: 'BODY', parentElement: null };
let visibleChoices = [];
const scroller = {
  nodeType: 1,
  tagName: 'DIV',
  parentElement: body,
  scrollHeight: 300,
  clientHeight: 100,
  scrollTop: 0,
  isConnected: true,
  dispatchEvent() {},
  scrollTo(_x, y) {
    this.scrollTop = y;
    visibleChoices = y >= 80 ? [target] : [first];
  }
};
const first = node('Project Alpha', scroller);
const target = node('Project Omega', scroller);
visibleChoices = [first];
const overlay = {
  querySelectorAll() { return [scroller]; }
};
let changed = 0;
let outcome = null;
const adapter = revealPolicy.create({
  actionableNodes: () => visibleChoices,
  visibleOverlayRoots: () => [overlay],
  isVisible: () => true,
  labelOf: (value) => value.label,
  roleOf: (value) => value.role,
  setTimeout: (callback) => callback(),
  createScrollEvent: () => ({ type: 'scroll' })
});

adapter.reveal('Project Omega', () => { changed += 1; }, (ok, detail) => {
  outcome = { ok, detail };
});

assert.deepEqual(outcome, { ok: true, detail: 'project_choice_revealed' });
assert.equal(target.scrollIntoViewCalls, 1);
assert.equal(changed, 1);

visibleChoices = [first];
scroller.scrollTop = 0;
outcome = null;
adapter.reveal('Missing Project', () => { changed += 1; }, (ok, detail) => {
  outcome = { ok, detail };
});
assert.deepEqual(outcome, { ok: false, detail: 'project_choice_not_rendered' });
assert.equal(scroller.scrollTop, 0, 'an exhausted read-only scan restores the original position');

process.stdout.write('chatgpt project choice reveal tests passed\n');
