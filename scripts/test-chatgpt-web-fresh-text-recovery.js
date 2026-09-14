'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const moduleApi = require('../android/app/src/main/assets/chatgpt_web_fresh_text_recovery');
const tick = () => new Promise(resolve => setImmediate(resolve));
const deferred = () => { let resolve; const promise = new Promise(done => { resolve = done; }); return { promise, resolve }; };

function fixture(options = {}) {
  const calls = [], boundary = new AbortController();
  let current = true, time = 1000;
  const page = { document: { visibilityState: 'visible' }, AbortController, setTimeout, clearTimeout };
  const owner = { dispatched: true, finished: true, stopBoundary: boundary.signal, stopCurrent: () => current,
    request: { userMessageId: 'fixture-user' }, binding: { canReconcile: () => current } };
  const reconciliation = { async reconcile(...args) {
    calls.push(args);
    return options.reconcile ? options.reconcile(...args) : false;
  } };
  const api = moduleApi.create(page, { reconciliation, delays: [0, 0, 0], timeoutMs: 100,
    now: () => time, ...options });
  return { api, owner, calls, page, boundary, current: value => { current = value; },
    advance: () => { time += 10000; } };
}

test('delayed history is read again without a second write or any draft/DOM dependency', async () => {
  const f = fixture({ reconcile: async () => f.calls.length === 3 });
  const result = await f.api.recover(f.owner);
  assert.deepEqual(result, { status: 'accepted', code: 'history_reconciled' });
  assert.equal(f.calls.length, 3); assert.equal(f.owner.recoveryConfirmed, true);
  assert.ok(f.calls.every(args => args[0] === f.owner.binding && args[1] === f.owner.request));
  assert.equal(f.owner.recovering, false); assert.equal(f.owner.recoveryJob, null);
});

test('concurrent recovery triggers join the same bounded read job', async () => {
  const pending = deferred(), f = fixture({ reconcile: () => pending.promise });
  const first = f.api.recover(f.owner), second = f.api.recover(f.owner, true);
  assert.equal(first, second); assert.equal(f.calls.length, 1);
  pending.resolve(true); await first;
  assert.equal(f.calls.length, 1);
});

test('nonterminal history preserves the write barrier after three attempts', async () => {
  const f = fixture();
  assert.equal((await f.api.recover(f.owner)).code, 'history_reconciliation_pending');
  assert.equal(f.calls.length, 3); assert.notEqual(f.owner.recoveryConfirmed, true);
});

test('automatic retries have visibility, cooldown and per-turn budgets but manual reads remain possible', async () => {
  const f = fixture(); f.page.document.visibilityState = 'hidden';
  assert.equal((await f.api.recover(f.owner, true)).code, 'recovery_deferred');
  assert.equal(f.calls.length, 0);
  f.page.document.visibilityState = 'visible';
  for (let round = 0; round < 3; round++) {
    await f.api.recover(f.owner, true);
    assert.equal(f.calls.length, (round + 1) * 3);
    assert.equal((await f.api.recover(f.owner, true)).code, 'recovery_deferred');
    f.advance();
  }
  assert.equal((await f.api.recover(f.owner, true)).code, 'recovery_deferred');
  await f.api.recover(f.owner); assert.equal(f.calls.length, 12);
});

test('pre-dispatch, live streaming and concurrent stop cannot start history recovery', async () => {
  for (const change of [o => { o.dispatched = false; }, o => { o.finished = false; }, o => { o.stopping = true; }]) {
    const f = fixture(); change(f.owner);
    assert.equal((await f.api.recover(f.owner)).code, 'recovery_not_ready');
    assert.equal(f.calls.length, 0);
  }
});

test('account, document or branch ownership loss stops the remaining reads', async () => {
  const f = fixture({ reconcile: async () => { f.current(false); return true; } });
  assert.equal((await f.api.recover(f.owner)).code, 'context_changed');
  assert.equal(f.calls.length, 1); assert.notEqual(f.owner.recoveryConfirmed, true);
  assert.equal((await f.api.recover(f.owner)).code, 'context_changed');
  assert.equal(f.calls.length, 1);
});

test('timeout cancels the exact read and a late success cannot confirm a turn', async () => {
  const pending = deferred(), f = fixture({ timeoutMs: 10, reconcile: () => pending.promise });
  const result = await f.api.recover(f.owner);
  assert.equal(result.code, 'reconciliation_timeout');
  assert.equal(f.calls[0][2].aborted, true);
  pending.resolve(true); await tick();
  assert.notEqual(f.owner.recoveryConfirmed, true); assert.equal(f.calls.length, 1);
});

test('document cancellation and stop preemption cancel rather than restart reads', async () => {
  for (const abort of [f => f.boundary.abort(), f => f.owner.recoveryController.abort()]) {
    const pending = deferred(), f = fixture({ reconcile: () => pending.promise });
    const job = f.api.recover(f.owner); abort(f);
    const result = await job;
    assert.ok(['context_changed', 'recovery_cancelled'].includes(result.code));
    pending.resolve(true); await tick(); assert.notEqual(f.owner.recoveryConfirmed, true);
    assert.equal(f.calls[0][2].aborted, true);
  }
});

test('a stop attempt admits partial history; only its acknowledgement admits an empty stopped turn', async () => {
  for (const acknowledged of [false, true]) {
    const f = fixture({ reconcile: async () => true });
    f.owner.stopAttempted = true; f.owner.stopAcknowledged = acknowledged;
    await f.api.recover(f.owner);
    assert.equal(f.calls[0][3], true); assert.equal(f.calls[0][4], acknowledged);
  }
});

test('runtime errors are bounded and never leaked in recovery receipts', async () => {
  const f = fixture({ reconcile: () => { throw Error('fixture-private-details'); } });
  const result = await f.api.recover(f.owner);
  assert.deepEqual(result, { status: 'unknown', code: 'history_unavailable' });
  assert.equal(f.calls.length, 1); assert.notEqual(f.owner.recoveryConfirmed, true);
});

test('cooldown wakeups merge and an unsuccessful wakeup does not start polling', async t => {
  t.mock.timers.enable({ apis: ['setTimeout', 'Date'], now: 1000 });
  let wakeups = 0;
  const f = fixture({ now: Date.now, onAutomaticReady: () => { wakeups++; f.api.recover(f.owner, true); } });
  await f.api.recover(f.owner, true);
  for (let i = 0; i < 5; i++) assert.equal((await f.api.recover(f.owner, true)).code, 'recovery_deferred');
  const scheduled = f.owner.recoveryWakeup;
  assert.ok(scheduled); assert.equal(f.owner.autoRecoveryCount, 1);
  t.mock.timers.tick(10000); await tick();
  assert.equal(wakeups, 1); assert.equal(f.calls.length, 6);
  assert.equal(f.owner.recoveryWakeup, null); assert.equal(f.owner.autoRecoveryCount, 2);
  t.mock.timers.tick(60000); await tick(); assert.equal(wakeups, 1);
  await f.api.recover(f.owner, true);
  assert.equal(f.owner.autoRecoveryCount, 3);
  await f.api.recover(f.owner, true);
  assert.equal(f.owner.recoveryWakeup, null); assert.equal(f.calls.length, 9);
});

test('offline automatic recovery neither reads nor consumes retry budget', async () => {
  const f = fixture(); f.page.navigator = { onLine: false };
  assert.equal((await f.api.recover(f.owner, true)).code, 'recovery_deferred');
  assert.equal(f.calls.length, 0); assert.equal(f.owner.autoRecoveryCount, undefined);
  f.page.navigator.onLine = true;
  await f.api.recover(f.owner, true); assert.equal(f.calls.length, 3);
});

test('delayed wakeup rechecks visibility, network, ownership, stop and completion', async t => {
  t.mock.timers.enable({ apis: ['setTimeout', 'Date'], now: 1000 });
  for (const change of [f => { f.page.document.visibilityState = 'hidden'; },
    f => { f.page.navigator = { onLine: false }; }, f => f.current(false),
    f => { f.owner.stopping = true; }, f => { f.owner.stopConfirmed = true; },
    f => { f.owner.recoveryConfirmed = true; }, f => f.boundary.abort()]) {
    let wakeups = 0;
    const f = fixture({ now: Date.now, onAutomaticReady: () => { wakeups++; } });
    await f.api.recover(f.owner, true); await f.api.recover(f.owner, true);
    change(f); t.mock.timers.tick(10000); await tick();
    assert.equal(wakeups, 0); assert.equal(f.calls.length, 3);
    assert.equal(f.owner.recoveryWakeup, null);
  }
});

test('manual recovery or explicit suspension removes the pending automatic wakeup', async t => {
  t.mock.timers.enable({ apis: ['setTimeout', 'Date'], now: 1000 });
  for (const manual of [false, true]) {
    let wakeups = 0;
    const f = fixture({ now: Date.now, onAutomaticReady: () => { wakeups++; } });
    await f.api.recover(f.owner, true); await f.api.recover(f.owner, true);
    assert.ok(f.owner.recoveryWakeup);
    if (manual) await f.api.recover(f.owner); else f.api.cancelScheduled(f.owner);
    assert.equal(f.owner.recoveryWakeup, null);
    t.mock.timers.tick(10000); await tick(); assert.equal(wakeups, 0);
    assert.equal(f.calls.length, manual ? 6 : 3);
  }
});

test('suspension clears a resume queued behind an in-flight read', async t => {
  t.mock.timers.enable({ apis: ['setTimeout', 'Date'], now: 1000 });
  const pending = deferred(); let wakeups = 0;
  const f = fixture({ now: Date.now, reconcile: () => pending.promise,
    onAutomaticReady: () => { wakeups++; } });
  const job = f.api.recover(f.owner, true);
  f.api.requestAutomatic(f.owner); assert.equal(f.owner.recoveryResumeRequested, true);
  f.api.cancelScheduled(f.owner);
  pending.resolve(false); await job;
  t.mock.timers.tick(10000); await tick();
  assert.equal(f.owner.recoveryResumeRequested, false);
  assert.equal(f.owner.recoveryWakeup, undefined); assert.equal(wakeups, 0);
});

test('automatic history recovery stops between reads when visibility or network is lost', async () => {
  for (const mode of ['hidden', 'offline']) {
    const f = fixture({ reconcile: async () => {
      if (mode === 'hidden') f.page.document.visibilityState = 'hidden';
      else f.page.navigator = { onLine: false };
      return false;
    } });
    assert.equal((await f.api.recover(f.owner, true)).code, 'recovery_deferred');
    assert.equal(f.calls.length, 1);
    assert.equal(f.calls[0][2].aborted, true);
    assert.notEqual(f.owner.recoveryConfirmed, true);
  }
});

test('suspension aborts an automatic read and a queued resume rereads without accepting its late result', async t => {
  t.mock.timers.enable({ apis: ['setTimeout', 'Date'], now: 1000 });
  const pending = deferred(); let wakeups = 0;
  const f = fixture({ now: Date.now, reconcile: () => f.calls.length === 1 ? pending.promise : true,
    onAutomaticReady: () => { wakeups++; f.api.recover(f.owner, true); } });
  const first = f.api.recover(f.owner, true);
  f.api.suspend(f.owner);
  assert.equal(f.calls[0][2].aborted, true);
  f.api.requestAutomatic(f.owner);
  assert.equal((await first).code, 'recovery_deferred');
  pending.resolve(true); await tick();
  assert.notEqual(f.owner.recoveryConfirmed, true);
  assert.equal(f.calls.length, 1);
  t.mock.timers.tick(10000); await tick();
  assert.equal(wakeups, 1); assert.equal(f.calls.length, 2);
  assert.equal(f.owner.recoveryConfirmed, true);
  assert.equal(f.owner.suspendRecovery, null);
});

test('suspension does not cancel an explicit manual history check', async () => {
  const pending = deferred(), f = fixture({ reconcile: () => pending.promise });
  const job = f.api.recover(f.owner);
  f.api.suspend(f.owner);
  assert.equal(f.calls[0][2].aborted, false);
  pending.resolve(true);
  assert.equal((await job).status, 'accepted');
});

test('an already-cancelled document cannot start a history read', async () => {
  const f = fixture(); f.boundary.abort();
  assert.equal((await f.api.recover(f.owner)).code, 'context_changed');
  assert.equal(f.calls.length, 0);
});
