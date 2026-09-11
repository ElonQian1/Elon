const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const assets=path.join(__dirname,'../android/app/src/main/assets');
const vectors=require('./fixtures/binance-grid-create-economics-vectors.json');
function harness() {
  const window={},clock={now:200000},calls=[],state={account:'a'.repeat(64)};
  const config={minGridCount:2,maxGridCount:1000,maxTrailingGridCount:169,windowCount:169,priceDiffBuffer:10,adjustCoef:0.8,trailingCoef:2};
  for(const file of ['binance_grid_trailing_rules.js','binance_grid_create_rules.js','binance_grid_create_economics.js','binance_grid_create_reference.js'])
    vm.runInNewContext(fs.readFileSync(path.join(assets,file),'utf8'),{window,Date:{now:()=>clock.now}});
  const deps={headers:()=>({private:'CANARY'}),identity:async account=>{calls.push('identity');if(state.wait)await state.wait;if(state.account!==account)throw Error('changed');},
    configuration:async()=>{calls.push('config');return {success:true,code:'000000',data:config}},
    commission:async()=>{calls.push('fee');return {success:true,code:'000000',data:{makerCommission:state.fee??200}}}};
  const api=window.__elonBinanceCreateReferenceFactoryV1(deps,2),v1=window.__elonBinanceCreateReferenceFactoryV1(deps);
  const input={symbol:'NEARUSDT',direction:'LONG',lower:'1.2',upper:'2.4',count:'33',leverage:'10',spacing:'ARITH',triggerPrice:'',trailingUp:'false',trailingDown:'false',marginType:'ISOLATED',margin:'200'};
  const market={mark:'1.85',minQty:'0.1',minNotional:'5',tick:'0.001',qtyPrecision:1,pricePrecision:3,last:'1.84',observedAt:200000};
  return {window,api,v1,input,market,clock,state,calls,config,token:'doc_economics_test',id:'b'.repeat(64),account:state.account};
}
const settle=()=>new Promise(resolve=>setImmediate(resolve));
async function read(h,input=h.input,id=h.id){assert.equal(h.api.start(h.token,id,h.account,input,h.market),true);await settle();return h.api.read(h.token,id,h.account)}
test('378 independent official calculator vectors match signed profit and base/quote quantity exactly',()=>{
  const h=harness();assert.equal(vectors.vectors.length,378);
  for(const v of vectors.vectors)assert.equal(JSON.stringify(h.window.__elonBinanceCreateEconomicsV1[v.operation](...v.args)),JSON.stringify(v.expected),JSON.stringify(v));
});
test('margin and trailing edits use current inputs; no fake quantity when a prerequisite is absent',async()=>{
  const h=harness();let r=await read(h);assert.equal(r.schema,'yilong.binance_create_reference.v2');assert.equal(r.quantity_status,'ready');assert.equal(r.quantity_unit,'NEAR');
  assert.ok(Number(r.quantity)>0);assert.ok(Number(r.profit_max)>Number(r.profit_min));assert.ok(!JSON.stringify(r).includes('CANARY'));
  const more=await read(h,{...h.input,margin:'400'});assert.ok(Number(more.quantity)>Number(r.quantity));assert.equal(more.minimum_margin,r.minimum_margin);
  const trailing=await read(h,{...h.input,trailingUp:'true'});assert.equal(trailing.quantity_unit,'USDT');assert.notEqual(trailing.quantity,r.quantity);
  const empty=await read(h,{...h.input,margin:''});assert.equal(empty.quantity_status,'margin_required');assert.equal(empty.quantity,'');assert.equal(empty.profit_min,r.profit_min);
  const small=await read(h,{...h.input,margin:'0.001'});assert.equal(small.quantity_status,'margin_below_minimum');assert.equal(small.quantity,'');
  const count=await read(h,{...h.input,count:''});assert.equal(count.quantity_status,'count_unavailable');assert.equal(count.profit_min,'');assert.ok(count.maximum_count>0);
  assert.equal(h.calls.filter(x=>x==='fee').length,1);
});
test('V1 shape remains strict and V2 accepts neither arbitrary inputs nor missing source precision',async()=>{
  const h=harness(),{margin,...old}=h.input,{last,pricePrecision,...market}=h.market;
  assert.equal(h.v1.start(h.token,h.id,h.account,old,market),true);await settle();
  const result=h.v1.read(h.token,h.id,h.account);assert.equal(result.schema,'yilong.binance_create_reference.v1');assert.equal(result.quantity,undefined);
  assert.equal(h.v1.start(h.token,h.id,h.account,h.input,h.market),false);
  assert.equal(h.api.start(h.token,h.id,h.account,old,market),false);
  for(const patch of [{margin:'-1'},{margin:'1e3'},{script:'fetch()'},{quantity:'1'}])assert.equal(h.api.start(h.token,h.id,h.account,{...h.input,...patch},h.market),false);
  assert.equal(h.api.start(h.token,h.id,h.account,h.input,{...h.market,pricePrecision:null}),false);
  h.market.pricePrecision=5;
  const newer=await read(h);
  assert.equal(newer.minimum_margin,result.minimum_margin,'minimum margin retains tick precision rather than quantity price precision');
  assert.equal(newer.quantity,h.window.__elonBinanceCreateEconomicsV1.quantity('LONG','200','1.2','2.4','1.85','','33','10','0.8',1,5,'ARITH',false,'1.84',169));
});
test('no late account/draft result or expired quote survives; fee failure has no default',async()=>{
  const h=harness();h.api.start(h.token,h.id,h.account,h.input,h.market);h.state.account='c'.repeat(64);await settle();assert.equal(h.api.read(h.token,h.id,h.account).status,'unavailable');
  const x=harness();x.api.start(x.token,x.id,x.account,x.input,x.market);await read(x,{...x.input,margin:'100'},'d'.repeat(64));assert.equal(x.api.read(x.token,x.id,x.account),null);
  x.clock.now+=120001;assert.equal(x.api.read(x.token,'d'.repeat(64),x.account).status,'expired');
  const y=harness();y.state.fee='200';assert.equal((await read(y)).status,'unavailable');
  const z=harness();z.market.last='';assert.equal((await read(z,{...z.input,direction:'NEUTRAL',trailingUp:'true'})).status,'unavailable');
});
test('invalid pure calculator inputs do not manufacture usable values',()=>{
  const {window}=harness(),api=window.__elonBinanceCreateEconomicsV1;
  for(const args of [['2','1','2','0','ARITH'],['1','2','0','0','ARITH'],['1','2','2','1','GEO'],['1','2','2','0','other']])assert.equal(api.profit(...args),null);
  for(const change of [{0:'other'},{1:'0'},{6:'10001'},{9:21},{12:'true'}]) {
    const args=['LONG','50','1','2','1.5','','33','10','0.8',1,3,'ARITH',false,'1.5',169];Object.assign(args,change);assert.equal(api.quantity(...args),null);
  }
});
