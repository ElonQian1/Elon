'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture: journalFixture, asset, uid } = require('./fixtures/chatgpt-fresh-text-journal');
const sessionModule = asset('recovery_session');
const tick = () => new Promise(resolve => setImmediate(resolve));
const listeners = () => {
  const items = new Map();
  return { addEventListener: (name, action) => items.set(name, action),
    removeEventListener: name => items.delete(name), emit: name => items.get(name)?.(), items };
};
async function fixture(options = {}) {
  const f = journalFixture(options);
  if (options.pending !== false) (await f.prepare()).persist();
  const windowEvents = listeners(), documentEvents = listeners(), timers = new Map();
  let clock = 0, serial = 0, enabled = true, writer = false, stamp = 'account-and-conversation', captures = 0;
  Object.assign(f.page, windowEvents, { document: { ...documentEvents, visibilityState: 'visible' },
    __elonChatGptDocumentToken: 'doc_recovery_test', navigator: { onLine: true },
    location: { href: 'https://chatgpt.com/c/' + uid(2) },
    setTimeout: (action, delay) => { timers.set(++serial, { action, at: clock + delay }); return serial; },
    clearTimeout: id => timers.delete(id) });
  const phases = [], session = sessionModule.create(f.page, {
    now: () => clock, enabled: () => enabled, hasWriter: () => writer, stamp: () => stamp,
    context: { capture: async signal => { captures++; return options.capture ? options.capture(signal, f) : f.binding; } },
    journal: () => f.api, timeoutMs: 15000,
  });
  const snapshot = () => session.snapshot(() => phases.push(session.snapshot()));
  const advance = async milliseconds => {
    clock += milliseconds;
    for (const [id, timer] of [...timers]) if (timer.at <= clock) { timers.delete(id); timer.action(); }
    await tick();
  };
  return { ...f, session, snapshot, phases, timers, advance, windowEvents, documentEvents,
    captures: () => captures, enable: value => { enabled = value; }, write: value => { writer = value; },
    stamp: value => { stamp = value; } };
}
async function until(predicate) {
  for (let i = 0; i < 100; i++) { if (predicate()) return; await tick(); }
  assert.fail('fixture did not settle');
}

test('opening a conversation recovers a pending write without Send or composer access', async () => {
  const f = await fixture();
  try {
    assert.equal(f.snapshot(), 'idle');
    await until(() => f.snapshot() === 'recovered');
    assert.deepEqual(f.phases, ['checking', 'recovered']);
    assert.equal(f.rows().length, 0);
    assert.deepEqual(f.calls.map(call => call.kind), ['history', 'apply']);
    for (let i = 0; i < 100; i++) f.snapshot();
    assert.equal(f.captures(), 1); assert.equal(f.timers.size, 0);
  } finally { f.session.dispose(); }
});

test('no pending write means no history request and no checking banner', async () => {
  const f = await fixture({ pending: false });
  try {
    f.snapshot(); await until(() => f.snapshot() === 'clear');
    assert.deepEqual(f.calls, []); assert.deepEqual(f.phases, ['clear']);
    await f.advance(3600000); assert.equal(f.captures(), 1);
  } finally { f.session.dispose(); }
});

test('disabled candidate does not inspect identity, storage or runtime', async () => {
  const f = await fixture();
  try {
    f.enable(false); assert.equal(f.snapshot(), 'disabled');
    f.windowEvents.emit('online'); await tick();
    assert.equal(f.captures(), 0); assert.equal(f.calls.length, 0);
  } finally { f.session.dispose(); }
});

test('unknown new-conversation ID remains unconfirmed, never scans or resends', async () => {
  const f = await fixture({ pending: false });
  try {
    f.binding.conversationId = null; f.binding.newConversation = true; f.binding.parentId = 'client-created-root';
    (await f.prepare()).persist();
    f.snapshot(); await until(() => f.snapshot() === 'unconfirmed');
    assert.equal(f.rows().length, 1); assert.equal(f.calls.length, 0);
    assert.equal(f.timers.size, 0);
  } finally { f.session.dispose(); }
});

test('a record from another conversation is not read or projected into the selected chat', async () => {
  const f = await fixture();
  try {
    f.binding.conversationId = uid(8);
    f.snapshot(); await until(() => f.snapshot() === 'clear');
    assert.equal(f.rows().length, 1); assert.equal(f.calls.length, 0);
  } finally { f.session.dispose(); }
});

test('history failure stays uncertain without polling, then recovers on online', async () => {
  let fail = true;
  const f = await fixture({ hydrate: (_, options) => {
    if (fail) throw Error('fixture_network_failure');
    options.onConversationLoadedFromNetwork(f.payload);
    assert.equal(options.shouldApplyResponse(), true);
  } });
  try {
    f.snapshot(); await until(() => f.snapshot() === 'unconfirmed');
    for (let i = 0; i < 100; i++) f.snapshot();
    assert.equal(f.calls.length, 1); assert.equal(f.rows().length, 1);
    fail = false; f.windowEvents.emit('online');
    await f.advance(15000); await until(() => f.snapshot() === 'recovered');
    assert.equal(f.calls.length, 2); assert.equal(f.rows().length, 0);
  } finally { f.session.dispose(); }
});

test('storage failure is unavailable rather than a false empty journal', async () => {
  const f = await fixture();
  try {
    f.storage.key = () => { throw Error('fixture_storage_denied'); };
    f.snapshot(); await until(() => f.snapshot() === 'unavailable');
    assert.equal(f.calls.length, 0);
    assert.equal(f.storage.values.size, 1);
  } finally { f.session.dispose(); }
});

test('network flapping and repeated resume cannot bypass the read cooldown', async () => {
  const f = await fixture({ hydrate: () => { throw Error('fixture_offline'); } });
  try {
    f.snapshot(); await until(() => f.snapshot() === 'unconfirmed');
    for (let i = 0; i < 10; i++) {
      f.page.navigator.onLine = false; f.windowEvents.emit('offline'); f.snapshot();
      f.page.navigator.onLine = true; f.windowEvents.emit('online'); f.windowEvents.emit('pageshow');
      f.snapshot(); await tick();
    }
    assert.equal(f.calls.length, 1);
    await f.advance(15000); await until(() => f.calls.length === 2 && f.snapshot() === 'unconfirmed');
  } finally { f.session.dispose(); }
});

test('cold owner gets at most three attempts and no indefinite idle timer', async () => {
  const f = await fixture({ capture: () => { throw Error('context_unavailable'); } });
  try {
    f.snapshot(); await until(() => [...f.timers.values()].some(timer => timer.at === 1000));
    await f.advance(1000); await until(() => f.captures() === 2 && f.timers.size === 1);
    await f.advance(5000); await until(() => f.captures() === 3 && f.timers.size === 0);
    await f.advance(3600000); for (let i = 0; i < 20; i++) f.snapshot();
    assert.equal(f.captures(), 3); assert.equal(f.calls.length, 0); assert.equal(f.snapshot(), 'idle');
  } finally { f.session.dispose(); }
});

for (const [name, change] of [
  ['account', f => { f.stamp('other-account'); f.setIdentity(JSON.stringify(['different-user', 'different-account'])); }],
  ['route', f => { f.page.location.href += '/changed'; }],
  ['document', f => { f.page.document = { visibilityState: 'visible' }; }],
  ['token', f => { f.page.__elonChatGptDocumentToken = 'doc_changed'; }],
  ['writer', f => { f.write(true); f.session.cancel(); }],
  ['background', f => { f.page.document.visibilityState = 'hidden'; f.documentEvents.emit('visibilitychange'); }],
  ['offline', f => { f.page.navigator.onLine = false; f.windowEvents.emit('offline'); }],
  ['disabled', f => { f.enable(false); }],
  ['disposed', f => f.session.dispose()],
]) {
  test(`${name} invalidates a late history callback without clearing the pending record`, async () => {
    let options, finish;
    const f = await fixture({ hydrate: (_, value) => { options = value; return new Promise(done => { finish = done; }); } });
    try {
      f.snapshot(); await until(() => !!options);
      assert.equal(f.snapshot(), 'checking'); change(f);
      options.onConversationLoadedFromNetwork(f.payload);
      assert.equal(options.shouldApplyResponse(), false);
      finish(); await tick(); await tick();
      assert.equal(f.rows().length, 1);
      assert.equal(f.phases.includes('recovered'), false);
    } finally { f.session.dispose(); }
  });
}

test('hanging context load times out and a late result cannot start history reads', async () => {
  let finish;
  const f = await fixture({ capture: () => new Promise(done => { finish = done; }) });
  try {
    f.snapshot(); await until(() => !!finish); await f.advance(15000);
    await until(() => f.snapshot() === 'unavailable');
    finish(f.binding); await tick(); assert.equal(f.calls.length, 0);
    assert.equal(f.rows().length, 1); assert.equal(f.timers.size, 0);
  } finally { f.session.dispose(); }
});

test('temporary chats never touch recovery storage or history', async () => {
  const f = await fixture({ capture: () => null });
  try {
    f.snapshot(); await until(() => f.snapshot() === 'clear');
    assert.equal(f.calls.length, 0); assert.equal(f.rows().length, 1);
  } finally { f.session.dispose(); }
});

test('a suspended recovery resumes in the foreground and disposal removes listeners', async () => {
  const f = await fixture();
  f.page.document.visibilityState = 'hidden'; f.snapshot(); assert.equal(f.captures(), 0);
  f.page.document.visibilityState = 'visible'; f.documentEvents.emit('visibilitychange');
  await until(() => f.snapshot() === 'recovered');
  f.session.dispose(); assert.equal(f.windowEvents.items.size, 0);
  assert.equal(f.documentEvents.items.size, 0); assert.equal(f.timers.size, 0);
});
