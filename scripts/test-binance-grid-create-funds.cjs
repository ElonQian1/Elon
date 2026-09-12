const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const code=fs.readFileSync(path.join(__dirname,'../android/app/src/main/assets/binance_grid_create_funds.js'),'utf8');
const settle=()=>new Promise(resolve=>setImmediate(resolve));
const ok=data=>({success:true,code:'000000',data});
function harness() {
  const clock={now:200000},state={owner:'a'.repeat(64),mode:ok({enable:false}),futures:ok('123.45678901234567890123'),spot:ok([{asset:'USDT',free:'12.30000000'},{asset:'BTC',free:'999'}])},calls=[];
  const notifications=[],window={ElonBinanceReference:{postMessage:value=>notifications.push(JSON.parse(value))}};vm.runInNewContext(code,{window,Date:{now:()=>clock.now}});
  const deps={headers:()=>({secret:'CANARY'}),identity:async account=>{calls.push('identity');if(state.wait)await state.wait;if(state.owner!==account)throw Error('changed')},
    mode:async()=>{calls.push('mode');return state.mode},futures:async()=>{calls.push('futures');if(state.onRead)state.onRead();return state.futures},spot:async()=>{calls.push('spot');return state.spot}};
  const api=window.__elonBinanceCreateFundsFactoryV1(deps),token='doc_funds_test',request='b'.repeat(64),account=state.owner;
  const read=(id=request)=>api.read(token,id,account);
  const start=async(id=request)=>{assert.equal(api.start(token,id,account),true);await settle();return read(id)};
  return {state,clock,calls,api,token,request,account,read,start,notifications};
}
test('funds completion notifies once without funds or credentials; cancellation suppresses late events',async()=>{
  const h=harness();await h.start();assert.deepEqual(h.notifications,[{token:h.token,request:h.request}]);
  h.read();await settle();assert.equal(h.notifications.length,1);
  const x=harness();x.api.start(x.token,x.request,x.account);x.api.cancel();await settle();assert.deepEqual(x.notifications,[]);
  const f=harness();f.state.mode=ok({});await f.start();assert.deepEqual(f.notifications,[{token:f.token,request:f.request}]);
});
test('ordinary UM funds retain exact decimals and only the required projection',async()=>{
  const h=harness(),r=await h.start();assert.equal(r.status,'ready');assert.equal(r.available,'123.45678901234567890123');assert.equal(r.source,'futures_transferable');
  assert.equal(Object.keys(r).sort().join(','),'account,asset,available,observed_at,request,schema,source,status');
  assert.deepEqual(h.calls,['identity','mode','futures','identity']);assert.ok(!JSON.stringify(r).includes('CANARY'));
});
test('portfolio mode selects only the unique USDT free amount',async()=>{
  const h=harness();h.state.mode=ok({enable:true});const r=await h.start();assert.equal(r.available,'12.30000000');assert.equal(r.source,'portfolio_spot_free');assert.ok(!h.calls.includes('futures'));
});
test('actual zero remains distinguishable from missing or failed data',async()=>{
  for(const value of ['0','0.00000000',0]){const h=harness();h.state.futures=ok(value);assert.equal((await h.start()).available,String(value))}
  for(const value of [null,undefined,'',-1,0.1,Number.MAX_SAFE_INTEGER+1,'-1','01','1e2','NaN','0.'+'1'.repeat(21),{}]){
    const h=harness();h.state.futures=ok(value);const r=await h.start();assert.equal(r.status,'unavailable');assert.ok(!('available' in r));
  }
});
test('unknown account mode and failed responses never default to ordinary funds',async()=>{
  for(const mode of [ok({}),ok({enable:'false'}),{success:false,code:'000000',data:{enable:false}},ok(null)]){
    const h=harness();h.state.mode=mode;assert.equal((await h.start()).status,'unavailable');assert.deepEqual(h.calls,['identity','mode']);
  }
  const h=harness();h.state.futures={success:false,code:'000000',data:'12'};assert.equal((await h.start()).status,'unavailable');
});
test('missing, duplicate and malformed portfolio assets do not become zero',async()=>{
  for(const rows of [[],{},[{asset:'BTC',free:'1'}],[{asset:'USDT'}],[{asset:'USDT',free:'2'},{asset:'USDT',free:'3'}]]){
    const h=harness();h.state.mode=ok({enable:true});h.state.spot=ok(rows);assert.equal((await h.start()).status,'unavailable');
  }
});
test('identity change during the read discards funds',async()=>{
  const h=harness();h.state.onRead=()=>{h.state.owner='c'.repeat(64)};assert.equal((await h.start()).status,'unavailable');
});
test('cancelled and replaced requests cannot publish late funds',async()=>{
  const h=harness();let release;h.state.wait=new Promise(r=>release=r);h.api.start(h.token,h.request,h.account);h.api.cancel();release();await settle();assert.equal(h.read(),null);
  h.state.wait=null;await h.start('c'.repeat(64));assert.equal(h.read(),null);assert.equal(h.read('c'.repeat(64)).status,'ready');
});
test('deadline stops follow-up reads and freshness removes the amount',async()=>{
  const h=harness();let release;h.state.wait=new Promise(r=>release=r);h.api.start(h.token,h.request,h.account);h.clock.now+=30001;release();await settle();assert.equal(h.read().status,'unavailable');assert.deepEqual(h.calls,['identity']);
  const f=harness();await f.start();f.clock.now+=60000;assert.equal(f.read().status,'ready');f.clock.now++;assert.equal(f.read().status,'expired');assert.ok(!('available' in f.read()));
});
test('invalid identifiers cannot initiate a request or read another scope',async()=>{
  const h=harness();assert.equal(h.api.start('wrong',h.request,h.account),false);assert.equal(h.api.start(h.token,'bad',h.account),false);assert.equal(h.calls.length,0);
  await h.start();assert.equal(h.api.read('doc_other',h.request,h.account),null);assert.equal(h.api.read(h.token,h.request,'d'.repeat(64)),null);
});
