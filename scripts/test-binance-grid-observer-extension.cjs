'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const { randomUUID } = require('node:crypto');
const folder = path.resolve(__dirname, '../tools/binance-grid-observer');
const store = require(path.join(folder, 'store.js'));
const sanitizer = require(path.join(folder, 'sanitize.js'));
const origin = 'https://www.binance.com';
const sample = () => ({ schema_version: 'binance-grid-observation.v1', method: 'POST',
  path: '/bapi/futures/v1/private/strategy/grid/list', status: 200,
  requestShape: { type: 'object', fields: { page: { type: 'number' } }, unknownFields: false },
  responseShape: { type: 'object', fields: { data: { type: 'array', items: [] } }, unknownFields: false }, outcome: 'json' });
const sender = () => ({ id: 'extension', tab: { id: 7 }, frameId: 0, documentId: 'doc-a',
  origin, url: origin + '/zh-CN/trading-bots/futures/grid/EXAMPLEUSDT' });
const capture = () => store.create(7, 'doc-a', 'a'.repeat(32), 1000);
const envelope = () => ({ sessionId: 'a'.repeat(32), observation: sample() });

test('manifest grants only temporary observation permissions and no remote scripts', () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(folder, 'manifest.json'), 'utf8'));
  assert.equal(manifest.manifest_version, 3);
  assert.deepEqual(manifest.permissions.slice().sort(), ['activeTab', 'scripting', 'storage']);
  assert.equal(manifest.host_permissions, undefined);
  assert.equal(manifest.externally_connectable, undefined);
  assert.equal(manifest.content_scripts, undefined);
  assert.equal(manifest.minimum_chrome_version, '106');
});
test('samples are bound to origin, top frame, tab, document and generation', () => {
  for (const change of [{ origin: 'https://evil.invalid' }, { frameId: 1 },
    { tab: { id: 8 } }, { documentId: 'doc-b' }]) {
    const state = capture();
    assert.equal(store.accept(state, { ...sender(), ...change }, envelope(), sanitizer.sanitizeObservation, 2000), false);
    assert.equal(state.records.length, 0);
  }
  assert.equal(store.accept(capture(), sender(), { ...envelope(), sessionId: 'b'.repeat(32) }, sanitizer.sanitizeObservation, 2000), false);
});
test('same sanitized contract deduplicates, adds counts, and omits runtime identifiers from report', () => {
  const state = capture();
  assert.equal(store.accept(state, sender(), envelope(), sanitizer.sanitizeObservation, 2000), true);
  assert.equal(store.accept(state, sender(), envelope(), sanitizer.sanitizeObservation, 3000), true);
  assert.equal(state.records.length, 1);
  assert.equal(state.records[0].count, 2);
  const report = store.report(state, 4000);
  assert.equal(report.provenance, 'untrusted_page_observation');
  assert.equal(report.coverage.contractVerified, false);
  assert.equal(report.coverage.tradingEnabled, false);
  assert.doesNotMatch(JSON.stringify(report), /sessionId|documentId|tabId|EXAMPLEUSDT/);
});
test('stop, expiry and replacement reject in-flight old observations', () => {
  const state = capture(); store.stop(state);
  assert.equal(store.accept(state, sender(), envelope(), sanitizer.sanitizeObservation, 2000), false);
  const expired = capture();
  assert.equal(store.accept(expired, sender(), envelope(), sanitizer.sanitizeObservation, expired.expiresAt), false);
  assert.equal(expired.reason, 'expired');
  const replaced = store.create(7, 'doc-a', 'b'.repeat(32), 2000);
  assert.equal(store.accept(replaced, sender(), envelope(), sanitizer.sanitizeObservation, 3000), false);
});
test('oversize and hostile observations cannot enter persistent report', () => {
  const state = capture();
  const bad = { ...envelope(), observation: { ...sample(), token: 'SECRET_VALUE_NEVER_EXPORT' } };
  store.accept(state, sender(), bad, sanitizer.sanitizeObservation, 2000);
  assert.doesNotMatch(JSON.stringify(state), /SECRET_VALUE_NEVER_EXPORT|token/);
  assert.equal(store.accept(state, sender(), { ...envelope(), observation: { x: 'x'.repeat(17000) } }, sanitizer.sanitizeObservation, 2100), false);
  assert.equal(store.accept(state, sender(), { ...envelope(), observation: { responseShape: { type: 'string', value: 'secret' } } }, sanitizer.sanitizeObservation, 2200), false);
});
test('rate and sample caps keep page event floods bounded', () => {
  const state = capture();
  for (let i = 0; i < 180; i++) store.accept(state, sender(), envelope(), sanitizer.sanitizeObservation, 2000);
  assert.equal(state.records.length, 1);
  assert.equal(state.observations, 120);
  assert.equal(state.dropped, 60);
  for (let i = 0; i < 150; i++) {
    const message = envelope(); message.observation.status = 200 + i;
    store.accept(state, sender(), message, sanitizer.sanitizeObservation, i < 120 ? 62000 : 122000);
  }
  assert.equal(state.records.length, 128);
  assert.ok(JSON.stringify(state.records).length <= store.LIMITS.bytes);
});
test('worker restart revalidates session data without reviving expired state', () => {
  const state = capture(); store.accept(state, sender(), envelope(), sanitizer.sanitizeObservation, 2000);
  const restored = store.restore(state, sanitizer.sanitizeObservation, 3000);
  assert.equal(restored.records.length, 1);
  assert.equal(store.restore({ ...state, expiresAt: 99999999 }, sanitizer.sanitizeObservation, 3000), null);
  const expired = store.restore(state, sanitizer.sanitizeObservation, state.expiresAt);
  assert.equal(expired.active, false); assert.equal(expired.reason, 'expired');
});

function workerHarness({ failMain = false } = {}) {
  let messageListener, updatedListener, removedListener;
  let activeUrl = sender().url;
  let documentId = 'doc-a';
  const saved = {};
  const injections = [];
  const context = vm.createContext({ URL, Date, TextEncoder, console, crypto: { randomUUID },
    chrome: {
      runtime: { id: 'extension', getURL: (file) => 'chrome-extension://extension/' + file,
        onMessage: { addListener: (listener) => { messageListener = listener; } } },
      storage: { session: { get: async () => structuredClone(saved),
        set: async (data) => Object.assign(saved, structuredClone(data)),
        remove: async (key) => { delete saved[key]; } } },
      tabs: { query: async () => [{ id: 7, url: activeUrl }],
        onUpdated: { addListener: (listener) => { updatedListener = listener; } },
        onRemoved: { addListener: (listener) => { removedListener = listener; } } },
      scripting: { executeScript: async (injection) => {
        injections.push(injection);
        if (failMain && injection.files?.includes('observer.js')) throw new Error('raw secret error');
        return [{ documentId, frameId: 0, result: { active: true } }];
      } }
    } });
  context.importScripts = (...files) => files.forEach((file) => vm.runInContext(fs.readFileSync(path.join(folder, file), 'utf8'), context));
  vm.runInContext(fs.readFileSync(path.join(folder, 'worker.js'), 'utf8'), context);
  const send = (message, from) => new Promise((resolve) => {
    const keep = messageListener(message, from, (response) => resolve(structuredClone(response)));
    if (!keep) resolve(null);
  });
  return { saved, injections, send, setUrl: (url) => { activeUrl = url; },
    setDocument: (value) => { documentId = value; },
    updated: (...args) => updatedListener(...args), removed: (...args) => removedListener(...args),
    ui: (action) => send({ type: 'observer-ui', action }, { id: 'extension', url: 'chrome-extension://extension/popup.html' }) };
}
test('worker rejects page impersonation of popup and prevents off-origin injection', async () => {
  const h = workerHarness();
  assert.equal(await h.send({ type: 'observer-ui', action: 'start' }, sender()), null);
  h.setUrl('https://evil.invalid/futures/grid');
  assert.deepEqual(await h.ui('start'), { ok: false, error: 'wrong_page' });
  assert.equal(h.injections.length, 0);
});
test('start pins all subsequent injections; stop remains effective after late sample', async () => {
  const h = workerHarness();
  assert.equal((await h.ui('start')).state.active, true);
  for (const injection of h.injections.slice(1)) assert.deepEqual(Array.from(injection.target.documentIds), ['doc-a']);
  const state = Object.values(h.saved)[0];
  const message = { type: 'observer-sample', sessionId: state.sessionId, observation: sample() };
  assert.equal((await h.send(message, sender())).ok, true);
  assert.equal((await h.ui('export')).report.records.length, 1);
  assert.equal((await h.ui('stop')).state.active, false);
  assert.equal((await h.send(message, sender())).ok, false);
  assert.equal((await h.ui('clear')).state.records, 0);
  assert.equal((await h.ui('export')).error, 'no_samples');
});
test('injection failure is rolled back with fixed error and no raw details', async () => {
  const h = workerHarness({ failMain: true });
  assert.deepEqual(await h.ui('start'), { ok: false, error: 'operation_failed' });
  assert.equal(Object.values(h.saved)[0].active, false);
  assert.doesNotMatch(JSON.stringify(h.saved), /raw secret/);
});
test('navigation and worker events stop capture without closing or reloading any page', async () => {
  const h = workerHarness(); await h.ui('start');
  h.updated(7, { status: 'loading' });
  assert.equal((await h.ui('status')).state.active, false);
  await h.ui('start'); h.removed(7);
  assert.equal((await h.ui('status')).state.reason, 'navigated');
});
test('a new document in the same tab cannot export the previous capture', async () => {
  const h = workerHarness(); await h.ui('start');
  const state = Object.values(h.saved)[0];
  await h.send({ type: 'observer-sample', sessionId: state.sessionId, observation: sample() }, sender());
  h.setDocument('doc-b');
  assert.deepEqual(await h.ui('export'), { ok: false, error: 'not_started' });
  assert.equal(Object.values(h.saved)[0].active, false);
});

function bridgeHarness() {
  const events = {};
  const forwarded = [];
  const context = vm.createContext({ URL, Date, TextEncoder, location: {
    origin, pathname: '/zh-CN/trading-bots/futures/grid/EXAMPLEUSDT' },
  chrome: { runtime: { sendMessage: async (message) => { forwarded.push(structuredClone(message)); } } } });
  context.window = context; context.top = context;
  context.addEventListener = (name, callback) => { events[name] = callback; };
  for (const file of ['sanitize.js', 'bridge.js']) vm.runInContext(fs.readFileSync(path.join(folder, file), 'utf8'), context);
  const message = (changes = {}) => {
    context.eventOverrides = changes;
    context.sampleInput = sample();
    const event = vm.runInContext(`({ source: window, origin: location.origin,
      data: { channel: 'binance-grid-observer-v1', sessionId: '${'a'.repeat(32)}', observation: sampleInput }, ...eventOverrides })`, context);
    events.message(event);
  };
  return { context, forwarded, message, start: () => context.__binanceGridBridgeV1.start('a'.repeat(32)) };
}
test('isolated bridge accepts only matching window origin and validated observations', () => {
  const h = bridgeHarness(); assert.equal(h.start().active, true);
  h.message(); assert.equal(h.forwarded.length, 1);
  h.message({ origin: 'https://evil.invalid' });
  h.message({ source: {} });
  h.message({ data: { channel: 'binance-grid-observer-v1', sessionId: 'a'.repeat(32), observation: { secret: 'PRIVATE_VALUE_NEVER_EXPORT' } } });
  assert.equal(h.forwarded.length, 1);
  assert.doesNotMatch(JSON.stringify(h.forwarded), /PRIVATE_VALUE_NEVER_EXPORT/);
});
test('isolated bridge blocks late events after stop or page-path change', () => {
  const h = bridgeHarness(); h.start(); h.context.__binanceGridBridgeV1.stop(); h.message();
  assert.equal(h.forwarded.length, 0);
  h.start(); h.context.location.pathname = '/zh-CN/login'; h.message();
  assert.equal(h.forwarded.length, 0);
  assert.equal(h.start().active, false);
});
