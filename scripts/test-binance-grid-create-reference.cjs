const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const assets=path.join(__dirname,'../android/app/src/main/assets');
const vectors=JSON.parse(fs.readFileSync(path.join(__dirname,'fixtures/binance-grid-create-reference-vectors.json'),'utf8'));
function harness() {
  const window={},clock={now:200000},state={account:'a'.repeat(64)},calls=[];
  const config={windowCount:169,maxGridCount:1000,minGridCount:2,maxTrailingGridCount:169,adjustCoef:0.8,trailingCoef:2,priceDiffBuffer:10};
  for(const name of ['binance_grid_trailing_rules.js','binance_grid_create_rules.js','binance_grid_create_reference.js'])
    vm.runInNewContext(fs.readFileSync(path.join(assets,name),'utf8'),{window,Date:{now:()=>clock.now}});
  const deps={headers:()=>({secret:'CANARY'}),identity:async account=>{calls.push('identity');if(state.wait)await state.wait;if(state.account!==account)throw Error('account_changed')},
    configuration:async()=>{calls.push('config');return {success:true,code:'000000',data:config}},
    commission:async(h,symbol)=>{calls.push('commission:'+symbol);return {success:true,code:'000000',data:{makerCommission:state.fee??200}}}};
  const api=window.__elonBinanceCreateReferenceFactoryV1(deps);
  const input={symbol:'NEARUSDT',direction:'LONG',lower:'1.2',upper:'2.4',count:'33',leverage:'10',spacing:'ARITH',triggerPrice:'',trailingUp:'false',trailingDown:'false',marginType:'ISOLATED'};
  const market={mark:'1.85',minQty:'0.1',minNotional:'5',tick:'0.001',qtyPrecision:1,observedAt:200000};
  const token='doc_reference_test',id='b'.repeat(64),account=state.account;
  return {window,clock,state,calls,config,api,input,market,token,id,account};
}
const settle=()=>new Promise(resolve=>setImmediate(resolve));
async function start(h,input=h.input,id=h.id){assert.equal(h.api.start(h.token,id,h.account,input,h.market),true);await settle();return h.api.read(h.token,id,h.account);}
test('336 captured same-generation oracle results match exact minimum margin and range calculations',()=>{
  const h=harness();assert.equal(vectors.vectors.length,336);
  for(const v of vectors.vectors)assert.equal(h.window.__elonBinanceCreateRulesV1[v.operation](...v.args),v.expected,JSON.stringify(v));
});
test('live configuration and current account commission drive results, without exporting credentials',async()=>{
  const h=harness(),r=await start(h);
  assert.equal(r.status,'ready');assert.ok(r.maximum_count>33);assert.ok(Number(r.minimum_margin)>0);
  assert.ok(!JSON.stringify(r).includes('CANARY'));assert.deepEqual(h.calls,['identity','config','commission:NEARUSDT','identity']);
  const more=await start(h,{...h.input,count:'66'},'c'.repeat(64));assert.ok(Number(more.minimum_margin)>Number(r.minimum_margin));
  const leveraged=await start(h,{...h.input,leverage:'20'},'d'.repeat(64));assert.notEqual(leveraged.minimum_margin,r.minimum_margin);
  const narrow=await start(h,{...h.input,upper:'1.21'},'e'.repeat(64));assert.ok(narrow.maximum_count<r.maximum_count);
  assert.equal(h.calls.filter(x=>x==='config').length,1);assert.equal(h.calls.filter(x=>x.startsWith('commission')).length,1);
});
test('range is returned before grid count; invalid count and too narrow range have no fake minimum',async()=>{
  const h=harness();let r=await start(h,{...h.input,count:''});assert.equal(r.code,'count_required');assert.equal(r.minimum_margin,'');
  r=await start(h,{...h.input,count:'10000'});assert.equal(r.code,'count_outside_range');assert.equal(r.minimum_margin,'');
  r=await start(h,{...h.input,upper:'1.2001'});assert.equal(r.code,'range_too_narrow');assert.equal(r.minimum_margin,'');
});
test('absent or malformed coefficient and account commission never fall back to hardcoded values',async()=>{
  for(const key of ['minGridCount','maxGridCount','maxTrailingGridCount','priceDiffBuffer','adjustCoef','trailingCoef','windowCount']) {
    const h=harness();delete h.config[key];assert.equal((await start(h)).status,'unavailable',key);
  }
  const h=harness();h.state.fee='200';assert.equal((await start(h)).status,'unavailable');
});
test('account switch or cancellation during a read cannot produce a usable result',async()=>{
  const h=harness();let release;h.state.wait=new Promise(resolve=>release=resolve);
  h.api.start(h.token,h.id,h.account,h.input,h.market);h.state.account='c'.repeat(64);release();await settle();
  assert.equal(h.api.read(h.token,h.id,h.account).status,'unavailable');
  const x=harness();x.api.start(x.token,x.id,x.account,x.input,x.market);x.api.cancel();await settle();assert.equal(x.api.read(x.token,x.id,x.account),null);
});
test('latest draft wins, scope mismatches fail closed, and expired market values cannot be renewed',async()=>{
  const h=harness();h.api.start(h.token,h.id,h.account,h.input,h.market);
  const newer='c'.repeat(64);await start(h,{...h.input,count:'66'},newer);
  assert.equal(h.api.read(h.token,h.id,h.account),null);assert.equal(h.api.read('doc_other_test',newer,h.account),null);
  assert.equal(h.api.read(h.token,newer,'d'.repeat(64)),null);
  h.clock.now+=120001;assert.equal(h.api.read(h.token,newer,h.account).status,'expired');
  assert.equal(h.api.start(h.token,h.id,h.account,h.input,h.market),false);
});
test('extra fields and arbitrary request details never enter the reference operation',()=>{
  const h=harness();for(const extra of [{url:'/trade'},{headers:{token:'secret'}},{script:'alert(1)'}])
    assert.equal(h.api.start(h.token,h.id,h.account,{...h.input,...extra},h.market),false);
  for(const edit of [{lower:'0'},{lower:'3'},{count:'1e3'},{leverage:'126'},{trailingUp:'yes'},{symbol:'BTCUSD_PERP'}])
    assert.equal(h.api.start(h.token,h.id,h.account,{...h.input,...edit},h.market),false);
  assert.deepEqual(h.calls,[]);
});
test('production adapter binds the reference operation to only observed identity, configuration and commission reads',async()=>{
  const crypto=require('node:crypto'),account=crypto.createHash('sha256').update('42').digest('hex');
  const h=harness(),calls=[];
  const window={crypto:crypto.webcrypto,fetch:async(url,init={})=>{
    calls.push({url,method:init.method,body:init.body});
    const data=url.endsWith('get-user-base-info')?{userId:'42',subUser:false,parentUser:true}:
      url.endsWith('/coef')?h.config:url.endsWith('account-tier-commission')?{makerCommission:200}:[];
    return {status:200,clone(){return this},text:async()=>JSON.stringify({success:true,code:'000000',data})};
  }};window.top=window;
  class Xhr{open(){}send(){}setRequestHeader(){}}
  const context={window,location:{origin:'https://www.binance.com',href:'https://www.binance.com/'},URL,Headers,XMLHttpRequest:Xhr,AbortController,TextEncoder,setTimeout,clearTimeout,Date};
  for(const name of ['binance_grid_trailing_rules.js','binance_grid_create_rules.js','binance_grid_create_economics.js','binance_grid_create_reference.js','binance_grid_create_adapter.js'])
    vm.runInNewContext(fs.readFileSync(path.join(assets,name),'utf8'),context);
  await window.fetch('/bapi/futures/v2/private/future/grid/query-open-grids',{method:'POST',headers:{'x-fixture':'CANARY'}});
  const api=window.__elonBinanceCreateReferenceV1;
  assert.equal(api.start(h.token,h.id,account,h.input,{...h.market,observedAt:Date.now()}),true);
  for(let i=0;i<10 && api.read(h.token,h.id,account)?.status==='pending';i++)await new Promise(resolve=>setTimeout(resolve,5));
  const result=api.read(h.token,h.id,account);assert.equal(result.status,'ready');assert.ok(!JSON.stringify(result).includes('CANARY'));
  assert.equal(calls.filter(x=>x.url.endsWith('account-tier-commission')).length,1);
  assert.deepEqual(JSON.parse(calls.find(x=>x.url.endsWith('account-tier-commission')).body),{name:'NEARUSDT'});
  const v2=window.__elonBinanceCreateReferenceV2;
  assert.equal(v2.start(h.token,h.id,account,{...h.input,margin:'200'},{...h.market,observedAt:Date.now(),last:'1.84',pricePrecision:3}),true);
  for(let i=0;i<10 && v2.read(h.token,h.id,account)?.status==='pending';i++)await new Promise(resolve=>setTimeout(resolve,5));
  assert.equal(v2.read(h.token,h.id,account).quantity_status,'ready');
  assert.equal(v2.read(h.token,h.id,account).quantity_unit,'NEAR');
  assert.ok(calls.every(x=>['query-open-grids','get-user-base-info','coef','account-tier-commission'].some(endpoint=>x.url.endsWith(endpoint))));
});
