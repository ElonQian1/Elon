const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const window={};
for(const asset of ['protection_contract','trailing_rules','trailing_contract'])
 vm.runInNewContext(fs.readFileSync(path.join(__dirname,`../android/app/src/main/assets/binance_grid_${asset}.js`),'utf8'),{window});
const api=window.__elonBinanceTrailingV1;
const detail=()=>({strategyStatus:'WORKING',direction:'LONG',gridLowerLimit:'5',gridUpperLimit:'10',gridCount:10,gridType:'ARITH',
 cps:false,cos:true,sharing:true,trailingUp:true,trailingDown:false,trailingUpLimitPrice:'20.00',
 stopLowerLimit:'4',stopUpperLimit:null,stopTpPnl:null,stopSlPnl:null,stopTriggerType:'MARK_PRICE',
 tpslCps:true,triggerPrice:'6',triggerType:'CONTRACT_PRICE',autoInitPos:true});
const snapshot=d=>({strategy_id:'123',symbol:'TESTUSDT',provider_status:d.strategyStatus,cps:d.cps,cos:d.cos,sharing:d.sharing,
 trailingStopLowerLimit:true,trailingStopUpperLimit:false,investment:{initial_value:'100',initial_leverage:3,total_adjustment:'0'},
 range:null,protection:window.__elonBinanceProtectionV1.snapshot(d),trailing:api.snapshot(d)});
const rules={minimum:'0.01',maximum:'1000',tick:'0.01'};
test('trailing snapshot requires explicit running strategy fields and retains canonical enabled prices',()=>{
 const v=api.snapshot(detail());assert.equal(v.up,true);assert.equal(v.down,false);assert.equal(v.up_price,'20');assert.equal(v.down_price,'');
 for(const edit of [{trailingUp:null},{trailingDown:null},{trailingUp:false},{gridCount:'10'},
  {gridUpperLimit:'4'},{gridType:'OTHER'},{strategyStatus:'NEW'},{trailingUpLimitPrice:'NaN'}])
  assert.equal(api.snapshot({...detail(),...edit}),null);
});
test('only the selected trailing limit changes; preserve protection and ordinary position handling',()=>{
 const s=snapshot(detail()),d=api.draft({up_price:'21.00',down_price:''}),body=api.body('123',s,d,rules);
 assert.equal(body.trailingUpLimitPrice,'21');assert.equal('trailingDownLimitPrice' in body,false);
 assert.equal(body.stopLowerLimit,'4');assert.equal(body.stopUpperLimit,'');assert.equal(body.stopTpPnl,'');assert.equal(body.stopSlPnl,'');
 assert.equal(body.stopTriggerType,'MARK_PRICE');assert.equal(body.triggerType,'CONTRACT_PRICE');assert.equal(body.triggerPrice,'6');
 assert.equal(body.cos,true);assert.equal(body.cps,false);assert.equal(body.tpslCps,true);assert.equal(body.sharing,true);assert.equal(body.autoInitPos,true);
 assert.equal(body.trailingStopLowerLimit,true);assert.equal(body.trailingStopUpperLimit,false);
 for(const key of ['trailingUp','trailingDown','tpSlType','investmentDelta','up_price'])assert.equal(key in body,false);
});
test('PNL thresholds are retained exactly and price-specific stop flags recompute without mode replacement',()=>{
 const s=snapshot({...detail(),direction:'SHORT',trailingUp:false,trailingDown:true,trailingUpLimitPrice:null,
  trailingDownLimitPrice:'4',stopLowerLimit:null,stopTpPnl:'10.01',stopSlPnl:'2.03'});
 const body=api.body('123',s,api.draft({up_price:'',down_price:'3'}),rules);
 assert.equal(body.stopTpPnl,'10.01');assert.equal(body.stopSlPnl,'2.03');assert.equal(body.trailingDownLimitPrice,'3');
 assert.equal(body.trailingStopLowerLimit,false);assert.equal(body.trailingStopUpperLimit,false);
});
test('malformed drafts, inactive-side edits, unchanged intent and unsafe IDs cannot create a payload',()=>{
 const s=snapshot(detail());
 for(const v of [{up_price:'21',down_price:'',extra:true},{up_price:21,down_price:''},{up_price:'1e2',down_price:''}])assert.equal(api.draft(v),null);
 for(const d of [{up_price:'20',down_price:''},{up_price:'10',down_price:''},{up_price:'21',down_price:'3'}])assert.throws(()=>api.body('123',s,api.draft(d),rules));
 for(const id of ['0','9007199254740992','123x'])assert.throws(()=>api.body(id,s,api.draft({up_price:'21',down_price:''}),rules));
});
test('full baseline rejects changes to retained fields as well as the edited trailing limit',()=>{
 const s=snapshot(detail()),copy=JSON.parse(JSON.stringify(s));assert.equal(api.sameBaseline(s,copy),true);
 for(const mutate of [v=>v.cps=true,v=>v.cos=false,v=>v.sharing=false,v=>v.symbol='OTHERUSDT',
  v=>v.protection.sl='2',v=>v.trailing.upper='11',v=>v.investment.total_adjustment='1']) {
  const changed=JSON.parse(JSON.stringify(copy));mutate(changed);assert.equal(api.sameBaseline(s,changed),false);
 }
});
