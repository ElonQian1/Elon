const test=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const {webcrypto,createHash}=require('node:crypto');
const INFO='/bapi/accounts/v1/private/account/get-user-base-info';
const WALLET='/bapi/asset/v2/private/asset-service/wallet/balance?needBalanceDetail=true&quoteAsset=USDT';
const request='b'.repeat(64),account=createHash('sha256').update('42').digest('hex');
function response(data){const text=async()=>JSON.stringify({code:'000000',success:true,data});return {status:200,text,clone:()=>({text})};}
function harness(){
  const events=[],calls=[];let release,block=false;
  class Xhr {open(){}send(){}setRequestHeader(){}addEventListener(){}}
  const window={ElonBinanceRead:{postMessage:value=>events.push(JSON.parse(value))},fetch:async(url,init)=>{
    calls.push({url,init});
    if(url===INFO)return response({userId:'42',subUser:true,parentUser:false});
    assert.equal(url,WALLET);
    if(block)await new Promise(resolve=>{release=resolve;});
    return response([{accountType:'STRATEGY',activate:true,balance:'2.5'}]);
  }};window.top=window;
  const context={window,location:{origin:'https://www.binance.com',href:'https://www.binance.com/'},URL,Headers,XMLHttpRequest:Xhr,AbortController,setTimeout,clearTimeout,crypto:webcrypto,TextEncoder};
  for(const file of ['binance_wallet_adapter.js','binance_grid_read_adapter.js'])vm.runInNewContext(fs.readFileSync('android/app/src/main/assets/'+file,'utf8'),context);
  window.__elonBinanceReadV1.bind('doc_wallet_fixture');
  return {window,calls,events,block:()=>{block=true;},release:()=>release()};
}
async function until(condition){for(let i=0;i<50&&!condition();i++)await new Promise(r=>setTimeout(r,2));assert.ok(condition());}
test('composed host reads wallet using identity context without a single grid/list request',async()=>{
  const h=harness();await h.window.fetch(INFO,{method:'GET',headers:{'x-fixture-context':'keep-in-document'}});
  await until(()=>h.events.some(e=>e.kind==='identity'));
  assert.equal(h.window.__elonBinanceReadV1.wallet({kind:'read',request,account}),true);
  await until(()=>h.events.some(e=>e.kind==='wallet'));
  const event=h.events.find(e=>e.kind==='wallet');assert.equal(event.schema,'yilong.binance_wallet_observation.v1');assert.equal(event.token,'doc_wallet_fixture');
  assert.deepEqual(event.wallets,[{type:'STRATEGY',active:true,balance:'2.5'}]);
  assert.equal(h.calls.filter(c=>c.url===INFO).length,3);assert.equal(h.calls.filter(c=>c.url===WALLET).length,1);
  assert.equal(new Headers(h.calls.find(c=>c.url===WALLET).init.headers).get('x-fixture-context'),'keep-in-document');
  assert.ok(!JSON.stringify(h.events).includes('keep-in-document'));
});
test('rebinding the document cancels and discards the old wallet response',async()=>{
  const h=harness();await h.window.fetch(INFO,{method:'GET'});await until(()=>h.events.length>0);
  h.block();h.window.__elonBinanceReadV1.wallet({kind:'read',request,account});await until(()=>h.calls.some(c=>c.url===WALLET));
  h.window.__elonBinanceReadV1.bind('doc_wallet_new_generation');h.release();await new Promise(r=>setTimeout(r,20));
  assert.equal(h.events.filter(e=>e.schema==='yilong.binance_wallet_observation.v1').length,0);
});
