'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture } = require('./fixtures/chatgpt-private-text-input');
const tick = () => new Promise(resolve => setImmediate(resolve));

test('production native submit UI uses text readiness without relabeling voice readiness', () => {
  const fs = require('node:fs'), path = require('node:path');
  const root = path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app');
  const ui = fs.readFileSync(path.join(root, 'MainSocialAiChatFeature.kt'), 'utf8');
  assert.equal((ui.match(/composerReady = controller.textInputReady\(\)/g) || []).length, 2);
  const voice = fs.readFileSync(path.join(root, 'MainRealtimeVoiceCoordinatorFactory.kt'), 'utf8');
  assert.match(voice, /controller.composerReady\(\)/);
  const controller = fs.readFileSync(path.join(root, 'ChatGptSocialChatController.kt'), 'utf8');
  assert.match(controller, /override fun composerReady\(\): Boolean = session.currentSnapshot\(\)\?\.composerReady == true/);
  assert.match(controller, /override fun textInputReady\(\).*ChatGptWebAccessPolicy::canSendText/);
  const snapshot = fs.readFileSync(path.join(root, 'chatgptweb/WebChatSnapshotStore.kt'), 'utf8');
  assert.equal(snapshot.includes('privateSendReady'), false, 'private runtime readiness is never persisted to disk');
});

test('production fresh context uses the committed official draft with no mounted composer', async () => {
  const f = fixture();
  assert.equal(f.runtime.captureConversation(null), null, 'legacy DOM sender is not relaxed');
  const owner = await f.api.snapshot(null, () => {});
  assert.deepEqual(owner, { ready: false, draft: null });
  await tick();
  assert.deepEqual(f.api.snapshot(null), { ready: true, draft: f.draft() });
  assert.equal(f.edits.length, 0);
  const binding = await f.context.capture(null);
  assert.equal(binding.draft.read(), f.draft());
  assert.equal(binding.current(), true);
});

test('cold input preparation is single-flight, not one import per snapshot', async () => {
  const f = fixture(); let notified = 0;
  for (let n = 0; n < 20; n++) f.api.snapshot(null, () => { notified++; });
  await tick();
  assert.equal(notified, 1);
  const loaded = f.loads.length;
  for (let n = 0; n < 20; n++) assert.equal(f.api.snapshot(null).ready, true);
  assert.equal(f.loads.length, loaded);
  assert.equal(f.api.snapshot({}).ready, false, 'mounted DOM path remains authoritative');
});

test('native draft mutation uses the exact official editor without focus or sending', async () => {
  const f = fixture(); f.api.snapshot(null); await tick();
  assert.equal(f.api.setDraft('changed fixture', 'not current'), false);
  assert.equal(f.api.setDraft('changed fixture', f.draft()), true);
  assert.equal(f.api.snapshot(null).draft, 'changed fixture');
  assert.deepEqual(f.edits, ['changed fixture']);
  f.page.__elonChatGptFreshTextDispatchEnabled = false;
  assert.equal(f.api.setDraft('other', f.draft()), false);
  assert.equal(f.api.snapshot(null).ready, false);
});

function toolInput(hint, enabled) {
  const f = fixture();
  f.hints.activeSystemHintType = hint;
  f.page.__elonChatGptFreshTextToolsEnabled = enabled;
  f.page.__elonChatGptPrivateComposerToolContext = { capture: () => {
    const owner = f.runtime.capturePrivateConversation(null);
    return owner && { ...owner, document: f.page.document,
      model: f.conversation.Nrn(f.selected).id, allowed: 'search,picture_v2' };
  } };
  return f;
}

test('accepted personal Search keeps native input usable without composer DOM', async () => {
  const f = toolInput('search');
  f.api.snapshot(null); await tick();
  assert.deepEqual(f.api.snapshot(null), { ready: true, draft: f.draft() });
  assert.equal(f.api.setDraft('Synthetic Search draft', f.draft()), true);
  assert.deepEqual(f.edits, ['Synthetic Search draft']);
  assert.equal(f.hints.activeSystemHintType, 'search', 'editing must not reset the selected tool');
});

test('additional tool input follows the explicit sender switch, not preset visibility', async () => {
  for (const enabled of [undefined, false, true]) {
    const f = toolInput('picture_v2', enabled);
    f.api.snapshot(null); await tick();
    assert.equal(f.api.snapshot(null).ready, enabled === true);
    assert.equal(f.api.setDraft('Synthetic picture draft', f.draft()), enabled === true);
    assert.equal(f.edits.length, enabled === true ? 1 : 0);
  }
});

test('disabled Search does not inherit the accepted default', async () => {
  const f = toolInput('search', false);
  f.api.snapshot(null); await tick();
  assert.equal(f.api.snapshot(null).ready, false);
  assert.equal(f.api.setDraft('must not write', f.draft()), false);
  assert.equal(f.edits.length, 0);
});

test('turning off the tool switch revokes a cached native draft immediately', async () => {
  const f = toolInput('search');
  f.api.snapshot(null); await tick();
  assert.equal(f.api.snapshot(null).ready, true);
  f.page.__elonChatGptFreshTextToolsEnabled = false;
  assert.equal(f.api.setDraft('must not write', f.draft()), false);
  assert.equal(f.api.snapshot(null).ready, false);
  assert.equal(f.edits.length, 0);
  await tick();
});

test('a changed tool cannot reuse previously authorized Search input', async () => {
  const f = toolInput('search');
  f.api.snapshot(null); await tick();
  assert.equal(f.api.snapshot(null).ready, true);
  f.hints.activeSystemHintType = 'picture_v2';
  assert.equal(f.api.setDraft('must not write', f.draft()), false);
  assert.equal(f.api.snapshot(null).ready, false);
  await tick();
  assert.equal(f.api.snapshot(null).ready, false);
  assert.equal(f.edits.length, 0);
});

test('tool capability or model mismatch cannot grant native input', async () => {
  for (const field of ['allowed', 'model']) {
    const f = toolInput('search');
    const capture = f.page.__elonChatGptPrivateComposerToolContext.capture;
    f.page.__elonChatGptPrivateComposerToolContext.capture = () => ({ ...capture(), [field]: 'not-matching' });
    f.api.snapshot(null); await tick();
    assert.equal(f.api.snapshot(null).ready, false);
    assert.equal(f.api.setDraft('must not write', f.draft()), false);
    assert.equal(f.edits.length, 0);
  }
});

test('draft command preserves mounted DOM semantics and uses memory only when absent', async () => {
  const f = fixture(), receipts = [], signals = [], dom = {}, writes = [];
  const respond = (...args) => receipts.push(args);
  const io = { find: () => dom, read: () => 'DOM draft', compare: text => text.trim(),
    write: (node, value) => { assert.equal(node, dom); writes.push(value); return true; },
    notify: force => signals.push(force) };
  f.api.setCommand('first', 'wrong', respond, io);
  assert.equal(receipts.at(-1)[1], false); assert.equal(writes.length, 0);
  f.api.setCommand('second', ' DOM draft ', respond, io);
  assert.equal(receipts.at(-1)[1], true); assert.deepEqual(writes, ['second']);
  f.api.snapshot(null); await tick(); io.find = () => null;
  f.api.setCommand('private draft', f.draft(), respond, io);
  assert.equal(receipts.at(-1)[1], true); assert.equal(f.draft(), 'private draft');
  assert.equal(writes.length, 1); assert.equal(signals.at(-1), true);
});

for (const [name, change] of Object.entries({
  account: f => f.identity('another fixture'),
  route: f => { f.page.location.href = 'https://chatgpt.com/'; },
  document: f => { f.page.document = {}; },
  token: f => { f.page.__elonChatGptDocumentToken = 'doc_another_input'; },
  owner: f => { f.props.composerController = { conversation: f.selected }; },
  editor: f => { f.view.isDestroyed = true; },
  upload: f => { f.files.hasUploadInProgress$ = () => true; },
  tool: f => { f.hints.activeSystemHintType = 'search'; },
  parent: f => { f.props.currentLeafId = 'another leaf'; },
  uncommitted: f => { f.root.stateNode.current = { stateNode: f.root.stateNode }; },
})) {
  test('cached private input revokes admission immediately on ' + name, async () => {
    const f = fixture(); f.api.snapshot(null); await tick();
    assert.equal(f.api.snapshot(null).ready, true);
    change(f);
    assert.equal(f.api.setDraft('must not write', f.draft()), false);
    assert.equal(f.api.snapshot(null).ready, false);
    assert.equal(f.edits.length, 0);
    await tick();
  });
}

test('a second conflicting committed file owner is not silently selected', () => {
  const f = fixture();
  const sibling = { ...f.fiber, dependencies: { firstContext: { memoizedValue: f.binding.shared,
    next: { memoizedValue: { ...f.files } } } } };
  f.fiber.sibling = sibling;
  assert.equal(f.runtime.capturePrivateConversation(null), null);
});

test('bounded root traversal rejects cycles and unknown container ownership', () => {
  const f = fixture(); f.fiber.sibling = f.fiber;
  assert.equal(f.runtime.capturePrivateConversation(null), null);
  f.fiber.sibling = null; f.root.stateNode.containerInfo = {};
  assert.equal(f.runtime.capturePrivateConversation(null), null);
});

test('retained initial HostRoot field follows the new committed tree', () => {
  const f = fixture(), next = { stateNode: f.root.stateNode, child: f.fiber, return: null };
  f.root.alternate = next; next.alternate = f.root;
  f.root.stateNode.current = next;
  const binding = f.runtime.capturePrivateConversation(null);
  assert.equal(binding?.draft.read(), f.draft(), 'bailout return pointer can retain the old parent');
});

function commitInputRootAgain(f, child) {
  const alternate = { stateNode: f.root.stateNode, return: null };
  f.root.alternate = alternate; alternate.alternate = f.root;
  f.root.stateNode.current = alternate;
  f.root.child = child;
  f.root.stateNode.current = f.root;
}

test('private input rediscovers an owner after a negative lookup and HostRoot reuse', () => {
  const f = fixture(); f.root.child = null;
  assert.equal(f.runtime.capturePrivateConversation(null), null);
  commitInputRootAgain(f, f.fiber);
  assert.equal(f.runtime.capturePrivateConversation(null)?.draft.read(), f.draft());
});

test('private input rediscovers a replacement owner after HostRoot reuse', () => {
  const f = fixture();
  assert.equal(f.runtime.capturePrivateConversation(null)?.draft.read(), f.draft());
  commitInputRootAgain(f, { ...f.fiber });
  assert.equal(f.runtime.capturePrivateConversation(null)?.draft.read(), f.draft());
});

test('cached input refuses a new conflicting file owner after HostRoot reuse', async () => {
  const f = fixture(); f.api.snapshot(null); await tick();
  assert.equal(f.api.snapshot(null).ready, true);
  f.fiber.sibling = { ...f.fiber, dependencies: { firstContext: { memoizedValue: f.binding.shared,
    next: { memoizedValue: { ...f.files } } } } };
  commitInputRootAgain(f, f.fiber);
  assert.equal(f.api.setDraft('must not write', f.draft()), false);
  assert.equal(f.edits.length, 0);
  assert.equal(f.api.snapshot(null).ready, false);
  await tick();
});

test('rich input, overlong drafts and unmapped runtime are not mistaken for empty text', () => {
  for (const change of [f => { f.draft('x'.repeat(20001)); },
    f => { f.view.state.doc.toJSON = () => ({ type: 'doc', content: [{ type: 'image' }] }); },
    f => { f.page.__elonChatGptPrivateRuntimeBindings.observed = () => false; }]) {
    const f = fixture(); change(f);
    assert.equal(f.runtime.capturePrivateConversation(null), null);
    assert.equal(f.edits.length, 0);
  }
});

test('timed-out or previous-account preparation cannot publish readiness later', async () => {
  let finish, time = 0, scheduled, calls = 0, notified = 0, stamp = 'fixture-owner';
  const binding = { current: () => true, draft: { read: () => 'late synthetic draft' } };
  const context = { stamp: () => stamp, capture: () => { calls++; return new Promise(resolve => { finish = resolve; }); } };
  const page = { document: {}, __elonChatGptPrivateTextTransactionsEnabled: true,
    setTimeout: fn => { scheduled = fn; return 1; }, clearTimeout() {} };
  const api = require('../android/app/src/main/assets/chatgpt_web_private_text_input').create(page,
    { context, now: () => time });
  api.snapshot(null, () => { notified++; }); scheduled();
  finish(binding); await tick();
  assert.equal(notified, 0); assert.equal(api.snapshot(null).ready, false); assert.equal(calls, 1);
  time = 2001; api.snapshot(null, () => { notified++; }); assert.equal(calls, 2);
  stamp = 'another fixture owner'; finish(binding); await tick();
  assert.equal(notified, 0); assert.equal(api.snapshot(null).ready, false);
  finish(binding); await tick();
});

for (const boundary of ['account', 'scope', 'document']) {
  test('pending input preparation yields immediately to a changed ' + boundary, async () => {
    let stamp = 'first-owner', notified = 0, timerId = 0;
    const jobs = [], timers = new Map();
    const page = { document: {}, __elonChatGptPrivateTextTransactionsEnabled: true,
      setTimeout(fn) { timers.set(++timerId, fn); return timerId; }, clearTimeout: id => timers.delete(id) };
    const context = { stamp: () => stamp,
      capture: () => new Promise(resolve => jobs.push(resolve)) };
    const api = require('../android/app/src/main/assets/chatgpt_web_private_text_input').create(page, { context });
    api.snapshot(null, () => { notified++; });
    if (boundary === 'account') stamp = 'second-owner';
    if (boundary === 'scope') page.__elonChatGptFreshTextToolsEnabled = true;
    if (boundary === 'document') page.document = {};
    api.snapshot(null, () => { notified++; });
    assert.equal(jobs.length, 2, 'new owner does not wait for the old five-second deadline');
    assert.equal(timers.size, 1, 'only the current capture retains a deadline');
    jobs[0]({ current: () => true, draft: { read: () => 'old synthetic draft' } }); await tick();
    assert.equal(notified, 0);
    assert.equal(api.snapshot(null).ready, false);
    assert.equal(jobs.length, 2, 'late completion must not retire the new capture');
    jobs[1]({ current: () => true, draft: { read: () => 'current synthetic draft' } }); await tick();
    assert.deepEqual(api.snapshot(null), { ready: true, draft: 'current synthetic draft' });
    assert.equal(notified, 1); assert.equal(timers.size, 0);
  });
}

test('disabling private input retires its local wait without publishing a late binding', async () => {
  let resolve, notified = 0;
  const timers = new Set();
  const page = { document: {}, __elonChatGptPrivateTextTransactionsEnabled: true,
    setTimeout(fn) { timers.add(fn); return fn; }, clearTimeout: timer => timers.delete(timer) };
  const context = { stamp: () => 'fixture-owner', capture: () => new Promise(done => { resolve = done; }) };
  const api = require('../android/app/src/main/assets/chatgpt_web_private_text_input').create(page, { context });
  api.snapshot(null, () => { notified++; });
  page.__elonChatGptPrivateTextTransactionsEnabled = false;
  assert.equal(api.snapshot(null).ready, false);
  assert.equal(timers.size, 0);
  resolve({ current: () => true, draft: { read: () => 'must not publish' } }); await tick();
  assert.equal(notified, 0); assert.equal(api.snapshot(null).ready, false);
});
