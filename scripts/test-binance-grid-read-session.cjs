const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const code = fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets/binance_grid_read_adapter.js'), 'utf8');
const LIST = '/bapi/futures/v2/private/future/grid/query-open-grids';
const DETAIL = '/bapi/futures/v1/private/future/grid/query-grid-detail';
const INFO = '/bapi/accounts/v1/private/account/get-user-base-info';
const tick = () => new Promise(resolve => setImmediate(resolve));
const row = (uid = '42', profit = '1.2') => ({strategyId: '123', rootUserId: uid, symbol: 'NEARUSDT',
  strategyStatus: 'WORKING', direction: 'LONG', gridProfit: profit});
function response(data, success = true) {
  return {status: 200, clone: () => ({text: async () => JSON.stringify({code: '000000', success, data})})};
}
function delayed() {
  let resolve, reject;
  const promise = new Promise((a, b) => { resolve = a; reject = b; });
  return {promise, resolve, reject};
}
function harness(bind = true) {
  const events = [], calls = [], queue = [], identities = [];
  const account = {userId: '42', subUser: false, parentUser: true, email: 'fixture-private@example.test'};
  class Xhr { open() {} send() {} addEventListener(_, callback) { this.callback = callback; } }
  const window = {ElonBinanceRead: {postMessage: raw => events.push(JSON.parse(raw))}, fetch: (url, init) => {
    calls.push({url, init});
    return Promise.resolve(url === INFO ? (identities.shift() || response(account)) : queue.shift());
  }};
  window.top = window;
  vm.runInNewContext(code, {window, location: {origin: 'https://www.binance.com', href: 'https://www.binance.com/'}, URL, XMLHttpRequest: Xhr, AbortController, setTimeout, clearTimeout});
  if (bind) window.__elonBinanceReadV1.bind('doc_test_123');
  const list = async data => { queue.push(response(data)); await window.fetch(LIST, {method: 'POST'}); await tick(); };
  return {window, events, calls, queue, identities, account, list, Xhr};
}
test('empty list uses authenticated subaccount proof and emits no unrelated identity fields', async () => {
  const h = harness(); Object.assign(h.account, {userId: '43', subUser: true, parentUser: false});
  await h.list([]);
  assert.equal(h.events.at(-1).kind, 'list'); assert.equal(h.events.at(-1).account, '43');
  assert.equal(h.events.at(-1).account_kind, 'sub'); assert.equal(h.events.at(-1).rows.length, 0);
  assert.ok(!JSON.stringify(h.events).includes('fixture-private'));
});
test('parent-owned rows cannot be bound to a child UID', async () => {
  const h = harness(); Object.assign(h.account, {userId: '43', subUser: true, parentUser: false});
  await h.list([row('42')]); assert.equal(h.events.at(-1).kind, 'unavailable');
  assert.equal(h.window.__elonBinanceReadV1.detail('123'), false);
});
test('a late failed list cannot clear a newer successful response', async () => {
  const h = harness(), pending = delayed(); h.queue.push(pending.promise);
  const old = h.window.fetch(LIST, {method: 'POST'}).catch(() => {});
  await h.list([row()]); const before = h.events.length;
  pending.reject(new Error('synthetic offline')); await old; await tick();
  assert.equal(h.events.length, before); assert.equal(h.events.at(-1).kind, 'list');
});
test('old detail completion cannot replace a refreshed list with the same strategy ID', async () => {
  const h = harness(); await h.list([row()]);
  const pending = delayed(); h.queue.push(pending.promise); h.window.__elonBinanceReadV1.detail('123');
  await h.list([row('42', '2.5')]); const before = h.events.length;
  pending.resolve(response(row('42', '0.1'))); await tick();
  assert.equal(h.events.length, before); assert.equal(h.events.at(-1).rows[0].profit, '2.5');
});
test('late failed detail does not revoke a new list', async () => {
  const h = harness(); await h.list([row()]);
  const pending = delayed(); h.queue.push(pending.promise); h.window.__elonBinanceReadV1.detail('123');
  await h.list([row()]); const before = h.events.length;
  pending.reject(new Error('synthetic offline')); await tick(); assert.equal(h.events.length, before);
});
test('detail without row UID requires the current authenticated account proof', async () => {
  const h = harness(); await h.list([row()]);
  h.queue.push(response({...row(), rootUserId: null})); h.window.__elonBinanceReadV1.detail('123'); await tick();
  assert.equal(h.events.at(-1).kind, 'detail'); assert.equal(h.events.at(-1).account, '42');
  Object.assign(h.account, {userId: '43', subUser: true, parentUser: false});
  h.queue.push(response({...row(), rootUserId: null})); h.window.__elonBinanceReadV1.detail('123'); await tick();
  assert.equal(h.events.at(-1).kind, 'identity'); assert.equal(h.events.at(-1).account, '43');
  assert.equal(h.window.__elonBinanceReadV1.detail('123'), false);
});
test('website identity failure clears known grids without needing a new grid response', async () => {
  const h = harness(); await h.list([row()]);
  h.identities.push(response(null, false)); await h.window.fetch(INFO, {method: 'GET'}); await tick();
  assert.equal(h.events.at(-1).kind, 'unavailable'); assert.equal(h.window.__elonBinanceReadV1.detail('123'), false);
});
test('identity verification failure cannot publish an empty list as a logged-in account', async () => {
  const h = harness(); h.identities.push(response(null, false)); await h.list([]);
  assert.equal(h.events.at(-1).kind, 'unavailable'); assert.equal(h.events.some(x => x.kind === 'list'), false);
});
test('older account response cannot undo a newer identity switch', async () => {
  const h = harness(); await h.list([row()]);
  const pending = delayed(); h.identities.push(pending.promise); const old = h.window.fetch(INFO);
  Object.assign(h.account, {userId: '43', subUser: true, parentUser: false});
  await h.window.fetch(INFO); await tick(); const before = h.events.length;
  pending.resolve(response({userId: '42', parentUser: true, subUser: false})); await old; await tick();
  assert.equal(h.events.length, before); assert.equal(h.events.at(-1).account, '43');
});
test('only a known detail and exact read-only identity GET can be issued by the adapter', async () => {
  const h = harness(); await h.list([row()]); const before = h.calls.length;
  assert.equal(h.window.__elonBinanceReadV1.detail('999'), false); assert.equal(h.calls.length, before);
  h.queue.push(response(row())); h.window.__elonBinanceReadV1.detail('123'); await tick();
  for (const call of h.calls.filter(c => c.url !== LIST)) {
    assert.ok(call.url === INFO || call.url === DETAIL + '?strategyId=123');
    assert.equal(call.init.method, 'GET'); assert.equal(call.init.credentials, 'same-origin');
  }
});
test('same-account identity response before native binding must preserve the pending list', async () => {
  const h = harness(false); await h.list([row()]);
  await h.window.fetch(INFO); await tick(); assert.equal(h.events.length,0);
  h.window.__elonBinanceReadV1.bind('doc_test_123');
  assert.equal(h.events.length,1); assert.equal(h.events[0].kind,'list');
  assert.equal(h.events[0].rows.length,1);
});
test('account switch before native binding must discard the pending old-account list', async () => {
  const h = harness(false); await h.list([row()]);
  Object.assign(h.account,{userId:'43',subUser:true,parentUser:false});
  await h.window.fetch(INFO); await tick(); h.window.__elonBinanceReadV1.bind('doc_test_123');
  assert.equal(h.events.length,1); assert.equal(h.events[0].kind,'identity');
  assert.equal(h.events[0].account,'43'); assert.equal(h.window.__elonBinanceReadV1.detail('123'),false);
});
