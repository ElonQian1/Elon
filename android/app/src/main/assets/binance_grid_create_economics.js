/* Pure reference values, independently implemented from verified public modules
 * 78387/64290/30370. No network, account storage or execution capability. */
(() => {
  'use strict';
  const math=window.__elonBinanceDecimalV1,rules=window.__elonBinanceCreateRulesV1;
  if(!math || !rules)return;
  const {decimal:d,add,subtract:sub,multiply:mul,divide:div,compare:cmp,text,gridRatio}=math;
  const one=d('1'),hundred=d('100');
  function profit(lower,upper,count,fee,type) {
    try {
      const l=d(lower),u=d(upper),n=d(count),f=d(fee,true);
      if(cmp(l,u)>=0 || !Number.isSafeInteger(Number(count)) || Number(count)<2 || Number(count)>10000 ||
        cmp(f,d('-1',true))<=0 || cmp(f,one)>=0)return null;
      if(type==='GEO') {
        const rate=mul(hundred,sub(sub(mul(sub(one,f),gridRatio(u,l,n)),one),f));
        return [text(rate),text(rate)];
      }
      if(type!=='ARITH')return null;
      const step=div(sub(u,l),n,32);
      const min=mul(hundred,sub(sub(div(mul(u,sub(one,f)),sub(u,step),16,true),one),f));
      const max=mul(hundred,sub(div(mul(sub(one,f),step),l,16,true),mul(d('2'),f)));
      return [text(min),text(max)];
    }catch{return null;}
  }
  function quantity(direction,margin,lower,upper,mark,trigger,count,leverage,adjust,qtyPrecision,pricePrecision,type,trailing,last,windowCount) {
    try {
      if(!['LONG','SHORT','NEUTRAL'].includes(direction) || !['ARITH','GEO'].includes(type) || typeof trailing!=='boolean' ||
        !/^[1-9][0-9]{0,4}$/.test(count) || Number(count)<2 || Number(count)>10000 ||
        [qtyPrecision,pricePrecision].some(v=>!Number.isInteger(v)||v<0||v>20) ||
        !Number.isInteger(windowCount) || windowCount<1 || windowCount>10000 || cmp(d(lower),d(upper))>=0 ||
        cmp(d(leverage),d('125'))>0 || cmp(d(adjust),one)>0)return null;
      if(trailing)return window.__elonBinanceTrailingRulesV1.quoteQuantity(direction,margin,lower,upper,mark,trigger,count,
        leverage,adjust,qtyPrecision,pricePrecision,type,last,windowCount);
      return text(div(mul(mul(d(adjust),d(margin)),d(leverage)),
        rules.weightedSum(direction,lower,upper,mark,trigger,count,leverage,pricePrecision,type,windowCount),qtyPrecision));
    }catch{return null;}
  }
  window.__elonBinanceCreateEconomicsV1=Object.freeze({profit,quantity});
})();
