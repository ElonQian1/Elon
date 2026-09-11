const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const {createHash}=require('node:crypto');
const prefix='/bapi/futures/v1/private/future/grid/',info='/bapi/accounts/v1/private/account/get-user-base-info';
const coef='/bapi/futures/v1/public/future/common/grid/coef',list='/bapi/futures/v2/private/future/grid/query-open-grids';
const accountHash=createHash('sha256').update('42').digest('hex'),token='doc_trailing_123',id='a'.repeat(32);
const tick=async()=>{for(let i=0;i<25;i++)await new Promise(r=>setImmediate(r));};
function fixture() {
 const events=[],calls=[],clock={now:100000},account={userId:'42',subUser:false,parentUser:true};
 const detail={strategyId:123,rootUserId:'42',symbol:'TESTUSDT',strategyStatus:'WORKING',direction:'LONG',cps:false,cos:true,sharing:true,
  trailingStopLowerLimit:true,trailingStopUpperLimit:false,trailingUp:true,trailingDown:false,trailingUpLimitPrice:'20',
  gridLowerLimit:'5',gridUpperLimit:'10',gridCount:10,gridType:'ARITH',gridInitialValue:'600.00',initialLeverage:3,totalAdjustmentAmount:'0.00',
  stopLowerLimit:'4',stopUpperLimit:null,stopTpPnl:null,stopSlPnl:null,tpslCps:true,autoInitPos:true};
 const behavior={fail:false,switchOnCoef:false,adjust:'0.8'},market=()=>({symbol:'TESTUSDT',tick:'0.01',min_quantity:'0.01',min_price:'0.01',max_price:'100000',
  quantity_step:'0.01',price_precision:2,mark:'7',last:'7',observed_at:clock.now});
 const response=data=>({status:200,clone:()=>({text:async()=>JSON.stringify({code:'000000',success:true,data})})});
 const window={crypto:{subtle:{digest:async(_,v)=>createHash('sha256').update(Buffer.from(v)).digest()}},
  ElonBinanceCreate:{postMessage:v=>events.push(JSON.parse(v))},fetch:async(url,init={})=>{
   calls.push({url,init});if(url===info)return response(account);
   if(url.startsWith(prefix+'query-grid-detail'))return response(detail);
   if(url===coef){if(behavior.switchOnCoef)account.userId='43';return response({windowCount:169,adjustCoef:behavior.adjust});}
   if(url===prefix+'update-grid'){if(behavior.fail)throw Error('CANARY');return response({strategyId:123,strategyStatus:'WORKING',updateStatus:'SUCCESS'});}
   return response([]);
  }};window.top=window;
 class Xhr{open(){}send(){}setRequestHeader(){}}
 const code=['protection_contract','trailing_rules','trailing_contract','create_adapter'].map(name=>fs.readFileSync(path.join(__dirname,`../android/app/src/main/assets/binance_grid_${name}.js`),'utf8')).join('\n');
 vm.runInNewContext(code,{window,location:{origin:'https://www.binance.com',href:'https://www.binance.com/'},URL,Headers,XMLHttpRequest:Xhr,AbortController,TextEncoder,setTimeout,clearTimeout,Date:{now:()=>clock.now}});
 const api=window.__elonBinanceManageV1,writes=()=>calls.filter(c=>c.url===prefix+'update-grid');
 async function read(enrich=false){await window.fetch(list,{method:'POST',headers:{'x-canary':'CANARY'}});assert.equal(api.inspect(token,id,accountHash,'123',enrich?market():null),true);await tick();return events.at(-1).snapshot;}
 async function prepare(){const baseline=await read();baseline.investment.initial_value='600';baseline.investment.total_adjustment='0';
  const input={baseline,draft:{up_price:'21',down_price:''},market:market()};assert.equal(api.prepareTrailing(token,id,accountHash,'123',input),true);await tick();return input;}
 return {api,events,calls,clock,account,detail,behavior,market,read,prepare,writes};
}
test('V4 reads computed rules and prepares a bound draft without a trade',async()=>{
 const h=fixture(),s=await h.read(true);assert.equal(s.trailing.up,true);assert.equal(s.trailing_rules.tick,'0.01');
 await h.prepare();assert.equal(h.events.at(-1).kind,'prepared');assert.equal(h.events.at(-1).trailing_draft.up_price,'21');assert.equal(h.writes().length,0);
});
test('one fresh confirmed submit preserves protection and never duplicates an unknown write',async()=>{
 for(const fail of [false,true]) {
  const h=fixture(),input=await h.prepare();input.draft.up_price='999';h.behavior.fail=fail;
  assert.equal(h.api.submit(token,id,h.market()),true);assert.equal(h.api.submit(token,id,h.market()),false);await tick();
  assert.equal(h.writes().length,1);const body=JSON.parse(h.writes()[0].init.body);
  assert.equal(body.trailingUpLimitPrice,'21');assert.equal(body.stopLowerLimit,'4');assert.equal(body.tpslCps,true);assert.equal(body.cps,false);assert.equal(body.cos,true);
  assert.equal(h.events.at(-1).kind,fail?'unknown':'accepted');assert.equal(JSON.stringify(h.events).includes('CANARY'),false);
 }
});
test('prepare rejects account switch after coefficient read and complete baseline drift',async()=>{
 for(const change of [h=>h.detail.cps=true,h=>h.detail.gridUpperLimit='11',h=>h.detail.totalAdjustmentAmount='1',h=>h.behavior.switchOnCoef=true]) {
  const h=fixture(),baseline=await h.read();change(h);
  h.api.prepareTrailing(token,id,accountHash,'123',{baseline,draft:{up_price:'21',down_price:''},market:h.market()});await tick();
  assert.equal(h.events.at(-1).kind,'prepare_failed');assert.equal(h.writes().length,0);
 }
});
test('submit rechecks private settings, account and fresh public rules before any write',async()=>{
 for(const change of [h=>h.detail.stopLowerLimit='3',h=>h.detail.sharing=false,h=>h.account.userId='43',h=>h.behavior.switchOnCoef=true,h=>h.behavior.adjust='']) {
  const h=fixture();await h.prepare();change(h);assert.equal(h.api.submit(token,id,h.market()),true);await tick();
  assert.equal(h.events.at(-1).kind,'not_sent');assert.equal(h.writes().length,0);
 }
 const h=fixture();await h.prepare();const stale=h.market();h.clock.now+=21000;h.api.submit(token,id,stale);await tick();
 assert.equal(h.events.at(-1).kind,'not_sent');assert.equal(h.writes().length,0);
});
test('expiry, cancellation and missing public inputs do not dispatch',async()=>{
 for(const expire of [true,false]){const h=fixture();await h.prepare();if(expire)h.clock.now+=60001;else h.api.cancel();
  assert.equal(h.api.submit(token,id,h.market()),false);assert.equal(h.writes().length,0);}
 const h=fixture();await h.prepare();assert.equal(h.api.submit(token,id),false);assert.equal(h.writes().length,0);
});
