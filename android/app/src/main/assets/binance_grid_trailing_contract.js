/* Working-grid limit editing, source module 59977. Pure payload construction only. */
(() => {
  'use strict';
  const keys=['up_price','down_price'];
  function price(value,empty=false) {
    if(empty && (value==null || value==='' || value===0))return '';
    if(Number.isSafeInteger(value))value=String(value);
    if(typeof value!=='string' || !/^(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/.test(value))throw Error('decimal');
    let [whole,fraction='']=value.split('.');fraction=fraction.replace(/0+$/,'');
    const result=whole+(fraction?'.'+fraction:'');
    if(result==='0'){if(empty)return '';throw Error('zero');}
    return result;
  }
  function snapshot(d) {
    try {
      if(d.strategyStatus!=='WORKING' || typeof d.trailingUp!=='boolean' || typeof d.trailingDown!=='boolean' ||
        (!d.trailingUp && !d.trailingDown) || !Number.isInteger(d.gridCount) || d.gridCount<2 || d.gridCount>10000 ||
        !['ARITH','GEO'].includes(d.gridType))return null;
      const lower=price(d.gridLowerLimit),upper=price(d.gridUpperLimit);
      const units=v=>{const [a,b='']=v.split('.');return BigInt(a)*10n**20n+BigInt(b.padEnd(20,'0'));};
      if(units(lower)>=units(upper))return null;
      // Preserve every optional provider field in the separate full protection snapshot.
      const upPrice=price(d.trailingUpLimitPrice,true),downPrice=price(d.trailingDownLimitPrice,true);
      return Object.freeze({up:d.trailingUp,down:d.trailingDown,up_price:d.trailingUp?upPrice:'',down_price:d.trailingDown?downPrice:'',
        lower,upper,count:d.gridCount,type:d.gridType});
    } catch{return null;}
  }
  function draft(v) {
    try {
      if(!v || typeof v!=='object' || Array.isArray(v) || Object.keys(v).sort().join(',')!=='down_price,up_price' ||
        keys.some(k=>typeof v[k]!=='string'))return null;
      return Object.freeze(Object.fromEntries(keys.map(k=>[k,v[k]===''?'':price(v[k])])));
    } catch{return null;}
  }
  function canonical(v) {
    if(Array.isArray(v))return v.map(canonical);
    if(v && typeof v==='object')return Object.fromEntries(Object.keys(v).sort().map(k=>[k,canonical(v[k])]));
    return v;
  }
  function sameBaseline(current,expected) {
    if(!current?.trailing || !expected?.trailing || !window.__elonBinanceProtectionV1?.sameBaseline(current,expected))return false;
    const rest=v=>Object.fromEntries(Object.entries(v).filter(([k])=>!['investment','protection','trailing_rules'].includes(k)));
    return JSON.stringify(canonical(rest(current)))===JSON.stringify(canonical(rest(expected)));
  }
  function body(id,s,input,rules) {
    const v=draft(input),p=s?.protection,t=s?.trailing;
    if(typeof id!=='string' || !/^[1-9][0-9]{0,15}$/.test(id) || !Number.isSafeInteger(Number(id)) ||
      s.strategy_id!==id || !/^[A-Z0-9]{1,24}USDT$/.test(s.symbol) || s.provider_status!=='WORKING' ||
      !v || !p || !t || p.trailing_up!==t.up || p.trailing_down!==t.down ||
      !window.__elonBinanceTrailingRulesV1?.validate(t,v,rules))throw Error('trailing_unavailable');
    if(keys.every(k=>v[k]===t[k]))throw Error('no_change');
    const mode=p.tp || p.sl?'PNL':p.lower || p.upper?'PRICE':'CLEAR';
    const original={mode,lower:p.lower,upper:p.upper,tp:p.tp,sl:p.sl,stop_type:p.stop_type,tpsl_cps:p.tpsl_cps};
    const result=window.__elonBinanceProtectionV1.body(id,s,original);
    if(t.up)result.trailingUpLimitPrice=v.up_price;
    if(t.down)result.trailingDownLimitPrice=v.down_price;
    return result;
  }
  window.__elonBinanceTrailingV1=Object.freeze({snapshot,draft,body,sameBaseline});
})();
