const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const {createHash,webcrypto}=require('node:crypto');
const code=fs.readFileSync(require('node:path').join(__dirname,'../android/app/src/main/assets/binance_grid_create_adapter.js'),'utf8');
const rangeCode=fs.readFileSync(require('node:path').join(__dirname,'../android/app/src/main/assets/binance_grid_range_contract.js'),'utf8');
const prefix='/bapi/futures/v1/private/future/grid/';
const LIST='/bapi/futures/v2/private/future/grid/query-open-grids',INFO='/bapi/accounts/v1/private/account/get-user-base-info';
const hash=createHash('sha256').update('42').digest('hex'),token='doc_manage_123',attempt='a'.repeat(32);
const tick=async()=>{for(let i=0;i<20;i++)await new Promise(r=>setImmediate(r));};
function fixture(){
  const calls=[],events=[],account={userId:'42',subUser:false,parentUser:true};
  const detail={strategyId:123,rootUserId:'42',symbol:'NEARUSDT',strategyStatus:'WORKING',cps:false,cos:true,sharing:true,trailingStopLowerLimit:false,trailingStopUpperLimit:true,
    gridInitialValue:'600',initialLeverage:3,totalAdjustmentAmount:'-0.25',gridLowerLimit:'1',gridUpperLimit:'2',gridCount:10,
    trailingUp:false,trailingDown:false,gridType:'ARITH',direction:'LONG',stopUpperLimit:'3',stopLowerLimit:'0.8',tpslCps:true};
  const behavior={fail:'',detailReads:0,switchAfterDetail:false};
  const response=(data,success=true,code='000000',status=200)=>({status,clone:()=>({text:async()=>JSON.stringify({data,success,code})})});
  class Xhr{open(){}send(){}setRequestHeader(){}}
  const window={crypto:{getRandomValues:v=>webcrypto.getRandomValues(v),subtle:{digest:async(_,v)=>createHash('sha256').update(Buffer.from(v)).digest()}},
    ElonBinanceCreate:{postMessage:r=>events.push(JSON.parse(r))},fetch:async(url,init={})=>{
      calls.push({url,init});
      if(url===INFO)return response(account);
      if(url.startsWith(prefix+'query-grid-detail')){behavior.detailReads++;if(behavior.switchAfterDetail)account.userId='43';return response(detail);}
      if(url===prefix+'update-grid' || url===prefix+'close-grid' || url===prefix+'update-grid-investment' || url===prefix+'update-grid-range'){
        if(behavior.fail==='network')throw Error('SECRET_CANARY');
        if(behavior.fail==='http')return response({},false,'999',500);
        if(behavior.fail==='business')return response({},false,'123456');
        if(url.endsWith('update-grid-investment') || url.endsWith('update-grid-range'))return response(behavior.fail==='false_data'?false:null);
        return response({strategyId:behavior.fail==='mismatch'?999:123,strategyStatus:url.endsWith('close-grid')?'CANCELED':'WORKING',updateStatus:'SUCCESS'});
      }
      return response([]);
    }};window.top=window;
  vm.runInNewContext(rangeCode+'\n'+code,{window,location:{origin:'https://www.binance.com',href:'https://www.binance.com/'},URL,Headers,XMLHttpRequest:Xhr,AbortController,TextEncoder,setTimeout,clearTimeout,Date});
  const api=window.__elonBinanceManageV1,create=window.__elonBinanceCreateV1;
  const observe=()=>window.fetch(LIST,{method:'POST',headers:{'x-canary':'SECRET_CANARY'}});
  const prepare=async(action='settings',cps=true,id=attempt)=>{await observe();assert.equal(api.prepare(token,id,hash,'123',action,cps),true);await tick();};
  const writes=()=>calls.filter(c=>c.url===prefix+'update-grid'||c.url===prefix+'close-grid'||c.url===prefix+'update-grid-investment'||c.url===prefix+'update-grid-range');
  return{api,create,observe,prepare,events,calls,detail,account,behavior,writes};
}
test('inspection and preparation are fixed reads without credentials in receipt',async()=>{
  const h=fixture();await h.observe();h.api.inspect(token,attempt,hash,'123');await tick();assert.equal(h.events.at(-1).kind,'detail');
  await h.prepare();assert.equal(h.events.at(-1).kind,'prepared');assert.equal(h.writes().length,0);
  assert.ok(!JSON.stringify(h.events).includes('SECRET_CANARY'));
});

const rangeDraft=()=>({lower:'1.1',upper:'2.1',count:12,close_positions:false,investment_delta:'0'});
async function prepareRange(h,draft=rangeDraft()){await h.observe();assert.equal(h.api.prepareRange(token,attempt,hash,'123',draft),true);await tick();}
test('range edits preserve price stops and explicit position treatment with a single write',async()=>{
  const h=fixture();const input=rangeDraft();await prepareRange(h,input);
  assert.equal(h.events.at(-1).kind,'prepared');assert.equal(h.writes().length,0);
  input.close_positions=true;input.lower='9';
  assert.equal(h.api.submit(token,attempt),true);assert.equal(h.api.submit(token,attempt),false);await tick();
  assert.equal(h.writes().length,1);assert.equal(h.writes()[0].url,prefix+'update-grid-range');
  assert.deepEqual(JSON.parse(h.writes()[0].init.body),{strategyId:123,symbol:'NEARUSDT',gridLowerLimit:'1.1',gridUpperLimit:'2.1',gridCount:12,
    updateRangeCps:false,investmentDelta:'0',stopUpperLimit:'3',stopLowerLimit:'0.8',tpslCps:true});
  assert.equal(h.events.at(-1).kind,'accepted');assert.equal(h.events.at(-1).provider_status,'');
});
test('range draft rejects missing decisions, excessive amounts, bad prices and no-op ranges',async()=>{
  const h=fixture();await h.observe();
  for(const d of [{...rangeDraft(),count:1},{...rangeDraft(),count:10001},{...rangeDraft(),lower:'2.2'},
    {...rangeDraft(),investment_delta:'2000.00000001'},{...rangeDraft(),investment_delta:''},{...rangeDraft(),close_positions:null},
    {...rangeDraft(),extra:true},{...rangeDraft(),lower:'1e-2'}])assert.equal(h.api.prepareRange(token,attempt,hash,'123',d),false);
  await prepareRange(h,{...rangeDraft(),lower:'1.000',upper:'2',count:10});assert.equal(h.events.at(-1).code,'no_change');assert.equal(h.writes().length,0);
});
test('advanced or imprecise range details block only range capability, not existing read or management',async()=>{
  for(const change of [d=>d.trailingUp=true,d=>delete d.trailingDown,d=>d.stopTpPnl='20',d=>d.gridLowerLimit=1.1,d=>delete d.gridCount,d=>d.stopUpperLimit='NaN']){
    const h=fixture();change(h.detail);await prepareRange(h);assert.equal(h.events.at(-1).code,'range_unavailable');assert.equal(h.writes().length,0);
    h.api.inspect(token,'b'.repeat(32),hash,'123');await tick();assert.equal(h.events.at(-1).kind,'detail');assert.equal(h.events.at(-1).snapshot.range,null);
  }
});
test('range revalidates original stops, range, funding and account before any financial request',async()=>{
  for(const change of [d=>d.gridLowerLimit='1.2',d=>d.gridCount=11,d=>d.tpslCps=false,d=>d.stopUpperLimit='4',d=>d.totalAdjustmentAmount='0',d=>d.direction='SHORT']){
    const h=fixture();await prepareRange(h);change(h.detail);h.api.submit(token,attempt);await tick();
    assert.equal(h.events.at(-1).kind,'not_sent');assert.equal(h.writes().length,0);
  }
  const h=fixture();await prepareRange(h);h.behavior.switchAfterDetail=true;h.api.submit(token,attempt);await tick();assert.equal(h.writes().length,0);
});
test('range unknown response is not retried and cancellation before dispatch stays read-only',async()=>{
  for(const fail of ['network','http','false_data','business']){
    const h=fixture();await prepareRange(h);h.behavior.fail=fail;h.api.submit(token,attempt);await tick();
    assert.equal(h.events.at(-1).kind,fail==='business'?'rejected':'unknown');assert.equal(h.writes().length,1);assert.equal(h.api.submit(token,attempt),false);
    assert.ok(!JSON.stringify(h.events).includes('SECRET_CANARY'));
  }
  const h=fixture();await prepareRange(h);h.api.submit(token,attempt);h.api.cancel();await tick();assert.equal(h.writes().length,0);
});
test('range preserves omitted versus null stops and the official tpslCps fallback',async()=>{
  const h=fixture();delete h.detail.stopUpperLimit;h.detail.stopLowerLimit=null;delete h.detail.tpslCps;
  await prepareRange(h,{...rangeDraft(),close_positions:true,investment_delta:'20.5'});h.api.submit(token,attempt);await tick();
  const body=JSON.parse(h.writes()[0].init.body);assert.equal('stopUpperLimit' in body,false);assert.equal(body.stopLowerLimit,null);
  assert.equal(body.tpslCps,false);assert.equal(body.updateRangeCps,true);assert.equal(body.investmentDelta,'20.5');
});
test('investment uses the V2 entry and sends only explicit margin delta once',async()=>{
  const h=fixture();await h.observe();
  assert.equal(h.api.prepare(token,attempt,hash,'123','investment',false),false);
  assert.equal(h.api.prepareInvestment(token,attempt,hash,'123','20.125'),true);await tick();
  assert.equal(h.events.at(-1).kind,'prepared');assert.equal(h.events.at(-1).investment_delta,'20.125');assert.equal(h.writes().length,0);
  assert.equal(h.api.submit(token,attempt),true);assert.equal(h.api.submit(token,attempt),false);await tick();
  assert.deepEqual(JSON.parse(h.writes()[0].init.body),{strategyId:123,symbol:'NEARUSDT',investmentDelta:'20.125'});
  assert.equal(h.writes().length,1);assert.equal(h.events.at(-1).kind,'accepted');assert.equal(h.events.at(-1).provider_status,'');
});
test('investment rejects invalid precision and never invents missing funding values',async()=>{
  const h=fixture();await h.observe();
  for(const delta of ['0','-1','1e3','2000.00000001','0.000000001','01',' 1',1,{},null])assert.equal(h.api.prepareInvestment(token,attempt,hash,'123',delta),false);
  delete h.detail.totalAdjustmentAmount;
  assert.equal(h.api.prepareInvestment(token,attempt,hash,'123','20'),true);await tick();
  assert.equal(h.events.at(-1).code,'investment_unavailable');assert.equal(h.api.submit(token,attempt),false);
});
test('investment compares funding and account again before dispatch',async()=>{
  for(const field of ['gridInitialValue','initialLeverage','totalAdjustmentAmount']){
    const h=fixture();await h.observe();h.api.prepareInvestment(token,attempt,hash,'123','20');await tick();
    h.detail[field]=field==='initialLeverage'?4:'1';h.api.submit(token,attempt);await tick();
    assert.equal(h.events.at(-1).kind,'not_sent');assert.equal(h.writes().length,0);
  }
  const h=fixture();await h.observe();h.api.prepareInvestment(token,attempt,hash,'123','20');await tick();
  h.behavior.switchAfterDetail=true;h.api.submit(token,attempt);await tick();assert.equal(h.writes().length,0);
});
test('investment ambiguous failures remain unresolved without a second write',async()=>{
  for(const fail of ['network','http','false_data','business']){
    const h=fixture();await h.observe();h.api.prepareInvestment(token,attempt,hash,'123','20');await tick();
    h.behavior.fail=fail;h.api.submit(token,attempt);await tick();
    assert.equal(h.events.at(-1).kind,fail==='business'?'rejected':'unknown');assert.equal(h.writes().length,1);
    assert.equal(h.api.submit(token,attempt),false);
  }
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
