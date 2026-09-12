'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const observer = require('../android/app/src/main/assets/chatgpt_web_private_canvas_edit_observer.js');
const { observerFixture } = require('./fixtures/chatgpt-canvas-edit-observer.cjs');
const ID = 'synthetic_original_canvas';
const mutation = (status, patch = {}) => ({ options: { mutationKey: ['canvas', 'textdoc', 'persist'] },
  state: { status, variables: { textdocId: ID, lastVersion: 4 }, submittedAt: 1000, ...patch } });
function fixture() {
  const f = observerFixture();
  const api = observer.create({ document: f.document }, f.runtime, f.conversation, f.client);
  return { ...f, check: () => api.check(ID), dispose: api.dispose };
}

test('reads only a committed hook result, without mounting UI or leaving a subscriber', () => {
  const f = fixture();
  f.check(); f.check();
  assert.deepEqual(f.counts(), { mounts: 2, unmounts: 2 });
  assert.throws(() => f.conversation.useCanvasDirty(ID), /hook_outside_root/);
});
test('debounced website edits prevent native writes before the mutation starts', () => {
  const f = fixture(); f.edits.timestamps[ID] = { lastTriggeredAt: 10, lastFlushedAt: null };
  assert.throws(f.check, /web_edit_pending/);
  f.edits.timestamps[ID].lastFlushedAt = 10;
  f.check();
});
for (const status of ['pending', 'error']) test('flushed timestamps do not hide ' + status + ' saves', () => {
  const f = fixture(); f.rows.push(mutation(status));
  assert.throws(f.check, /web_edit_pending/);
});
test('a later persisted version clears an older failed save, not a newer pending save', () => {
  const f = fixture(); f.rows.push(mutation('error'), mutation('success', { submittedAt: 1100, data: 5 }));
  f.check(); f.rows.push(mutation('pending', { submittedAt: 1200 }));
  assert.throws(f.check, /web_edit_pending/);
});
for (const patch of [{ submittedAt: 900, data: 5 }, { submittedAt: 1100, data: 4 },
  { submittedAt: 1100, data: 5, variables: { textdocId: ID, lastVersion: 3 } }]) {
  test('an unrelated success cannot acknowledge a failed save: ' + JSON.stringify(patch), () => {
    const f = fixture(); f.rows.push(mutation('error'), mutation('success', patch));
    assert.throws(f.check, /web_edit_pending/);
  });
}
test('another Canvas or unrelated mutation cannot block this document', () => {
  const f = fixture();
  f.rows.push(mutation('pending', { variables: { textdocId: 'other' } }),
    { options: { mutationKey: ['other'] }, state: { status: 'error' } });
  f.check();
});
test('a failed save stays protected after cache GC and clears only after a newer success', () => {
  const f = fixture(), error = mutation('error');
  f.emit({ type: 'updated', mutation: error });
  f.emit({ type: 'removed', mutation: error });
  assert.equal(f.rows.length, 0); assert.throws(f.check, /web_edit_pending/);
  f.emit({ type: 'updated', mutation: mutation('success', { submittedAt: 1200, data: 5 }) });
  f.check(); f.dispose();
  assert.equal(f.listeners.size, 0); assert.throws(f.check, /runtime_unavailable/);
});
for (const fault of ['uncommitted', 'hook_throw', 'bad_hook_result', 'cleanup_error', 'unknown_mutation', 'missing_owner']) {
  test('unknown observer state fails closed and cleans up: ' + fault, () => {
    const f = fixture();
    if (fault === 'uncommitted') f.react.useLayoutEffect = () => {};
    if (fault === 'hook_throw') f.conversation.useCanvasDirty = () => { throw Error('bad_hook'); };
    if (fault === 'bad_hook_result') f.conversation.useCanvasDirty = () => undefined;
    if (fault === 'cleanup_error') f.renderer.createRoot = () => ({ render() {}, unmount() { throw Error('cleanup'); } });
    if (fault === 'unknown_mutation') f.rows.push(mutation('mystery'));
    if (fault === 'missing_owner') f.rows.push(mutation('pending', { variables: undefined }));
    assert.throws(f.check, /runtime_unavailable/);
    if (fault !== 'cleanup_error') assert.equal(f.counts().mounts, f.counts().unmounts);
  });
}

test('real React root subscribes, commits, and unmounts without visible UI', {
  skip: !process.env.CHATGPT_REACT_TEST_MODULES && 'Optional installed React/ReactDOM/jsdom validation.'
}, () => {
  const { createRequire } = require('node:module');
  const load = createRequire(process.env.CHATGPT_REACT_TEST_MODULES + '/package.json');
  const { JSDOM } = load('jsdom'), react = load('react');
  const browser = new JSDOM('<!doctype html><body></body>'), oldWindow = global.window, oldDocument = global.document;
  global.window = browser.window; global.document = browser.window.document;
  try {
    const dom = load('react-dom'), renderer = load('react-dom/client');
    let dirty = false, subscribers = 0;
    const conversation = { canvasDirtyInit() {}, useCanvasDirty: () => react.useSyncExternalStore(
      () => { subscribers += 1; return () => { subscribers -= 1; }; }, () => dirty) };
    const api = observer.create(browser.window, { reactApi: () => react, reactDom: () => dom, reactRoot: () => renderer },
      conversation, { getMutationCache: () => ({ getAll: () => [], subscribe: () => () => {} }) });
    api.check(ID); assert.equal(subscribers, 0);
    dirty = true; assert.throws(() => api.check(ID), /web_edit_pending/);
    assert.equal(subscribers, 0); assert.equal(browser.window.document.body.childElementCount, 0); api.dispose();
  } finally { global.window = oldWindow; global.document = oldDocument; browser.window.close(); }
});
