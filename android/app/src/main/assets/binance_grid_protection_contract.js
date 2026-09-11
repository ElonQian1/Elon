/* Binance grid edit modules 59977/24012/37954. Pure, fixed fields; no I/O or credentials. */
(() => {
  'use strict';
  const types=['MARK_PRICE','CONTRACT_PRICE'];
  const keys=['mode','lower','upper','tp','sl','stop_type','tpsl_cps'];
  function decimal(value,empty=true,precision=20) {
    if(value==null || value==='') {if(empty)return '';throw Error('missing');}
    if(typeof value==='number' && Number.isSafeInteger(value))value=String(value);
    if(typeof value!=='string' || !new RegExp('^(0|[1-9][0-9]{0,29})(\\.[0-9]{1,'+precision+'})?$').test(value))throw Error('decimal');
    let [whole,fraction='']=value.split('.');fraction=fraction.replace(/0+$/,'');
    const result=whole+(fraction?'.'+fraction:'');
    if(result==='0') {if(empty)return '';throw Error('zero');}return result;
  }
  const units=v=>{const [a,b='']=v.split('.');return BigInt(a)*10n**20n+BigInt(b.padEnd(20,'0'));};
  const exactKeys=(v,allowed)=>v && typeof v==='object' && !Array.isArray(v) && Object.keys(v).length===allowed.length && allowed.every(k=>Object.hasOwn(v,k));
  function draft(v) {try {
    if(!exactKeys(v,keys) || !['PRICE','PNL','CLEAR'].includes(v.mode) || !types.includes(v.stop_type) || typeof v.tpsl_cps!=='boolean')return null;
    const numbers=['lower','upper','tp','sl'];if(numbers.some(k=>typeof v[k]!=='string'))return null;
    if(v.mode==='PRICE' ? v.tp!=='' || v.sl!=='' : v.lower!=='' || v.upper!=='')return null;
    if(v.mode==='CLEAR' ? numbers.some(k=>v[k]!=='') : numbers.every(k=>v[k]===''))return null;
    const out={...v};for(const k of numbers)out[k]=v[k]===''?'':decimal(v[k],false,v.mode==='PNL'?2:20);
    if(out.lower && out.upper && units(out.lower)>=units(out.upper))return null;
    return Object.freeze(out);
  }catch{return null;}}
  function snapshot(d) {try {
    if(!['LONG','SHORT','NEUTRAL'].includes(d.direction))return null;
    for(const k of ['trailingUp','trailingDown','autoInitPos'])if(d[k]!=null && typeof d[k]!=='boolean')return null;
    if(typeof d.cps!=='boolean' || (d.tpslCps!=null && typeof d.tpslCps!=='boolean'))return null;
    const out={direction:d.direction,stop_type:d.stopTriggerType || 'CONTRACT_PRICE',
      lower:decimal(d.stopLowerLimit),upper:decimal(d.stopUpperLimit),tp:decimal(d.stopTpPnl),sl:decimal(d.stopSlPnl),
      tpsl_cps:typeof d.tpslCps==='boolean'?d.tpslCps:d.cps,trailing_up:!!d.trailingUp,trailing_down:!!d.trailingDown,
      trigger_type:d.triggerType || 'CONTRACT_PRICE',trigger_price:decimal(d.triggerPrice),auto_init:d.autoInitPos ?? null,
      trailing_up_price:decimal(d.trailingUpLimitPrice),trailing_down_price:decimal(d.trailingDownLimitPrice)};
    if(!types.includes(out.stop_type) || !types.includes(out.trigger_type))return null;
    if(!draft({mode:out.tp || out.sl?'PNL':out.lower || out.upper?'PRICE':'CLEAR',
      ...Object.fromEntries(keys.filter(k=>k!=='mode').map(k=>[k,out[k]]))}))return null;
    return Object.freeze(out);
  }catch{return null;}}
  function canonical(v) {
    if(Array.isArray(v))return v.map(canonical);
    if(v && typeof v==='object')return Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])]));
    return v;
  }
  function sameBaseline(current,expected) {try {
    const funding=v=>v==null?null:{initial_value:decimal(v.initial_value),initial_leverage:v.initial_leverage,
      total_adjustment:(String(v.total_adjustment).startsWith('-')?'-':'')+decimal(String(v.total_adjustment).replace(/^-/,''))};
    return JSON.stringify(canonical(current.protection))===JSON.stringify(canonical(expected.protection)) &&
      JSON.stringify(funding(current.investment))===JSON.stringify(funding(expected.investment));
  }catch{return false;}}
  function body(id,s,v) {
    if(!draft(v) || !s.protection)throw Error('protection_unavailable');
    const p=s.protection;
    const result={strategyId:Number(id),symbol:s.symbol,sharing:s.sharing,triggerType:p.trigger_type,triggerPrice:p.trigger_price,
      stopTriggerType:v.stop_type,stopLowerLimit:v.lower,stopUpperLimit:v.upper,stopTpPnl:v.tp,stopSlPnl:v.sl,
      cos:s.cos,cps:s.cps,tpslCps:v.tpsl_cps,
      trailingStopLowerLimit:p.trailing_up && !p.trailing_down && !!v.lower && ['LONG','NEUTRAL'].includes(p.direction),
      trailingStopUpperLimit:!p.trailing_up && p.trailing_down && !!v.upper && ['SHORT','NEUTRAL'].includes(p.direction)};
    if(p.auto_init!==null)result.autoInitPos=p.auto_init;
    if(p.trailing_up_price)result.trailingUpLimitPrice=p.trailing_up_price;
    if(p.trailing_down_price)result.trailingDownLimitPrice=p.trailing_down_price;
    return result;
  }
  window.__elonBinanceProtectionV1=Object.freeze({snapshot,draft,body,sameBaseline});
})();
