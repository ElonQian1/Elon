'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const setup = require('./fixtures/chatgpt-library-append');
const bindings = require('../android/app/src/main/assets/chatgpt_web_private_runtime_bindings');

for (const [file, alias] of [
  ['8b34dbc2-kjj15hg4y6iyx13p.js', 'fh'], ['8b34dbc2-nhot65scqrg20d6p.js', 'Oh'],
  ['8b34dbc2-mx35vjavisrk7hwp.js', 'kh'], ['8b34dbc2-cj4kfo18e1ldvw16.js', 'kh'],
  ['8b34dbc2-l0q54hwyus1gmzd1.js', 'Ph'],
]) {
  test('attachment validator export matches the observed build ' + file, async () => {
    const url = 'https://chatgpt.com/cdn/assets/' + file, store = {};
    const page = { location: { origin: 'https://chatgpt.com' }, document: { querySelector: () => null },
      __elonChatGptDocumentToken: 'doc_test', setTimeout, clearTimeout,
      performance: { getEntriesByName: input => input === url ? [{}] : [] } };
    const api = bindings.create(page, { loadRuntime: async input => {
      assert.equal(input, url); return { fh: 'wrong-build-export', [alias]: store };
    } });
    assert.equal((await api.load('composer')).fh, store);
  });
}

test('current official limits and selected store are passed to the existing validator', async () => {
  const f = setup(); await f.attach(f.source(1));
  f.props.maxLibraryAttachmentCount = 7; f.props.maxTotalLibraryAttachmentCount = 8;
  await f.attach(f.source(2));
  assert.ok(f.validations.length > 0);
  for (const call of f.validations) {
    assert.equal(call[0], f.store); assert.equal(call[1], 45); assert.equal(call[2], undefined);
    assert.equal(call[3], 7); assert.equal(call[4], 8); assert.equal(call[5].length, 1);
  }
});

for (const fault of ['missing_owner', 'uncommitted', 'wrong_model', 'temporary', 'editor', 'invalid_limit', 'changed_runtime']) {
  test('no append through an unconfirmed official policy: ' + fault, async () => {
    const f = setup(); await f.attach(f.source(1));
    if (fault === 'missing_owner') f.fiber.type.name = 'Other';
    if (fault === 'uncommitted') f.top.stateNode.current = {};
    if (fault === 'wrong_model') f.props.currentModelId = 'other';
    if (fault === 'temporary') f.props.isTemporaryChat = true;
    if (fault === 'editor') f.props.gizmoEditorMode = true;
    if (fault === 'invalid_limit') f.props.maxLibraryAttachmentCount = NaN;
    if (fault === 'changed_runtime') f.root.__elonChatGptPrivateRuntimeBindings.peek = () => null;
    const before = f.store.files$();
    assert.equal((await f.attach(f.source(2)))[1], false);
    assert.equal(f.store.files$(), before);
    assert.equal(f.requests(), 0);
  });
}
