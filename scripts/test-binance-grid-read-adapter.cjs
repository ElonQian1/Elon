const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const code = fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets/binance_grid_read_adapter.js'), 'utf8');
const LIST = '/bapi/futures/v2/private/future/grid/query-open-grids';
const DETAIL = '/bapi/futures/v1/private/future/grid/query-grid-detail';
const row = {strategyId: 123, rootUserId: 42, symbol: 'NEARUSDT', strategyStatus: 'WORKING', direction: 'LONG',
  gridType: 'ARITH', gridLowerLimit: '1.2', gridUpperLimit: '2.4', gridCount: 12, initialLeverage: 2,
  gridProfit: '-0.15', createTime: 1788790000000, csrfToken: 'fixture-secret', unknownSecret: 'not-projected'};
const good = data => JSON.stringify({code: '000000', success: true, data});
const tick = () => new Promise(resolve => setImmediate(resolve));
function harness() {
  const events = [], calls = [], queue = [];
  class Xhr { open() {} send() {} addEventListener(name, callback) { this.callback = callback; } }
  const window = {ElonBinanceRead: {postMessage: raw => events.push(JSON.parse(raw))},
    fetch: (url, init) => { calls.push({url, init}); const value = queue.shift(); return Promise.resolve({status: value?.status ?? 200, clone: () => ({text: () => Promise.resolve(value?.text ?? '{}')})}); }};
  window.top = window;
  const context = vm.createContext({window, location: {origin: 'https://www.binance.com', href: 'https://www.binance.com/zh-CN/trading-bots/futures/grid/NEARUSDT'}, URL, XMLHttpRequest: Xhr, WeakMap, Set});
  vm.runInContext(code, context);
  return {window, events, calls, queue, Xhr};
}
(async () => {
  const h = harness(); h.queue.push({text: good([row])});
  await h.window.fetch(LIST, {method: 'POST'}); await tick();
  assert.equal(h.events.length, 0);
  h.window.__elonBinanceReadV1.bind('doc_test_123');
  assert.equal(h.events[0].rows.length, 1);
  assert.equal(h.events[0].rows[0].profit, '-0.15');
  assert.ok(!JSON.stringify(h.events).includes('fixture-secret'));
  assert.ok(!JSON.stringify(h.events).includes('not-projected'));
  assert.equal(h.window.__elonBinanceReadV1.detail('999'), false);
  h.queue.push({text: good(row)});
  assert.equal(h.window.__elonBinanceReadV1.detail('123'), true); await tick();
  assert.equal(h.calls.at(-1).url, DETAIL + '?strategyId=123');
  assert.equal(h.calls.at(-1).init.method, 'GET');
  assert.equal(h.events.at(-1).kind, 'detail');
  h.queue.push({text: good([row, row])}); await h.window.fetch(LIST, {method: 'POST'}); await tick();
  assert.equal(h.events.at(-1).kind, 'unavailable');
  assert.equal(h.window.__elonBinanceReadV1.detail('123'), false);
  const count = h.events.length;
  h.queue.push({text: good([row])}); await h.window.fetch('https://example.com' + LIST, {method: 'POST'}); await tick();
  assert.equal(h.events.length, count);
  const x = new h.Xhr(); x.open('POST', LIST); x.send(); x.status = 200; x.responseText = good([row]); x.callback();
  assert.equal(h.events.at(-1).kind, 'list');
  h.queue.push({text: JSON.stringify({code: '000000', success: false, data: [row]})});
  await h.window.fetch(LIST, {method: 'POST'}); await tick(); assert.equal(h.events.at(-1).kind, 'unavailable');
  console.log('Binance read adapter: 17 assertions passed (synthetic only; no network or orders).');
})().catch(error => { console.error(error); process.exitCode = 1; });
