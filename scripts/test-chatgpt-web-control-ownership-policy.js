'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const policy = require(
  '../android/app/src/main/assets/chatgpt_web_adapter_control_ownership_policy.js'
);

const layoutSource = fs.readFileSync(path.join(
  __dirname,
  '../android/app/src/main/assets/chatgpt_web_adapter_layout.js'
), 'utf8');
assert.match(
  layoutSource,
  /overlayOwnership\.rememberContextTrigger\(/,
  'layout records the context-bound control that requested a new overlay'
);
assert.match(
  layoutSource,
  /overlayOwnership\.resolveOverlayContext\(/,
  'layout resolves top-level overlay ownership before exporting controls'
);
assert.match(
  layoutSource,
  /addRegionControls\(\s*controls, overlay, 'overlay', used, null, overlayOwnership/,
  'layout exports inherited context on overlay controls'
);
assert.match(
  layoutSource,
  /used\.nodes = new WeakSet\(\)/,
  'layout tracks DOM identity across nested visible overlay roots'
);
assert.match(
  layoutSource,
  /used\.nodes\.has\(node\)/,
  'layout exports a nested overlay control only once'
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

let now = 1000;
const tracker = policy.createOverlayOwnershipTracker(() => now, 500);
const source = { isConnected: true };
const existingOverlay = { isConnected: true };
const openedOverlay = { isConnected: true };
const trigger = {
  region: 'message',
  semantic: 'more',
  contextId: 'conversation-turn-2'
};
assert.equal(
  tracker.rememberMessageTrigger(trigger, source, [existingOverlay], '/c/example'),
  true,
  'a message overflow trigger starts a bounded ownership claim'
);
assert.equal(
  tracker.resolveOverlayContext(existingOverlay, '/c/example'),
  '',
  'a menu that existed before the trigger is never claimed by the message'
);

assert.equal(
  tracker.resolveOverlayContext(openedOverlay, '/c/example'),
  'conversation-turn-2',
  'the newly opened menu inherits the triggering message context'
);
assert.equal(
  tracker.resolveOverlayContext(openedOverlay, '/c/example'),
  'conversation-turn-2',
  'the active menu keeps its context across repeated snapshots'
);
assert.equal(
  tracker.resolveOverlayContext({ isConnected: true }, '/c/example'),
  '',
  'probing another visible overlay does not claim the active menu context'
);
assert.equal(
  tracker.resolveOverlayContext(openedOverlay, '/c/example'),
  'conversation-turn-2',
  'probing another overlay does not clear the active menu ownership'
);
tracker.observeNoOverlay('/c/example');
assert.equal(
  tracker.resolveOverlayContext(openedOverlay, '/c/example'),
  '',
  'closing the menu clears active ownership'
);

const conversationTrigger = {
  region: 'overlay',
  semantic: 'conversation_options',
  contextId: 'conversation_123'
};
assert.equal(
  tracker.rememberContextTrigger(conversationTrigger, source, [], '/'),
  true,
  'a conversation overflow trigger starts a bounded ownership claim'
);
assert.equal(
  tracker.resolveOverlayContext({ isConnected: true }, '/'),
  'conversation_123',
  'the conversation action menu inherits the selected conversation context'
);
tracker.observeNoOverlay('/');

const nestedMenu = { isConnected: true };
const reusedOverlay = {
  isConnected: true,
  menu: '',
  contains(candidate) { return candidate === nestedMenu; }
};
tracker.rememberContextTrigger(
  conversationTrigger,
  source,
  [reusedOverlay],
  '/',
  (overlay) => overlay.menu
);
reusedOverlay.menu = 'rename|archive|delete';
assert.equal(
  tracker.resolveOverlayContext(reusedOverlay, '/', reusedOverlay.menu),
  'conversation_123',
  'management actions mounted into an existing overlay inherit the triggering conversation context'
);
assert.equal(
  tracker.resolveOverlayContext(nestedMenu, '/', reusedOverlay.menu),
  'conversation_123',
  'the nested structural menu root inherits ownership from its changed outer overlay'
);
assert.equal(
  tracker.resolveOverlayContext(reusedOverlay, '/', reusedOverlay.menu),
  'conversation_123',
  'probing the outer wrapper does not displace the more specific nested menu root'
);
assert.equal(
  tracker.resolveOverlayContext({ isConnected: true }, '/', reusedOverlay.menu),
  '',
  'an unrelated overlay still cannot steal nested menu ownership'
);
assert.equal(
  tracker.resolveOverlayContext(nestedMenu, '/', reusedOverlay.menu),
  'conversation_123',
  'the nested menu keeps ownership after unrelated overlay probes'
);
tracker.observeNoOverlay('/');

tracker.rememberMessageTrigger(trigger, source, [], '/c/example');
now += 501;
assert.equal(
  tracker.resolveOverlayContext({ isConnected: true }, '/c/example'),
  '',
  'a delayed unrelated menu cannot inherit an expired claim'
);

now += 1;
tracker.rememberMessageTrigger(trigger, source, [], '/c/example');
assert.equal(
  tracker.resolveOverlayContext({ isConnected: true }, '/settings'),
  '',
  'navigation clears pending message ownership'
);

const disconnectedSource = { isConnected: false };
tracker.rememberMessageTrigger(trigger, disconnectedSource, [], '/c/example');
assert.equal(
  tracker.resolveOverlayContext({ isConnected: true }, '/c/example'),
  '',
  'a removed trigger cannot own a later menu'
);

process.stdout.write('CHATGPT_WEB_CONTROL_OWNERSHIP_POLICY=passed\n');
