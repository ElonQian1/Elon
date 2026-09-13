(() => {
  'use strict';
  if(window.__elonBinanceRangeV1)return;
  const price=v=>typeof v==='string' && /^(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/.test(v);
  const scaled=v=>{const [a,b='']=v.split('.');return BigInt(a)*10n**20n+BigInt(b.padEnd(20,'0'));};
  const amount=v=>typeof v==='string' && /^(0|[1-9][0-9]{0,3})(\.[0-9]{1,8})?$/.test(v) && scaled(v)<=scaled('2000');
  const scalar=v=>typeof v==='string'?v:Number.isSafeInteger(v)?String(v):null;
  function draft(v) {
    if(!v || Object.keys(v).sort().join(',')!=='close_positions,count,investment_delta,lower,upper' ||
      !price(v.lower) || !price(v.upper) || scaled(v.lower)<=0n || scaled(v.lower)>=scaled(v.upper) ||
      !Number.isInteger(v.count) || v.count<2 || v.count>10000 || typeof v.close_positions!=='boolean' || !amount(v.investment_delta))return null;
    return Object.freeze({lower:v.lower,upper:v.upper,count:v.count,close_positions:v.close_positions,investment_delta:v.investment_delta});
  }
  function snapshot(d) {
    try {
      if(d.trailingUp!==false || d.trailingDown!==false || !['ARITH','GEO'].includes(d.gridType) ||
        !['LONG','SHORT','NEUTRAL'].includes(d.direction))return null;
      for(const key of ['stopTpPnl','stopSlPnl']) {
        if(d[key]!=null && d[key]!=='' && (scalar(d[key])===null || !price(scalar(d[key])) || scaled(scalar(d[key]))!==0n))return null;
      }
      const lower=scalar(d.gridLowerLimit),upper=scalar(d.gridUpperLimit),count=d.gridCount;
      if(!price(lower) || !price(upper) || scaled(lower)<=0n || scaled(lower)>=scaled(upper) ||
        !Number.isInteger(count) || count<2 || count>10000 || typeof d.cps!=='boolean')return null;
      if(d.tpslCps!=null && typeof d.tpslCps!=='boolean')return null;
      const preserved={tpslCps:typeof d.tpslCps==='boolean'?d.tpslCps:d.cps};
      for(const key of ['stopUpperLimit','stopLowerLimit','trailingUpLimitPrice','trailingDownLimitPrice']) {
        if(!Object.prototype.hasOwnProperty.call(d,key))continue;
        if(d[key]===null || d[key]===''){preserved[key]=d[key];continue;}
        const value=scalar(d[key]);if(!price(value))return null;preserved[key]=value;
      }
      return {lower,upper,count,grid_type:d.gridType,direction:d.direction,preserved};
    } catch(_){return null;}
  }
  function changed(before,after) {
    return before.count!==after.count || scaled(before.lower)!==scaled(after.lower) || scaled(before.upper)!==scaled(after.upper);
  }
  function body(strategyId,symbol,before,after) {
    if(!draft(after) || !before || !changed(before,after))throw Error('range_unavailable');
    return {strategyId:Number(strategyId),symbol,gridUpperLimit:after.upper,gridLowerLimit:after.lower,gridCount:after.count,
      updateRangeCps:after.close_positions,investmentDelta:after.investment_delta,...before.preserved};
  }
  window.__elonBinanceRangeV1=Object.freeze({draft,snapshot,changed,body});
})();
