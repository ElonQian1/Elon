const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const {createHash,webcrypto}=require('node:crypto');
const code=fs.readFileSync(require('node:path').join(__dirname,'../android/app/src/main/assets/binance_grid_create_adapter.js'),'utf8');
const prefix='/bapi/futures/v1/private/future/grid/';
const LIST='/bapi/futures/v2/private/future/grid/query-open-grids',INFO='/bapi/accounts/v1/private/account/get-user-base-info';
const hash=createHash('sha256').update('42').digest('hex'),token='doc_manage_123',attempt='a'.repeat(32);
const tick=async()=>{for(let i=0;i<20;i++)await new Promise(r=>setImmediate(r));};
function fixture(){
  const calls=[],events=[],account={userId:'42',subUser:false,parentUser:true};
  const detail={strategyId:123,rootUserId:'42',symbol:'NEARUSDT',strategyStatus:'WORKING',cps:false,cos:true,sharing:true,trailingStopLowerLimit:false,trailingStopUpperLimit:true};
  const behavior={fail:'',detailReads:0,switchAfterDetail:false};
  const response=(data,success=true,code='000000',status=200)=>({status,clone:()=>({text:async()=>JSON.stringify({data,success,code})})});
  class Xhr{open(){}send(){}setRequestHeader(){}}
  const window={crypto:{getRandomValues:v=>webcrypto.getRandomValues(v),subtle:{digest:async(_,v)=>createHash('sha256').update(Buffer.from(v)).digest()}},
    ElonBinanceCreate:{postMessage:r=>events.push(JSON.parse(r))},fetch:async(url,init={})=>{
      calls.push({url,init});
      if(url===INFO)return response(account);
      if(url.startsWith(prefix+'query-grid-detail')){behavior.detailReads++;if(behavior.switchAfterDetail)account.userId='43';return response(detail);}
      if(url===prefix+'update-grid' || url===prefix+'close-grid'){
        if(behavior.fail==='network')throw Error('SECRET_CANARY');
        if(behavior.fail==='http')return response({},false,'999',500);
        if(behavior.fail==='business')return response({},false,'123456');
        return response({strategyId:behavior.fail==='mismatch'?999:123,strategyStatus:url.endsWith('close-grid')?'CANCELED':'WORKING',updateStatus:'SUCCESS'});
      }
      return response([]);
    }};window.top=window;
  vm.runInNewContext(code,{window,location:{origin:'https://www.binance.com',href:'https://www.binance.com/'},URL,Headers,XMLHttpRequest:Xhr,AbortController,TextEncoder,setTimeout,clearTimeout,Date});
  const api=window.__elonBinanceManageV1,create=window.__elonBinanceCreateV1;
  const observe=()=>window.fetch(LIST,{method:'POST',headers:{'x-canary':'SECRET_CANARY'}});
  const prepare=async(action='settings',cps=true,id=attempt)=>{await observe();assert.equal(api.prepare(token,id,hash,'123',action,cps),true);await tick();};
  const writes=()=>calls.filter(c=>c.url===prefix+'update-grid'||c.url===prefix+'close-grid');
  return{api,create,observe,prepare,events,calls,detail,account,behavior,writes};
}
test('inspection and preparation are fixed reads without credentials in receipt',async()=>{
  const h=fixture();await h.observe();h.api.inspect(token,attempt,hash,'123');await tick();assert.equal(h.events.at(-1).kind,'detail');
  await h.prepare();assert.equal(h.events.at(-1).kind,'prepared');assert.equal(h.writes().length,0);
  assert.ok(!JSON.stringify(h.events).includes('SECRET_CANARY'));
});
test('settings preserves the four other typed fields and is single-use',async()=>{
  const h=fixture();await h.prepare();assert.equal(h.api.submit(token,attempt),true);assert.equal(h.api.submit(token,attempt),false);await tick();
  assert.deepEqual(JSON.parse(h.writes()[0].init.body),{strategyId:123,symbol:'NEARUSDT',cps:true,sharing:true,trailingStopLowerLimit:false,trailingStopUpperLimit:true});
  assert.equal(h.writes().length,1);assert.equal(h.events.at(-1).kind,'accepted');
});
test('close sends only strategyId and never changes settings first',async()=>{
  const h=fixture();await h.prepare('close',false);h.api.submit(token,attempt);await tick();
  assert.equal(h.writes().length,1);assert.equal(h.writes()[0].url,prefix+'close-grid');assert.deepEqual(JSON.parse(h.writes()[0].init.body),{strategyId:123});
});
test('different close mode and no-op settings cannot prepare',async()=>{
  for(const [action,cps,reason] of [['close',true,'close_mode_changed'],['settings',false,'no_change']]){
    const h=fixture();await h.prepare(action,cps);assert.equal(h.events.at(-1).code,reason);assert.equal(h.api.submit(token,attempt),false);
  }
});
test('missing booleans, wrong owner/id and nonworking status fail closed',async()=>{
  for(const change of [d=>delete d.sharing,d=>d.cps=1,d=>d.strategyId=456,d=>d.rootUserId='99',d=>d.strategyStatus='CLOSE_WITH_POSITION']){
    const h=fixture();change(h.detail);await h.prepare();assert.equal(h.events.at(-1).kind,'prepare_failed');assert.equal(h.writes().length,0);
  }
});
test('every relevant setting is compared again immediately before dispatch',async()=>{
  for(const key of ['cps','cos','sharing','trailingStopLowerLimit','trailingStopUpperLimit']){
    const h=fixture();await h.prepare();h.detail[key]=!h.detail[key];h.api.submit(token,attempt);await tick();
    assert.equal(h.events.at(-1).code,'settings_changed');assert.equal(h.writes().length,0);
  }
});
test('account switch during detail read and close-before-send prevent dispatch',async()=>{
  const h=fixture();await h.prepare();h.behavior.switchAfterDetail=true;h.api.submit(token,attempt);await tick();assert.equal(h.writes().length,0);
  const n=fixture();await n.prepare();n.api.submit(token,attempt);n.api.cancel();await tick();assert.equal(n.writes().length,0);assert.equal(n.events.at(-1).kind,'not_sent');
});
test('network, http, mismatched and business replies never automatically retry',async()=>{
  for(const fail of ['network','http','mismatch','business']){
    const h=fixture();await h.prepare();h.behavior.fail=fail;h.api.submit(token,attempt);await tick();
    assert.equal(h.events.at(-1).kind,fail==='business'?'rejected':'unknown');assert.equal(h.writes().length,1);
    assert.equal(h.api.submit(token,attempt),false);assert.ok(!JSON.stringify(h.events).includes('SECRET_CANARY'));
  }
});
test('unsafe numeric ID, token mismatch and create/management confusion are rejected',async()=>{
  const h=fixture();await h.observe();assert.equal(h.api.prepare(token,attempt,hash,'9007199254740992','close',false),false);
  await h.prepare();assert.equal(h.api.submit('doc_other_123',attempt),false);assert.equal(h.create.submit(token,attempt),false);assert.equal(h.writes().length,0);
});
