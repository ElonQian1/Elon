'use strict';

const assert = require('node:assert/strict');
const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_control_ownership_policy.js'
);

function control(role = 'textbox') {
  const descendants = new Set();
  return {
    role,
    descendants,
    contains(candidate) { return descendants.has(candidate); }
  };
}

const describe = (node) => ({ role: node.role });
const composer = control();

const hiddenComposer = Object.assign(control(), { visible: false });
const visibleComposer = Object.assign(control(), { visible: true });
const composerRoot = {
  querySelectorAll(selector) {
    return selector === '[data-testid="prompt-textarea"]'
      ? [hiddenComposer, visibleComposer]
      : [];
  }
};
assert.equal(
  policy.findVisibleComposer(composerRoot, (node) => node.visible),
  visibleComposer,
  'hidden retained composers do not own the active composer region'
);

const fallbackComposer = Object.assign(control(), { visible: true });
assert.equal(
  policy.findVisibleComposer({
    querySelectorAll(selector) {
      return selector === 'form [contenteditable="true"]' ? [fallbackComposer] : [];
    }
  }, (node) => node.visible),
  fallbackComposer,
  'visible fallback selectors remain supported'
);

assert.equal(
  policy.findVisibleComposer(composerRoot, () => false),
  null,
  'no hidden composer is selected when every candidate is invisible'
);

assert.equal(
  policy.isPrimaryComposerTextControl(composer, 'composer', composer, describe),
  true,
  'the primary composer is owned by the native composer'
);

const wrapper = control();
wrapper.descendants.add(composer);
assert.equal(
  policy.isPrimaryComposerTextControl(wrapper, 'composer', composer, describe),
  true,
  'a textbox wrapper containing the composer is not exposed twice'
);

const child = control();
composer.descendants.add(child);
assert.equal(
  policy.isPrimaryComposerTextControl(child, 'composer', composer, describe),
  true,
  'a textbox descendant of the composer is not exposed twice'
);

const unrelated = control();
assert.equal(
  policy.isPrimaryComposerTextControl(unrelated, 'composer', composer, describe),
  false,
  'an unrelated composer-region form control remains discoverable'
);

const relatedButton = control('button');
composer.descendants.add(relatedButton);
assert.equal(
  policy.isPrimaryComposerTextControl(relatedButton, 'composer', composer, describe),
  false,
  'non-text controls inside the composer remain discoverable'
);

assert.equal(
  policy.isPrimaryComposerTextControl(composer, 'content', composer, describe),
  false,
  'ownership only applies to the composer region'
);

process.stdout.write('CHATGPT_WEB_CONTROL_OWNERSHIP_POLICY=passed\n');
