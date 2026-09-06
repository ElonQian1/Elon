'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const folder = path.resolve(__dirname, '../tools/binance-grid-observer');
const source = fs.readFileSync(path.join(folder, 'popup.js'), 'utf8');
const html = fs.readFileSync(path.join(folder, 'popup.html'), 'utf8');
const store = require(path.join(folder, 'store.js'));
const sanitizer = require(path.join(folder, 'sanitize.js'));
const tick = () => new Promise(setImmediate);
const now = 1_788_750_000_000;

function reportFixture() {
  const capture = store.create(7, 'document-a', 's'.repeat(32), now);
  const observation = {
    schema_version: 'binance-grid-observation.v1', method: 'GET',
    path: '/bapi/futures/v1/private/strategy/grid/list', status: 200,
    requestShape: { type: 'null' }, responseShape: { type: 'object', fields: {}, unknownFields: false }, outcome: 'json',
  };
  assert.equal(store.accept(capture, { tab: { id: 7 }, frameId: 0, documentId: 'document-a', origin: 'https://www.binance.com' },
    { sessionId: 's'.repeat(32), observation }, sanitizer.sanitizeObservation, now + 1000), true);
  store.stop(capture);
  return store.report(capture, now + 2000);
}

function harness(report = reportFixture()) {
  const nodes = new Map();
  for (const match of html.matchAll(/id="([^"]+)"/g)) {
    nodes.set(match[1], { disabled: false, textContent: '', hidden: false, dataset: {}, events: {},
      setAttribute() {}, addEventListener(name, listener) { this.events[name] = listener; } });
  }
  const timers = new Map(), windowEvents = new Map(), blobs = [], revoked = [], links = [], messages = [];
  let timerId = 0, downloads = 0, intervalCleared = false;
  let responder = async ({ action }) => ({ ok: true, state: report.status, ...(action === 'export' ? { report } : {}) });
  const context = vm.createContext({
    chrome: { runtime: { sendMessage: (message) => { messages.push(structuredClone(message)); return responder(message); } } },
    document: {
      getElementById: (id) => nodes.get(id), querySelector: () => ({ setAttribute() {} }), body: { appendChild() {} },
      createElement: () => { const link = { click() { downloads++; }, remove() {} }; links.push(link); return link; },
    },
    window: { addEventListener: (name, listener) => windowEvents.set(name, listener) },
    TextEncoder, Blob,
    URL: { createObjectURL: (blob) => { blobs.push(blob); return 'blob:test-' + blobs.length; }, revokeObjectURL: (url) => revoked.push(url) },
    setInterval: () => 1, clearInterval: () => { intervalCleared = true; },
    setTimeout: (callback, delay) => { const id = ++timerId; timers.set(id, { callback, delay }); return id; },
    clearTimeout: (id) => timers.delete(id),
  });
  vm.runInContext(source, context, { filename: path.join(folder, 'popup.js') });
  return {
    nodes, timers, blobs, revoked, links, messages,
    setResponder: (value) => { responder = value; }, click: (id) => nodes.get(id).events.click(),
    close: () => windowEvents.get('pagehide')(), get downloads() { return downloads; },
    get intervalCleared() { return intervalCleared; },
  };
}

test('popup uses local external scripts and explicit observation-only controls', async () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(folder, 'manifest.json'), 'utf8'));
  assert.equal(manifest.action.default_popup, 'popup.html');
  assert.doesNotMatch(html, /<script(?![^>]*\bsrc=)|\bonclick=|\bstyle=|https?:\/\//);
  assert.doesNotMatch(source, /innerHTML|outerHTML|eval\(|fetch\(|XMLHttpRequest|localStorage/);
  const h = harness(); await tick();
  for (const id of ['start', 'stop', 'clear', 'export', 'refresh']) assert.equal(typeof h.nodes.get(id).events.click, 'function');
  assert.deepEqual(h.messages, [{ type: 'observer-ui', action: 'status' }]);
  assert.match(html, /零样本不代表没有网格/);
  assert.match(html, /15 分钟后自动停止/);
});

test('popup consumes the actual store report and revokes its local download URL', async () => {
  const report = reportFixture(), h = harness(report); await tick();
  assert.equal(h.nodes.get('record-count').textContent, String(report.status.records));
  assert.equal(h.nodes.get('stop').disabled, true);
  h.click('export'); await tick();
  assert.equal(h.downloads, 1);
  assert.equal(await h.blobs[0].text(), JSON.stringify(report));
  assert.match(h.links[0].download, /^binance-grid-observation-\d{8}-\d{6}\.json$/);
  for (const timer of h.timers.values()) if (timer.delay === 1000) timer.callback();
  assert.deepEqual(h.revoked, ['blob:test-1']);
});

test('compact report envelope accepts above 1 MiB and rejects above 1.1 MiB', async () => {
  // Byte-budget probes intentionally inflate a record; semantic sanitization belongs to store tests.
  const accepted = reportFixture(); accepted.records[0].budgetProbe = 'x'.repeat(1_048_000);
  const acceptedSize = Buffer.byteLength(JSON.stringify(accepted));
  assert.ok(acceptedSize > 1_048_576 && acceptedSize < 1_153_434);
  const good = harness(accepted); await tick(); good.click('export'); await tick();
  assert.equal(good.downloads, 1);
  assert.equal(good.blobs[0].size, acceptedSize);
  const rejected = reportFixture(); rejected.records[0].budgetProbe = 'x'.repeat(1_153_434);
  const bad = harness(rejected); await tick(); bad.click('export'); await tick();
  assert.equal(bad.downloads, 0);
  assert.equal(bad.blobs.length, 0);
  assert.equal(bad.nodes.get('feedback').dataset.kind, 'error');
});

test('unknown report versions or upgraded trust and trading claims cannot download', async () => {
  for (const mutate of [
    (report) => { report.schema = 'unknown'; },
    (report) => { report.provenance = 'trusted'; },
    (report) => { report.coverage.tradingEnabled = true; },
    (report) => { report.coverage.requestValuesIncluded = true; },
  ]) {
    const report = reportFixture(); mutate(report);
    const h = harness(report); await tick(); h.click('export'); await tick();
    assert.equal(h.downloads, 0);
    assert.equal(h.nodes.get('feedback').dataset.kind, 'error');
  }
});

test('errors stay fixed Chinese text and failed status discards old counts', async () => {
  const h = harness(); await tick();
  for (const error of ['PRIVATE_CANARY', 'constructor', '__proto__']) {
    h.setResponder(async () => ({ ok: false, error }));
    h.click('refresh'); await tick();
    assert.doesNotMatch(h.nodes.get('feedback').textContent, /PRIVATE_CANARY|constructor|function|\[object/);
    assert.equal(h.nodes.get('record-count').textContent, '—');
    assert.equal(h.nodes.get('start').disabled, true);
    assert.equal(h.nodes.get('refresh').disabled, false);
  }
});

test('all controls are gated while waiting and active capture cannot start twice', async () => {
  const h = harness(); await tick(); let resolve;
  h.setResponder(() => new Promise((done) => { resolve = done; }));
  h.click('start');
  for (const id of ['start', 'stop', 'clear', 'export', 'refresh']) assert.equal(h.nodes.get(id).disabled, true);
  h.click('start');
  assert.equal(h.messages.filter((message) => message.action === 'start').length, 1);
  resolve({ ok: true, state: store.status(store.create(7, 'document-a', 's'.repeat(32), now), now + 1000) });
  await tick();
  assert.equal(h.nodes.get('start').disabled, true);
  assert.equal(h.nodes.get('stop').disabled, false);
});

test('closing popup cancels UI timers, revokes URLs and ignores a late export', async () => {
  const h = harness(); await tick(); h.click('export'); await tick();
  h.close();
  assert.equal(h.intervalCleared, true);
  assert.deepEqual(h.revoked, ['blob:test-1']);
  const late = harness(); await tick(); let resolve;
  late.setResponder(() => new Promise((done) => { resolve = done; }));
  late.click('export'); late.close();
  resolve({ ok: true, state: reportFixture().status, report: reportFixture() }); await tick();
  assert.equal(late.downloads, 0);
  assert.equal(late.blobs.length, 0);
});
