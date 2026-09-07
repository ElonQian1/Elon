'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const moduleUnderTest = require('../android/app/src/main/assets/chatgpt_web_private_attachment_selection.js');
const tick = () => new Promise(resolve => setImmediate(resolve));

function fixture(options = {}) {
  const timers = new Map();
  const binding = { token: 'doc_fixture_1', href: 'https://chatgpt.com/', ...options.binding };
  const id = 'selection_' + 'a'.repeat(32);
  const descriptor = { documentToken: binding.token, href: binding.href, selectionId: id };
  let prepared = 0, cancelled = 0, current = true;
  const transport = { prefetch: () => { prepared++; }, dispose: () => { cancelled++; } };
  const root = { AbortController, setTimeout: (fn, ms) => { timers.set(fn, ms); return fn; },
    clearTimeout: fn => timers.delete(fn) };
  const instance = moduleUnderTest.create(root, {
    composer: { available: () => true, capture: () => binding, current: () => current,
      prepare: options.prepare || (async () => true), pickerReservationContext: () => ({ useCase: 'ace_upload' }) },
    busy: () => options.busy === true, createTransport: () => transport,
  });
  return { instance, descriptor, timers, transport, binding, setCurrent: value => { current = value; },
    counts: () => ({ prepared, cancelled }),
    begin: (extra = {}) => instance.begin(JSON.stringify({ ...descriptor, id, kind: 'document', ...extra })),
    expire: ms => { for (const [fn, delay] of [...timers]) if (delay === ms) fn(); } };
}

test('native selection takes exactly once with the original composer binding and transport', async () => {
  const f = fixture(); assert.equal(f.begin(), true); await tick();
  const taken = f.instance.take(f.descriptor);
  assert.equal(taken.binding, f.binding);
  assert.equal(taken.transport, f.transport);
  assert.equal(f.timers.size, 0);
  assert.equal(f.instance.take(f.descriptor), null);
  f.instance.cancel();
  assert.equal(taken.controller.signal.aborted, false, 'ownership has transferred to the sender');
  taken.controller.abort(); taken.transport.dispose();
});

test('selection expiry cancels prewarm and releases its file-independent resources', async () => {
  const f = fixture(); f.begin(); await tick(); f.expire(120000);
  assert.equal(f.instance.take(f.descriptor), null);
  assert.equal(f.timers.size, 0);
  assert.deepEqual(f.counts(), { prepared: 1, cancelled: 1 });
});

test('context lookup deadline prevents a late metadata result from allocating', async () => {
  let release;
  const f = fixture({ prepare: () => new Promise(resolve => { release = resolve; }) });
  f.begin(); f.expire(5000); release(true); await tick();
  assert.equal(f.instance.take(f.descriptor), null);
  assert.equal(f.timers.size, 0);
  assert.equal(f.counts().prepared, 0);
});

test('take never waits for context preparation and its late result cannot create a reservation', async () => {
  let release;
  const f = fixture({ prepare: () => new Promise(resolve => { release = resolve; }) });
  f.begin(); assert.equal(f.instance.take(f.descriptor), null);
  release(true); await tick();
  assert.equal(f.counts().prepared, 0);
  assert.equal(f.timers.size, 0);
});

test('temporary selection transfers its scope unchanged and cancellation releases preparation', async () => {
  const f = fixture({ binding: { isTemporaryChat: true, href: 'https://chatgpt.com/?temporary-chat=true' } });
  assert.equal(f.begin(), true); await tick();
  const selected = f.instance.take(f.descriptor);
  assert.equal(selected.binding.isTemporaryChat, true);
  assert.equal(selected.binding, f.binding);
  selected.controller.abort(); selected.transport.dispose();
  assert.equal(f.timers.size, 0);
});

test('busy, unsupported, stale and project selections cannot start allocation', async () => {
  for (const options of [{ busy: true }, { binding: { projectId: 'project' } }]) {
    const f = fixture(options); assert.equal(f.begin(), false); assert.equal(f.timers.size, 0);
  }
  for (const changed of [{ kind: 'audio' }, { id: '../unsafe' }, { documentToken: 'doc_other_1' }, { href: 'https://chatgpt.com/c/other' }]) {
    const f = fixture(); assert.equal(f.begin(changed), false); assert.equal(f.timers.size, 0);
  }
  const f = fixture(); f.begin(); await tick(); f.setCurrent(false);
  assert.equal(f.instance.take(f.descriptor), null);
  assert.equal(f.counts().cancelled, 1);
});
