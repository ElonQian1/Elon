'use strict';

const assert = require('node:assert/strict');
const policy = require('../android/app/src/main/assets/chatgpt_web_adapter_dictation_session_policy.js');

function button({ left, top, right, bottom, label = '', testId = '' }) {
  return {
    id: '',
    textContent: '',
    getAttribute(name) {
      return ({ 'aria-label': label, 'data-testid': testId })[name] || null;
    },
    getBoundingClientRect() {
      return { left, top, right, bottom };
    }
  };
}

const oldCancel = button({ left: 280, top: 710, right: 330, bottom: 760, label: '取消听写' });
const oldSubmit = button({ left: 335, top: 710, right: 385, bottom: 760, label: '提交听写' });
assert.deepEqual(policy.collectNodes({ querySelectorAll(selector) {
  assert.equal(selector, 'button, [role="button"], [tabindex], svg');
  return [oldCancel, { closest: () => oldCancel }, oldSubmit];
} }), [oldCancel, oldSubmit], 'nested SVG controls retain one owner in document order');
const explicit = {
  nodes: [oldCancel, oldSubmit],
  isActionable: () => true,
  composerPresent: true,
  viewportWidth: 400,
  viewportHeight: 800
};
assert.equal(policy.find('cancel', explicit), oldCancel);
assert.equal(policy.find('submit', explicit), oldSubmit);
assert.equal(policy.active(explicit), true);

const plus = button({ left: 20, top: 710, right: 75, bottom: 765 });
const iconCancel = button({ left: 285, top: 710, right: 335, bottom: 760 });
const iconSubmit = button({ left: 340, top: 710, right: 390, bottom: 760 });
const iconSession = {
  nodes: [plus, iconCancel, iconSubmit],
  isActionable: () => true,
  composerPresent: false,
  captureActive: true,
  viewportWidth: 400,
  viewportHeight: 800
};
assert.equal(policy.find('cancel', iconSession), iconCancel);
assert.equal(policy.find('submit', iconSession), iconSubmit);
assert.equal(policy.active(iconSession), true);

for (const captureActive of [undefined, null, false, 'true', 1]) {
  const idle = { ...iconSession, captureActive };
  assert.equal(policy.active(idle), false, 'hidden composer plus round buttons is not a recording');
  assert.equal(policy.find('cancel', idle), null);
  assert.equal(policy.find('submit', idle), null);
}
assert.equal(policy.active({ ...explicit, captureActive: false }), true,
  'explicit official dictation controls still prove the official surface');

const webViewHitTestFallback = {
  ...iconSession,
  isActionable: () => false,
  isVisible: () => true
};
assert.equal(policy.find('cancel', webViewHitTestFallback), iconCancel);
assert.equal(policy.find('submit', webViewHitTestFallback), iconSubmit);
assert.equal(policy.active(webViewHitTestFallback), true);

assert.equal(policy.active({ ...iconSession, composerPresent: true }), false);
assert.equal(policy.active({ ...iconSession, nodes: [iconCancel, iconSubmit] }), false);
assert.equal(policy.active({ ...iconSession, nodes: [plus, iconSubmit] }), false);
assert.equal(policy.find('cancel', {
  ...iconSession,
  isActionable: (node) => node !== iconCancel
}), null);

console.log('ChatGPT dictation session policy tests passed.');
