const test=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const window={};vm.runInNewContext(fs.readFileSync(require('node:path').join(__dirname,'../android/app/src/main/assets/binance_grid_protection_contract.js'),'utf8'),{window});
const api=window.__elonBinanceProtectionV1;
const detail=()=>({direction:'LONG',cps:false,cos:true,sharing:false,trailingUp:true,trailingDown:false,
  trailingUpLimitPrice:'4.50',stopLowerLimit:'1.20',stopUpperLimit:null,stopTpPnl:null,stopSlPnl:null,autoInitPos:true});
const draft=(mode='PNL')=>({mode,lower:'',upper:'',tp:mode==='PNL'?'10':'',sl:mode==='PNL'?'5':'',stop_type:'MARK_PRICE',tpsl_cps:true});
test('source defaults and optional values are canonical and strict',()=>{
 const s=api.snapshot(detail());assert.equal(s.lower,'1.2');assert.equal(s.stop_type,'CONTRACT_PRICE');assert.equal(s.tpsl_cps,false);
 assert.equal(s.trailing_up_price,'4.5');assert.equal(s.auto_init,true);
 for(const edit of [d=>d.stopSlPnl='-5',d=>d.stopLowerLimit='1e1',d=>d.stopTpPnl='1',d=>d.trailingUp='true',d=>d.stopTriggerType='OTHER']){
  const d=detail();edit(d);assert.equal(api.snapshot(d),null);
 }
});
test('mode replacement preserves unrelated settings and derives trailing stop flags',()=>{
 const s=api.snapshot(detail()),base={symbol:'NEARUSDT',cos:true,cps:false,sharing:true,protection:s};
 const b=api.body('123',base,api.draft(draft()));
 assert.equal(b.stopLowerLimit,'');assert.equal(b.stopUpperLimit,'');assert.equal(b.stopTpPnl,'10');assert.equal(b.stopSlPnl,'5');
 assert.equal(b.cps,false);assert.equal(b.cos,true);assert.equal(b.tpslCps,true);assert.equal(b.autoInitPos,true);
 assert.equal(b.trailingUpLimitPrice,'4.5');assert.equal(b.trailingStopLowerLimit,false);assert.equal('tpSlType' in b,false);
 const price=api.draft({...draft('PRICE'),lower:'1.1',upper:'3'});
 assert.equal(api.body('123',base,price).trailingStopLowerLimit,true);
 const clear=api.body('123',base,api.draft(draft('CLEAR')));assert.equal(clear.stopTpPnl,'');assert.equal(clear.stopSlPnl,'');
});
test('reject ambiguous empty, exponent, mixed modes, extra keys and zero targets',()=>{
 for(const d of [draft('ROI'),{...draft(),sl:'-1'},{...draft(),tp:'0'}, {...draft(),tp:'1.001'},
  {...draft(),tp:'1e2'}, {...draft(),lower:'1'},{...draft(),extra:true},draft('PRICE')])assert.equal(api.draft(d),null);
 assert.ok(api.draft(draft('CLEAR')));
});
test('baseline ignores object key order and decimal formatting but detects changed margin',()=>{
 const p=api.snapshot(detail()),investment={initial_value:'100.00',initial_leverage:3,total_adjustment:'0.0100'};
 const expected={protection:{...p},investment:{initial_value:'100',initial_leverage:3,total_adjustment:'0.01'}};
 assert.equal(api.sameBaseline({protection:p,investment},expected),true);
 investment.total_adjustment='0.02';assert.equal(api.sameBaseline({protection:p,investment},expected),false);
});
