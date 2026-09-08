'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const modelFixture = require('./fixtures/chatgpt-model-state').fixture;
const attachmentFixture = require('./fixtures/chatgpt-attachment-composer');
const regeneration = require('./fixtures/chatgpt-runtime-regeneration');
const regenerate = require('../android/app/src/main/assets/chatgpt_web_private_regenerate_contract');

// React can reuse a child while its return pointer still names the old parent.
function replaceOwner(old) {
  const parent = old.return, current = { ...old, alternate: old };
  old.alternate = current;
  assert.equal(parent.child, old);
  parent.child = current;
  return current;
}

function deepen(host) {
  const parent = host.return;
  let child = host;
  for (let i = 0; i < 120; i++) {
    const next = { child }; child.return = next; child = next;
  }
  child.return = parent; parent.child = child;
}

test('model capture rejects an orphan even if its return chain reaches root.current', () => {
  const f = modelFixture(); f.top.child = null;
  assert.equal(f.contract.capture(f.getTrigger), null);
  assert.deepEqual(f.writes, []);
});

test('model selection uses the committed owner after a reused-child update', async () => {
  const f = modelFixture(); replaceOwner(f.ancestor);
  f.ancestor.memoizedProps = { ...f.props, ariaDisabled: true };
  f.request(); await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.fallback, 0); f.select(f.choice('High').id);
  assert.equal(f.model, 'fixture-thinking'); assert.equal(f.effort, 'high');
  assert.equal(f.results.at(-1)[1], true);
});

test('a deep committed model owner is not mistaken for a missing capability', () => {
  const f = modelFixture(); deepen(f.host);
  assert.equal(f.contract.capture(f.getTrigger)?.conversation, f.conversation);
});

test('regeneration rejects an orphan menu without preparing a request', () => {
  const f = regeneration.fixture(); f.menuRoot.child = null;
  assert.equal(regenerate.create(f.page).capture(f.turn, f.command.getModelTrigger), null);
  assert.deepEqual(f.calls, []); assert.equal(f.prepared(), 0);
});

test('regeneration resolves a reused menu child to the committed retry option', () => {
  const f = regeneration.fixture(); replaceOwner(f.menuRoot);
  f.menuRoot.memoizedProps = {};
  const binding = regenerate.create(f.page).capture(f.turn, f.command.getModelTrigger);
  assert.equal(binding?.message, f.message); assert.equal(binding?.menu, f.menu);
});

test('regeneration accepts a deep committed menu without traversing the whole page', () => {
  const f = regeneration.fixture(); deepen(f.button.__reactFiber$test);
  assert.equal(regenerate.create(f.page).capture(f.turn, f.command.getModelTrigger)?.message, f.message);
});

test('attachment store must belong to the current tree before bytes can be uploaded', () => {
  const f = attachmentFixture(); f.top.child = null;
  assert.equal(f.composer.available(), false);
  assert.throws(() => f.composer.capture(), /composer_context_unavailable/);
  assert.deepEqual(f.store.files$(), []);
});

test('attachment store and model policy come from the same committed ancestor path', () => {
  const f = attachmentFixture();
  const props = { conversation: {}, onCreateNewCompletion() {}, currentModelId: 'current-model',
    entrySurface: 'chat_composer', isLibraryEnabled: true, value: f.store };
  const owner = { memoizedProps: props, return: f.top, child: f.fiber };
  f.top.child = owner; f.fiber.return = owner;
  f.fiber.memoizedProps = {}; f.fiber.dependencies = null;
  replaceOwner(owner);
  owner.memoizedProps = { ...props, value: { ...f.store }, currentModelId: 'old-model', isLibraryEnabled: false };
  const binding = f.composer.capture();
  assert.equal(binding.store, f.store); assert.equal(binding.modelSlug, 'current-model');
  assert.equal(binding.libraryEnabled, true); assert.equal(f.composer.current(binding), true);
  f.top.child = null;
  assert.equal(f.composer.current(binding), false);
});

test('attachment state survives a deep committed input tree but not a cycle', () => {
  const f = attachmentFixture(); deepen(f.fiber);
  assert.equal(f.composer.capture().store, f.store);
  f.fiber.return.child = f.fiber.return;
  assert.equal(f.composer.available(), false);
});
