const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const { webcrypto, createHash } = require('node:crypto');
const code = fs.readFileSync(require('node:path').join(__dirname, '../android/app/src/main/assets/binance_grid_create_adapter.js'), 'utf8');
const fundsCode = fs.readFileSync(require('node:path').join(__dirname, '../android/app/src/main/assets/binance_grid_create_funds.js'), 'utf8');
const MODE='/bapi/futures/v1/private/future/portfolio/margin/get-user-basic';
const FUNDS='/bapi/futures/v2/private/future/user-data/getMaxWithdrawAmount';
const SPOT='/bapi/asset/v3/private/asset-service/asset/get-user-asset';
const LIST = '/bapi/futures/v2/private/future/grid/query-open-grids';
const CREATE = '/bapi/futures/v2/private/future/grid/place-grid';
const INFO = '/bapi/accounts/v1/private/account/get-user-base-info';
const COEF = '/bapi/futures/v1/public/future/common/grid/coef';
const DETAIL = '/bapi/futures/v1/private/future/grid/query-grid-detail';
const hash = createHash('sha256').update('42').digest('hex');
const tick = async () => { for (let i=0; i<10; i++) await new Promise(r=>setImmediate(r)); };
const payload = () => ({symbol:'NEARUSDT', direction:'LONG', marginType:'ISOLATED', gridType:'ARITH',
  gridLowerLimit:'1',gridUpperLimit:'2',gridInitialValue:'37.035',leverage:3,gridCount:10,
  cos:true,cps:true,autoInitPos:true,orderCurrency:'BASE'});
function harness() {
  const calls=[], events=[], account={userId:'42',subUser:false,parentUser:true};
  const behavior={windowCount:169,fail:'',clientMismatch:false, pending:null};
  class Xhr {open(){} send(){} setRequestHeader(){} }
  const response = (data, success=true, code='000000', status=200) => ({status,clone:()=>({text:async()=>JSON.stringify({data,success,code})})});
  const crypto={getRandomValues:value=>webcrypto.getRandomValues(value),subtle:{
    // Same digest bytes, without unrelated OS thread-pool timing in this state-machine test.
    digest:async(_,data)=>createHash('sha256').update(Buffer.from(data)).digest()}};
  const window={ElonBinanceCreate:{postMessage:raw=>events.push(JSON.parse(raw))},crypto,
    fetch:async(url, init={})=>{
      calls.push({url,init});
      if(url===INFO) return response(account);
      if(url===MODE)return response({enable:behavior.portfolio===true});
      if(url===FUNDS)return response('12.345678901234');
      if(url===SPOT)return response([{asset:'USDT',free:'7.8900'}]);
      if(url===COEF) {
        if(behavior.switchDuringConfig) {account.userId='43';account.subUser=true;account.parentUser=false;}
        return response({windowCount:behavior.windowCount});
      }
      if(url===CREATE) {
        if(behavior.pending) return behavior.pending;
        if(behavior.fail==='network') throw Error('SENSITIVE_CANARY');
        if(behavior.fail==='http') return response({},false,'999',503);
        if(behavior.fail==='business') return response({},false,'symbolInTwap');
        const body=JSON.parse(init.body);
        return response({strategyId:'123',clientStrategyId:behavior.clientMismatch?'wrong':body.clientStrategyId,strategyStatus:'NEW'});
      }
      if(url.startsWith(DETAIL)) return response({strategyId:'123',rootUserId:'42',strategyStatus:'WORKING'});
      return response([]);
    }};
  window.top=window;
  vm.runInNewContext(fundsCode,{window,Date});
  vm.runInNewContext(code,{window,location:{origin:'https://www.binance.com',href:'https://www.binance.com/'},
    URL,Headers,XMLHttpRequest:Xhr,AbortController,TextEncoder,setTimeout,clearTimeout,Date});
  const api=window.__elonBinanceCreateV1;
  const observe=()=>window.fetch(LIST,{method:'POST',headers:{'x-fixture-auth':'SENSITIVE_CANARY'}});
  const prepare=async(body=payload())=>{await observe(); assert.equal(api.prepare('doc_test_123','a'.repeat(32),hash,body),true); await tick();};
  return {window,api,observe,prepare,calls,events,account,behavior,Xhr};
}
test('funds uses the captured ordinary and portfolio read requests without trade bodies',async()=>{
  for(const portfolio of [false,true]) {
    const h=harness();h.behavior.portfolio=portfolio;await h.observe();const funds=h.window.__elonBinanceCreateFundsV1;
    assert.equal(funds.start('doc_funds_test','b'.repeat(64),hash),true);await tick();
    const result=funds.read('doc_funds_test','b'.repeat(64),hash);assert.equal(result.status,'ready');
    assert.equal(result.available,portfolio?'7.8900':'12.345678901234');
    const calls=h.calls.filter(c=>c.url!==LIST);assert.deepEqual(calls.map(c=>c.url),[INFO,MODE,portfolio?SPOT:FUNDS,INFO]);
    const mode=calls[1];assert.equal(mode.init.method,'POST');assert.ok(!Object.hasOwn(mode.init,'body'));
    const balance=calls[2];assert.equal(balance.init.method,'POST');assert.equal(balance.init.credentials,'same-origin');
    assert.equal(balance.init.redirect,'error');assert.deepEqual(JSON.parse(balance.init.body),portfolio?{}:{assetName:'USDT'});
    assert.ok(!JSON.stringify(result).includes('SENSITIVE_CANARY'));assert.equal(h.events.length,0);
  }
});
test('neutral and trailing payloads preserve the official conditional fields', async () => {
  const h=harness(), p=payload(); p.direction='NEUTRAL'; delete p.autoInitPos;
  Object.assign(p,{trailingUp:true,trailingDown:false,orderCurrency:'QUOTE',trailingStopLowerLimit:true,trailingStopUpperLimit:false,
    triggerPrice:'1.1',triggerType:'MARK_PRICE',stopLowerLimit:'0.8',stopTriggerType:'MARK_PRICE',tpslCps:true,autoAddMargin:false});
  await h.prepare(p);
  assert.equal(h.api.submit('doc_test_123','a'.repeat(32)),true); await tick();
  const body=JSON.parse(h.calls.find(c=>c.url===CREATE).init.body);
  assert.equal(body.direction,'NEUTRAL'); assert.equal(body.orderCurrency,'QUOTE');
  assert.ok(!Object.hasOwn(body,'autoInitPos')); assert.ok(!Object.hasOwn(body,'slideWindow'));
  assert.equal(body.stopLowerLimit,'0.8'); assert.equal(body.tpslCps,true);
});
test('prepare performs only fixed reads and never exports observed headers',async()=>{
  const h=harness();await h.prepare();assert.equal(h.events.at(-1).kind,'prepared');
  assert.equal(h.calls.filter(c=>c.url===CREATE).length,0);
  assert.ok(!JSON.stringify(h.events).includes('SENSITIVE_CANARY'));
});
test('user submit builds observed fields exactly once and NEW only means accepted',async()=>{
  const h=harness();await h.prepare();assert.equal(h.api.submit('doc_test_123','a'.repeat(32)),true);
  assert.equal(h.api.submit('doc_test_123','a'.repeat(32)),false);await tick();
  const sent=h.calls.filter(c=>c.url===CREATE);assert.equal(sent.length,1);
  const body=JSON.parse(sent[0].init.body);assert.equal(body.gridInitialValue,'37.035');
  assert.equal(body.slideWindow,false);assert.equal(Object.keys(body).length,15);
  assert.equal(sent[0].init.redirect,'error');assert.equal(sent[0].init.credentials,'same-origin');
  assert.equal(h.events.at(-1).kind,'accepted');assert.equal(h.events.at(-1).provider_status,'NEW');
});
test('account switch after preparation blocks POST',async()=>{
  const h=harness();await h.prepare();h.account.userId='43';h.account.subUser=true;h.account.parentUser=false;
  h.api.submit('doc_test_123','a'.repeat(32));await tick();
  assert.equal(h.calls.filter(c=>c.url===CREATE).length,0);assert.equal(h.events.at(-1).kind,'not_sent');
});
test('account switch during the last configuration read cannot dispatch under the next account',async()=>{
  const h=harness();await h.prepare();h.behavior.switchDuringConfig=true;
  h.api.submit('doc_test_123','a'.repeat(32));await tick();
  assert.equal(h.calls.filter(c=>c.url===CREATE).length,0);assert.equal(h.events.at(-1).kind,'not_sent');
});
test('late changed dynamic config blocks submission instead of changing confirmed draft',async()=>{
  const h=harness();await h.prepare();h.behavior.windowCount=5;
  h.api.submit('doc_test_123','a'.repeat(32));await tick();assert.equal(h.events.at(-1).kind,'not_sent');
  assert.equal(h.calls.filter(c=>c.url===CREATE).length,0);
});
test('unknown outcomes never retry, including HTTP errors and mismatched receipts',async()=>{
  for(const failure of ['network','http','mismatch']) {
    const h=harness();await h.prepare();h.behavior.fail=failure;h.behavior.clientMismatch=failure==='mismatch';
    h.api.submit('doc_test_123','a'.repeat(32));await tick();
    assert.equal(h.events.at(-1).kind,'unknown');assert.equal(h.calls.filter(c=>c.url===CREATE).length,1);
    assert.equal(h.api.submit('doc_test_123','a'.repeat(32)),false);
    assert.ok(!JSON.stringify(h.events).includes('SENSITIVE_CANARY'));
  }
});
test('explicit business rejection is separate from unknown',async()=>{
  const h=harness();await h.prepare();h.behavior.fail='business';h.api.submit('doc_test_123','a'.repeat(32));await tick();
  assert.equal(h.events.at(-1).kind,'rejected');assert.equal(h.events.at(-1).code,'symbolInTwap');
});
test('missing or invalid coefficient fails closed without guessed defaults',async()=>{
  const h=harness();h.behavior.windowCount=null;await h.prepare();assert.equal(h.events.at(-1).kind,'prepare_failed');
  assert.equal(h.api.submit('doc_test_123','a'.repeat(32)),false);
});
test('extra fields, override context, changed token and caller mutation are rejected or frozen',async()=>{
  const h=harness();await h.observe();assert.equal(h.api.prepare('doc_test_123','b'.repeat(32),hash,{...payload(),url:CREATE}),false);
  const d=payload();await h.prepare(d);d.gridInitialValue='999999';
  assert.equal(h.api.submit('doc_wrong_123','a'.repeat(32)),false);
  h.api.submit('doc_test_123','a'.repeat(32));await tick();
  assert.equal(JSON.parse(h.calls.find(c=>c.url===CREATE).init.body).gridInitialValue,'37.035');
  const other=harness();await other.window.fetch(LIST,{method:'POST',headers:{'x-http-method-override':'DELETE'}});
  assert.equal(other.api.prepare('doc_test_123','a'.repeat(32),hash,payload()),false);
});
test('detail is a separate read bound to current account and returned ID',async()=>{
  const h=harness();await h.prepare();h.api.submit('doc_test_123','a'.repeat(32));await tick();
  assert.equal(h.api.detail('doc_test_123','a'.repeat(32),hash,'123'),true);await tick();
  assert.equal(h.events.at(-1).kind,'detail');assert.equal(h.events.at(-1).provider_status,'WORKING');
  assert.equal(h.calls.filter(c=>c.url===CREATE).length,1);
});
test('closing before dispatch cancels the pending write and consumes its attempt',async()=>{
  const h=harness();await h.prepare();
  assert.equal(h.api.submit('doc_test_123','a'.repeat(32)),true);h.api.cancel();await tick();
  assert.equal(h.calls.filter(c=>c.url===CREATE).length,0);assert.equal(h.events.at(-1).kind,'not_sent');
  assert.equal(h.events.at(-1).code,'cancelled_before_send');
  assert.equal(h.api.submit('doc_test_123','a'.repeat(32)),false);
});
test('coefficient controls sliding orders and is not a hardcoded 169',async()=>{
  const h=harness();h.behavior.windowCount=5;await h.prepare();h.api.submit('doc_test_123','a'.repeat(32));await tick();
  assert.equal(JSON.parse(h.calls.find(c=>c.url===CREATE).init.body).slideWindow,true);
});
test('XHR observed context remains confined to the page',async()=>{
  const h=harness(), xhr=new h.Xhr();xhr.open('POST',LIST);xhr.setRequestHeader('x-fixture-auth','SENSITIVE_CANARY');xhr.send();
  assert.equal(h.api.prepare('doc_test_123','a'.repeat(32),hash,payload()),true);await tick();
  assert.equal(h.events.at(-1).kind,'prepared');assert.ok(!JSON.stringify(h.events).includes('SENSITIVE_CANARY'));
});
