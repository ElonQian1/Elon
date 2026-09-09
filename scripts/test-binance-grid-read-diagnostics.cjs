const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const asset = name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets', name), 'utf8');
function harness() {
  const events = [], requests = [], callbacks = {};
  const doc = {readyState:'complete', scripts:[{},{}], body:{textContent:'private-account-content-canary'},
    querySelectorAll:() => [{textContent:'运行中 (25)',getClientRects:() => [1]}]};
  const window = {addEventListener:(name, fn) => {callbacks[name] = fn;},
    ElonBinanceRead:{postMessage:raw => events.push(JSON.parse(raw))},
    fetch:(url, init) => {
      requests.push({url, init});
      const data = url.includes('get-user-base-info') ? {userId:'42',subUser:false,parentUser:true} :
        [{strategyId:'123',rootUserId:'43',symbol:'NEARUSDT',strategyStatus:'WORKING'}];
      return Promise.resolve({status:200,clone:() => ({text:async () => JSON.stringify({code:'000000',success:true,data})})});
    }};
  window.top = window;
  class Xhr {open() {} send() {} setRequestHeader() {} addEventListener() {}}
  const context = vm.createContext({window,document:doc,location:{origin:'https://www.binance.com',
    href:'https://www.binance.com/zh-CN/trading-bots/futures/grid/NEARUSDT',pathname:'/zh-CN/trading-bots/futures/grid/NEARUSDT'},
    URL,Headers,XMLHttpRequest:Xhr,AbortController,setTimeout,clearTimeout});
  vm.runInContext(asset('binance_grid_read_diagnostics.js'), context);
  return {window, doc, events, requests, callbacks, context, inspect:() => {
    window.__elonBinanceDiagnosticsV1.inspect('doc_test_123'); return events.at(-1);
  }};
}
test('inspection returns bounded facts without requests or private page text', () => {
  const h = harness(), d = h.inspect();
  assert.equal(d.route,'grid'); assert.equal(d.running_control,true); assert.equal(d.login_control,false);
  assert.equal(d.script_count,2); assert.equal(d.list_requests,0); assert.equal(h.requests.length,0);
  assert.ok(!JSON.stringify(d).includes('private-account-content-canary'));
  assert.ok(!JSON.stringify(d).includes('NEARUSDT')); assert.ok(!JSON.stringify(d).includes('binance.com'));
  const count = h.events.length;
  assert.equal(h.window.__elonBinanceDiagnosticsV1.inspect('bad-token'),false); assert.equal(h.events.length,count);
});
test('unknown exceptions and website errors cannot export messages or credentials', () => {
  const h = harness(), diagnostics = h.window.__elonBinanceDiagnosticsV1;
  diagnostics.failure('private-kind-canary','private-error-canary');
  h.callbacks.error({target:{tagName:'SCRIPT'},message:'secret-url-canary'});
  h.callbacks.error({target:{},message:'private-script-canary'});
  const d = h.inspect();
  assert.equal(d.last_failure,'transport_or_parse_failed'); assert.equal(d.failure_kind,'none');
  assert.equal(d.script_load_errors,1); assert.equal(d.script_errors,1);
  assert.ok(!JSON.stringify(d).includes('canary'));
});
test('real adapter callbacks distinguish an account mismatch without exposing rows', async () => {
  const h = harness(); vm.runInContext(asset('binance_grid_read_adapter.js'),h.context);
  h.window.__elonBinanceReadV1.bind('doc_test_123');
  await h.window.fetch('/bapi/futures/v2/private/future/grid/query-open-grids',{method:'POST'});
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.events.at(-1).kind,'unavailable');
  const d = h.inspect();
  assert.equal(d.list_requests,1); assert.equal(d.list_responses,1);
  assert.equal(d.identity_requests,1); assert.equal(d.identity_responses,1);
  assert.equal(d.last_failure,'account_mismatch'); assert.equal(d.failure_kind,'list');
  assert.equal(d.last_http_status,200); assert.equal(h.requests.length,2);
  assert.ok(!Object.hasOwn(d,'rows')); assert.ok(!Object.hasOwn(d,'account'));
});
test('counts are capped and diagnostic reinstallation is idempotent', () => {
  const h = harness(), d = h.window.__elonBinanceDiagnosticsV1;
  for(let i=0;i<10020;i++) d.request('list');
  d.request('__proto__'); d.response('detail',900);
  vm.runInContext(asset('binance_grid_read_diagnostics.js'),h.context);
  assert.equal(h.window.__elonBinanceDiagnosticsV1,d);
  assert.equal(h.inspect().list_requests,10000); assert.equal(h.inspect().last_http_status,0);
});

test('report diagnostics retain fixed failure facts but discard arbitrary values',()=>{
  const h=harness(), d=h.window.__elonBinanceDiagnosticsV1;
  d.report({kind:'orders',stage:'parse_windowOrders',outcome:'failed',error:'unsupported_field',http:200,business:'000000'});
  assert.equal(h.inspect().report.stage,'parse_windowOrders');
  d.report({kind:'private-canary',stage:'https://private-canary',outcome:'private-canary',error:'cookie=private-canary',http:999,business:'private-canary'});
  const r=h.inspect().report;assert.equal(r.kind,'none');assert.equal(r.http,0);assert.equal(r.error,'transport_or_parse_failed');
  assert.ok(!JSON.stringify(r).includes('canary'));
});
test('legacy list discovery is visible but does not turn into verified v2 rows', async () => {
  const h = harness(); vm.runInContext(asset('binance_grid_read_adapter.js'),h.context);
  h.window.__elonBinanceReadV1.bind('doc_test_123');
  await h.window.fetch('/bapi/futures/v1/private/future/grid/query-open-grids',{method:'GET'});
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(h.events.length,0); assert.equal(h.requests.length,1);
  assert.equal(h.inspect().legacy_list_requests,1); assert.equal(h.inspect().list_requests,0);
});
