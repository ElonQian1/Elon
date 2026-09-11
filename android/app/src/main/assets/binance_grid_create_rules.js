/* Pure UM create reference rules. Reconstructed behavior of 3899/64290/30370;
 * shared decimal semantics with the independently verified trailing calculator.
 * Inputs are live configuration, symbol filters and account fee, never defaults. */
(() => {
  'use strict';
  const math=window.__elonBinanceDecimalV1;
  if(!math)return;
  const {decimal:d,add,subtract:sub,multiply:mul,divide:div,compare:cmp,down,text,number:num,minimum:min,maximum:max,gridRatio}=math;
  const zero=d('0',true),one=d('1');
  function up(value,scale) {
    const floor=down(value,scale);
    return cmp(floor,value)<0?add(floor,d(`1e-${scale}`,true)):floor;
  }
  function divideUp(a,b,scale){
    const numerator=a.n*10n**BigInt(b.s+scale),denominator=b.n*10n**BigInt(a.s);
    const units=numerator/denominator+(numerator%denominator===0n?0n:1n);
    return d(`${units}e-${scale}`,true);
  }
  function precision(tick){return Math.max(0,d(tick).s);}
  function maximumCount(upper,lower,type,tick,fee,cap,buffer) {
    try {
      const u=d(upper),l=d(lower),t=d(tick),f=d(fee,true),c=d(cap),b=d(buffer);
      if(cmp(u,l)<=0 || cmp(f,d('-1',true))<=0 || cmp(f,one)>=0 || !['ARITH','GEO'].includes(type))return null;
      const step=mul(t,b),span=sub(u,l);
      if(type==='ARITH') {
        let feeStep=up(mul(mul(f,u),d('2')),precision(tick));
        if(cmp(feeStep,zero)<=0)feeStep=t;
        return num(min(min(div(span,step,0),div(span,feeStep,0)),c));
      }
      const ratio=num(div(u,l,16,true));
      const tickRatio=num(add(div(step,l,16,true),one));
      const feeRatio=num(div(add(one,f),sub(one,f),16,true));
      const byTick=Math.floor(Math.log(ratio)/Math.log(tickRatio));
      const byFee=cmp(f,zero)>0?Math.floor(Math.log(ratio)/Math.log(feeRatio)):num(c);
      const result=Math.min(byTick,byFee,num(c));
      return Number.isSafeInteger(result)&&result>=0?result:null;
    } catch{return null;}
  }
  function levels(lower,upper,count,scale,type,windowCount) {
    const n=Number(count),l=d(lower),u=d(upper),epsilon=d('0.0000000000000001');
    const step=type==='ARITH'?div(sub(u,l),d(count),32):gridRatio(u,l,d(count));
    const iterative=type==='GEO' && n>windowCount;
    let previous=l;
    return Array.from({length:n+1},(_,index)=>{
      let value;
      if(index===n)value=u;
      else if(type==='ARITH')value=add(l,mul(step,d(String(index),true)));
      else if(iterative){if(index>0)previous=down(mul(previous,down(step,16)),16);value=previous;}
      else value=d(String(num(l)*Math.pow(num(step),index)),true);
      return down(add(value,epsilon),scale);
    });
  }
  function minimumMargin(direction,minQty,minNotional,lower,upper,mark,trigger,count,leverage,adjust,qtyPrecision,pricePrecision,type,trailing,trailingCoef,windowCount) {
    try {
      if(!['LONG','SHORT','NEUTRAL'].includes(direction) || !['ARITH','GEO'].includes(type) ||
        !/^[1-9][0-9]{0,4}$/.test(count) || Number(count)<2 || Number(count)>10000 ||
        !Number.isInteger(windowCount) || windowCount<1 || windowCount>10000 || typeof trailing!=='boolean' ||
        [qtyPrecision,pricePrecision].some(v=>!Number.isInteger(v)||v<0||v>20))return null;
      const l=d(lower),u=d(upper),lev=d(leverage),coefficient=d(adjust),notional=d(minNotional);
      if(cmp(l,u)>=0 || cmp(lev,d('125'))>0 || cmp(coefficient,one)>0)return null;
      const qty=max(d(minQty),divideUp(notional,l,qtyPrecision));
      if(trailing) {
        const n=add(d(count),one),value=max(mul(n,notional),mul(mul(mul(n,d(trailingCoef)),u),qty));
        return text(div(value,lev,16,true));
      }
      const reference=trigger===''?d(mark):d(trigger);
      let sum=zero;
      for(const price of levels(lower,upper,count,pricePrecision,type,windowCount)) {
        let amount=price;
        if(direction!=='NEUTRAL') {
          const loss=max(zero,direction==='LONG'?sub(price,reference):sub(reference,price));
          amount=add(direction==='SHORT'?max(price,reference):price,mul(lev,loss));
        }
        sum=add(sum,mul(qty,amount));
      }
      return text(divideUp(sum,mul(lev,coefficient),pricePrecision));
    } catch{return null;}
  }
  window.__elonBinanceCreateRulesV1=Object.freeze({maximumCount,minimumMargin});
})();
