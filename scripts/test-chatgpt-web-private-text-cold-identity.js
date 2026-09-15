'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture } = require('./fixtures/chatgpt-private-text-input');
const tick = () => new Promise(resolve => setImmediate(resolve));

function coldIdentity() {
  const f = fixture(), page = f.page, bindings = page.__elonChatGptPrivateRuntimeBindings;
  f.loads.length = 0; // Exclude the existing runtime's constructor composer prewarm.
  const peek = bindings.peek, load = bindings.load;
  const model = page.__elonChatGptPrivateModelContract.create();
  page.__elonChatGptPrivateModelContract = { create: () => ({ ...model,
    withRuntimeIdentity: (value, shared) => shared ? model.withRuntimeIdentity(value, shared) : null }) };
  let ready = false, resolve, reject, imports = 0, notifications = 0, sequence = 0;
  const timers = new Map();
  page.setTimeout = (fn, ms) => { const id = ++sequence; timers.set(id, { fn, ms }); return id; };
  page.clearTimeout = id => timers.delete(id);
  bindings.peek = role => role === 'shared' && !ready ? null : peek(role);
  bindings.load = role => {
    if (role !== 'shared' || ready) return load(role);
    imports++;
    return new Promise((yes, no) => { resolve = () => { ready = true; yes(f.shared); }; reject = no; });
  };
  return { ...f, page, bindings, timers, imports: () => imports,
    resolve: () => resolve(), reject: () => reject(Error('runtime_timeout')),
    notify: () => { notifications++; }, notifications: () => notifications,
    snapshot: () => f.api.snapshot(null, () => { notifications++; }),
    runTimer(ms) {
      const entry = [...timers].find(([, timer]) => timer.ms === ms);
      assert.ok(entry, 'expected bounded observer timer');
      timers.delete(entry[0]); entry[1].fn();
    } };
}

test('native input starts the cold identity import instead of waiting for another feature to load it', async () => {
  const f = coldIdentity();
  assert.equal(f.context.stamp(), null);
  for (let n = 0; n < 20; n++) assert.deepEqual(f.snapshot(), { ready: false, draft: null });
  await tick();
  assert.equal(f.imports(), 1);
  assert.deepEqual(f.loads, [], 'only the shared identity module is requested first');
  assert.equal(f.api.setDraft('must not write', f.draft()), false);
  f.resolve(); await tick();
  assert.equal(f.notifications(), 1);
  f.snapshot(); await tick();
  assert.deepEqual(f.snapshot(), { ready: true, draft: f.draft() });
  assert.deepEqual(f.edits, []);
  assert.equal(f.timers.size, 0);
});

for (const [name, change] of [
  ['disabled transport', f => { f.page.__elonChatGptPrivateTextTransactionsEnabled = false; }],
  ['disabled fresh input', f => { f.page.__elonChatGptFreshTextDispatchEnabled = false; }],
  ['unknown profile', f => { f.bindings.state = () => ({ profile_id: 'unreviewed' }); }],
  ['unobserved shared asset', f => { f.bindings.observed = role => role !== 'shared'; }],
  ['login route', f => { f.page.location.href = 'https://chatgpt.com/auth/login'; }],
  ['foreign origin', f => { f.page.location.href = 'https://example.test/'; }],
  ['missing document identity', f => { f.page.__elonChatGptDocumentToken = ''; }],
  ['hidden page', f => { f.page.document.hidden = true; }],
  ['offline page', f => { f.page.navigator = { onLine: false }; }],
]) {
  test(`${name} never starts cold private input work`, async () => {
    const f = coldIdentity(); change(f); f.snapshot(); await tick();
    assert.equal(f.imports(), 0); assert.equal(f.notifications(), 0);
    assert.equal(f.timers.size, 0); assert.deepEqual(f.edits, []);
  });
}

for (const [name, change] of [
  ['document', f => { f.page.document = {}; }],
  ['document token', f => { f.page.__elonChatGptDocumentToken = 'doc_another'; }],
  ['route', f => { f.page.location.href = 'https://chatgpt.com/'; }],
  ['profile', f => { f.bindings.state = () => ({ profile_id: 'web_20260915' }); }],
  ['binding instance', f => { f.page.__elonChatGptPrivateRuntimeBindings = { ...f.bindings }; }],
  ['disabled transport', f => { f.page.__elonChatGptPrivateTextTransactionsEnabled = false; }],
  ['hidden page', f => { f.page.document.hidden = true; }],
  ['offline page', f => { f.page.navigator = { onLine: false }; }],
]) {
  test(`late identity import does not notify a changed ${name}`, async () => {
    const f = coldIdentity(); f.snapshot(); await tick();
    assert.equal(f.imports(), 1); change(f); f.resolve(); await tick();
    assert.equal(f.notifications(), 0); assert.equal(f.timers.size, 0);
    assert.equal(f.api.setDraft('must not write', f.draft()), false);
    assert.deepEqual(f.edits, []);
  });
}

test('failed imports back off and stop after three attempts rather than polling indefinitely', async () => {
  const f = coldIdentity();
  for (let attempt = 1; attempt <= 3; attempt++) {
    f.snapshot(); await tick(); assert.equal(f.imports(), attempt);
    f.reject(); await tick();
    for (let n = 0; n < 20; n++) f.snapshot();
    await tick(); assert.equal(f.imports(), attempt);
    f.advance(10000);
  }
  f.snapshot(); await tick();
  assert.equal(f.imports(), 3); assert.equal(f.notifications(), 0);
  assert.equal(f.timers.size, 0); assert.deepEqual(f.edits, []);
});

test('a timed-out observer ignores late completion without cancelling other module consumers', async () => {
  const f = coldIdentity(); f.snapshot(); await tick();
  assert.equal(f.imports(), 1); f.runTimer(5000);
  f.resolve(); await tick();
  assert.equal(f.notifications(), 0);
  f.snapshot(); await tick();
  assert.equal(f.snapshot().ready, true, 'a fresh snapshot may use the now-loaded module');
  assert.deepEqual(f.edits, []); assert.equal(f.timers.size, 0);
});

test('a loaded but signed-out identity is not mistaken for permission to send or retried forever', async () => {
  const f = coldIdentity(); f.identity(null); f.snapshot(); await tick();
  assert.equal(f.imports(), 1); f.resolve(); await tick();
  for (let n = 0; n < 20; n++) { f.advance(10000); assert.equal(f.snapshot().ready, false); }
  assert.equal(f.notifications(), 0); assert.equal(f.imports(), 1);
  assert.deepEqual(f.edits, []);
});

test('an account change during import only wakes a fresh validation, never captures or changes its draft', async () => {
  const f = coldIdentity(); f.snapshot(); await tick();
  f.identity('different-synthetic-account'); f.resolve(); await tick();
  assert.equal(f.notifications(), 1);
  assert.equal(f.api.setDraft('must not write', f.draft()), false);
  assert.deepEqual(f.edits, []);
});

test('an immediate snapshot notification can start owner validation without losing its new deadline', async () => {
  const f = coldIdentity(); let notifications = 0;
  const notify = () => { notifications++; f.api.snapshot(null, notify); };
  f.api.snapshot(null, notify); await tick(); f.resolve(); await tick();
  assert.equal(notifications, 2, 'identity and validated draft each notify once');
  assert.equal(f.snapshot().ready, true); assert.equal(f.timers.size, 0);
  assert.deepEqual(f.edits, []);
});

test('network flapping does not reset the failed identity import cooldown', async () => {
  const f = coldIdentity(); f.snapshot(); await tick(); f.reject(); await tick();
  for (let n = 0; n < 20; n++) {
    f.page.navigator = { onLine: false }; f.snapshot();
    f.page.navigator.onLine = true; f.snapshot();
  }
  await tick(); assert.equal(f.imports(), 1);
  f.advance(10000); f.snapshot(); await tick(); assert.equal(f.imports(), 2);
  f.reject(); await tick(); assert.equal(f.timers.size, 0);
});

test('loading identity does not bypass a missing committed owner, changed model, or pending attachment', async () => {
  for (const change of [f => { f.root.child = null; },
    f => { f.conversation.Nrn = () => null; },
    f => { f.files.hasUploadInProgress$ = () => true; }]) {
    const f = coldIdentity(); f.snapshot(); await tick(); change(f);
    f.resolve(); await tick(); f.snapshot(); await tick();
    assert.equal(f.snapshot().ready, false);
    assert.equal(f.api.setDraft('must not write', f.draft()), false);
    assert.deepEqual(f.edits, []); assert.equal(f.timers.size, 0);
  }
});

test('cold identity and editor caches prepare in order without any composer DOM', async () => {
  const f = coldIdentity(), peek = f.bindings.peek, load = f.bindings.load;
  let editorReady = false, completeEditor, editorImports = 0, editorJob;
  f.bindings.peek = role => role === 'composer' && !editorReady ? null : peek(role);
  f.bindings.load = role => {
    if (role !== 'composer' || editorReady) return load(role);
    if (!editorJob) {
      editorImports++;
      editorJob = new Promise(resolve => { completeEditor = () => { editorReady = true; resolve(f.editor); }; });
    }
    return editorJob;
  };
  f.snapshot(); await tick(); assert.equal(editorImports, 0);
  f.resolve(); await tick(); f.snapshot(); await tick();
  assert.equal(editorImports, 1); assert.equal(f.snapshot().ready, false);
  completeEditor(); await tick();
  assert.equal(f.snapshot().ready, true); assert.equal(f.view.dom.isConnected, false);
  assert.deepEqual(f.edits, []); assert.equal(f.timers.size, 0);
});

test('a pending model remains not-ready and recovers when its real configuration arrives', async () => {
  const f = coldIdentity(), readModel = f.conversation.Nrn;
  f.snapshot(); await tick(); f.resolve(); await tick();
  for (const id of [undefined, '', false, {}, 'x'.repeat(129), 'invalid model']) {
    f.conversation.Nrn = () => id === undefined ? null : { id };
    f.advance(2000); f.snapshot(); await tick();
    assert.equal(f.snapshot().ready, false);
    await assert.rejects(f.context.capture(null), error =>
      error.message === 'context_unavailable' && error.admissionStage === 'base_model');
  }
  f.conversation.Nrn = readModel; f.advance(2000); f.snapshot(); await tick();
  assert.equal(f.snapshot().ready, true);
  assert.equal(f.imports(), 1); assert.deepEqual(f.edits, []);
});
