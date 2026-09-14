const {test}=require('node:test');
const assert=require('node:assert/strict');
const vm=require('node:vm');
const fs=require('node:fs');
const {webcrypto,createHash}=require('node:crypto');
const source=fs.readFileSync('android/app/src/main/assets/binance_wallet_adapter.js','utf8');
const request='a'.repeat(64), account='123', expected=createHash('sha256').update(account).digest('hex');
function harness(options={}) {
  const events=[],calls=[];let proves=0;
  const root={window:{},crypto:webcrypto,TextEncoder,AbortController,setTimeout,clearTimeout};
  vm.runInNewContext(source,root);
  const adapter=root.window.__elonBinanceWalletFactoryV1({
    context:()=>options.noContext?null:{headers:{}},
    prove:async()=>({account:++proves===2&&options.switchAccount?'456':account,account_kind:'sub'}),
    emit:event=>events.push(JSON.parse(JSON.stringify(event))),
    fetch:async(url,init)=>{calls.push({url,init});if(options.wait)await options.wait;
      return {status:options.http??200,text:async()=>JSON.stringify(options.body??{code:'000000',success:true,data:[{accountType:'FUTURE',activate:true,balance:'1.23000000000000000001',assetBalances:[{asset:'SECRET',free:'99'}]}]})};}
  });return {adapter,events,calls};
}
async function settled(h){for(let i=0;i<25&&!h.events.length;i++)await new Promise(r=>setTimeout(r,2));assert.ok(h.events.length);return h.events.at(-1);}
test('fixed same-origin GET returns summary with double identity proof and no asset details',async()=>{
  const h=harness();assert.equal(h.adapter.query({kind:'read',request,account:expected}),true);
  const e=await settled(h);assert.equal(e.kind,'wallet');assert.equal(e.quote_asset,'USDT');
  assert.deepEqual(e.wallets,[{type:'FUTURE',active:true,balance:'1.23000000000000000001'}]);
  assert.equal(h.calls[0].url,'/bapi/asset/v2/private/asset-service/wallet/balance?needBalanceDetail=true&quoteAsset=USDT');
  assert.equal(h.calls[0].init.method,'GET');assert.equal(h.calls[0].init.credentials,'same-origin');
  assert.equal(h.calls[0].init.redirect,'error');assert.ok(!JSON.stringify(e).includes('SECRET'));
});
test('identity-only authorization preparation does not read wallet',async()=>{
  const h=harness();h.adapter.query({kind:'identify',request});assert.equal((await settled(h)).kind,'identity');assert.equal(h.calls.length,0);
});
test('wrong authorized account is rejected before requesting money data',async()=>{
  const h=harness();h.adapter.query({kind:'read',request,account:'0'.repeat(64)});
  assert.equal((await settled(h)).error,'account_changed');assert.equal(h.calls.length,0);
});
test('account changes during read discard the result',async()=>{
  const h=harness({switchAccount:true});h.adapter.query({kind:'read',request,account:expected});
  const e=await settled(h);assert.equal(e.kind,'error');assert.equal(e.error,'account_changed');assert.equal(e.wallets,undefined);
});
for(const [name,options] of [['http',{http:429}],['business',{body:{code:'FAIL',success:false,data:[]}}],['duplicate',{body:{code:'000000',success:true,data:[{accountType:'FUTURE',activate:true,balance:'1'},{accountType:'FUTURE',activate:true,balance:'2'}]}}],['invalid decimal',{body:{code:'000000',success:true,data:[{accountType:'FUTURE',activate:true,balance:1}]}}]])
  test(name+' is not a zero wallet',async()=>{const h=harness(options);h.adapter.query({kind:'read',request,account:expected});assert.equal((await settled(h)).kind,'error');});
test('empty, inactive, missing amount, negative and unknown wallet types are distinct',async()=>{
  const h=harness({body:{code:'000000',success:true,data:[{accountType:'NEW_WALLET',activate:false},{accountType:'MARGIN',activate:true,balance:'-1.2'}]}});
  h.adapter.query({kind:'read',request,account:expected});assert.deepEqual((await settled(h)).wallets,[{type:'NEW_WALLET',active:false,balance:null},{type:'MARGIN',active:true,balance:'-1.2'}]);
  const empty=harness({body:{code:'000000',success:true,data:[]}});empty.adapter.query({kind:'read',request,account:expected});assert.deepEqual((await settled(empty)).wallets,[]);
});
test('unknown query fields, unsupported methods and missing context cannot trigger a read',()=>{
  const h=harness();assert.equal(h.adapter.query({kind:'read',request,account:expected,url:'https://example.com'}),false);
  assert.equal(h.adapter.query({kind:'trade',request}),false);
  assert.equal(harness({noContext:true}).adapter.query({kind:'identify',request}),false);
});
test('reset aborts and discards a response without replaying it',async()=>{
  let release;const h=harness({wait:new Promise(r=>{release=r;})});
  h.adapter.query({kind:'read',request,account:expected});for(let i=0;i<20&&!h.calls.length;i++)await new Promise(r=>setTimeout(r,2));
  h.adapter.reset();release();await new Promise(r=>setTimeout(r,20));assert.equal(h.events.length,0);assert.equal(h.calls[0].init.signal.aborted,true);
});
