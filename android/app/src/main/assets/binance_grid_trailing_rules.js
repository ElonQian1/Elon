/* Pure rules reconstructed from modules 15323, 48043, 2617 and 76807.
 * No I/O. Quote quantity uses verified editor inputs (37403/64290/30370),
 * not a saved per-grid quantity or a guessed adjustment coefficient.
 * Decimal division follows the website's explicit scale/rounding. Only geometric
 * pow/log use binary floating point, matching the website, never account balances. */
(() => {
  'use strict';
  const positive=/^(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/;
  const pow10=n=>10n**BigInt(n);
  function make(n,s) {
    while(s>0 && n%10n===0n){n/=10n;s--;}
    const digits=(n<0n?-n:n).toString().length;
    // Source decimal wrapper retains at most 38 decimal digits, including leading
    // fractional zeros. Do not approximate a quantity whose integer part exceeds it.
    if(digits-s>38)throw Error('precision');
    const cut=Math.max(digits,s+1)-38;
    if(cut>0){n/=pow10(cut);s-=cut;}
    while(s>0 && n%10n===0n){n/=10n;s--;}
    return {n,s};
  }
  function decimal(value,internal=false) {
    if(typeof value!=='string')throw Error('decimal');
    if(!internal && (!positive.test(value) || !/[1-9]/.test(value)))throw Error('decimal');
    const match=value.match(/^(-?)([0-9]+)(?:\.([0-9]+))?(?:e([+-]?[0-9]+))?$/i);
    if(!match)throw Error('decimal');
    const fraction=match[3] || '',exp=Number(match[4] || 0);
    if(Math.abs(exp)>100)throw Error('precision');
    let n=BigInt(match[1]+match[2]+fraction),s=fraction.length-exp;
    if(s<0){n*=pow10(-s);s=0;}
    return make(n,s);
  }
  function exactPrice(value) {
    if(typeof value!=='string' || !positive.test(value) || !/[1-9]/.test(value))throw Error('decimal');
    const [whole,fraction='']=value.split('.');
    return {n:BigInt(whole+fraction),s:fraction.length};
  }
  function align(a,b){const s=Math.max(a.s,b.s);return [a.n*pow10(s-a.s),b.n*pow10(s-b.s),s];}
  function add(a,b){const [x,y,s]=align(a,b);return make(x+y,s);}
  const neg=a=>({n:-a.n,s:a.s});
  const subtract=(a,b)=>add(a,neg(b));
  const multiply=(a,b)=>make(a.n*b.n,a.s+b.s);
  function compare(a,b){const [x,y]=align(a,b);return x===y?0:x>y?1:-1;}
  function divide(a,b,scale=16,halfUp=false) {
    if(b.n===0n)return make(0n,0); // Source decimal helper returns zero for a zero divisor.
    let x=a.n*pow10(b.s+scale),y=b.n*pow10(a.s),n=x/y;
    if(halfUp) {
      const remainder=x%y,abs=v=>v<0n?-v:v;
      if(abs(remainder)*2n>=abs(y))n+=(x<0n)!==(y<0n)?-1n:1n;
    }
    return make(n,scale);
  }
  function down(a,scale){return a.s<=scale?a:make(a.n/pow10(a.s-scale),scale);}
  function text(a) {
    const sign=a.n<0n?'-':'',raw=(a.n<0n?-a.n:a.n).toString().padStart(a.s+1,'0');
    return sign+(a.s?raw.slice(0,-a.s)+'.'+raw.slice(-a.s):raw);
  }
  const number=a=>Number(text(a));
  const minimum=(a,b)=>compare(a,b)<0?a:b;
  const maximum=(a,b)=>compare(a,b)>0?a:b;
  function gridRatio(upper,lower,count) {
    return decimal(String(Math.pow(number(divide(upper,lower,16,true)),number(divide(decimal('1'),count,16,true)))),true);
  }
  function neutralRatio(lower,upper,count,type,precision,reference,pivot,windowCount) {
    const n=Number(text(count)),epsilon=decimal('0.0000000000000001'),one=decimal('1');
    const step=type==='ARITH'?divide(subtract(upper,lower),count,32):gridRatio(upper,lower,count);
    const iterative=type==='GEO' && n>windowCount;
    let previous=lower,sum=make(0n,0);
    for(let i=0;i<=n;i++) {
      let level;
      if(i===n)level=upper;
      else if(type==='ARITH')level=add(lower,multiply(step,decimal(String(i),true)));
      else if(iterative) {
        if(i>0)previous=down(multiply(previous,down(step,16)),16);
        level=previous;
      } else level=decimal(String(number(lower)*Math.pow(number(step),i)),true);
      level=down(add(level,epsilon),precision);
      const higher=compare(level,pivot)>0;
      const denominator=higher?maximum(level,reference):minimum(level,reference);
      const numerator=higher?level:minimum(reference,level);
      sum=add(sum,divide(numerator,denominator,16,true));
    }
    return divide(sum,add(count,one),16,true);
  }
  function quoteQuantity(direction,margin,lower,upper,mark,trigger,count,leverage,adjust,quantityPrecision,pricePrecision,type,last,windowCount) {
    try {
      if(!['LONG','SHORT','NEUTRAL'].includes(direction) || !['ARITH','GEO'].includes(type) ||
        !/^[1-9][0-9]{0,4}$/.test(count) || Number(count)<2 || Number(count)>10000 ||
        !Number.isInteger(windowCount) || windowCount<1 || windowCount>10000 ||
        [quantityPrecision,pricePrecision].some(p=>!Number.isInteger(p) || p<0 || p>20))return null;
      const l=decimal(lower),u=decimal(upper),m=decimal(margin),lev=decimal(leverage),coefficient=decimal(adjust),n=decimal(count);
      const markPrice=decimal(mark),lastPrice=decimal(last),reference=trigger===''?markPrice:decimal(trigger);
      if(compare(l,u)>=0 || compare(lev,decimal('125'))>0 || compare(coefficient,decimal('1'))>0)return null;
      const ratio=direction==='LONG'?divide(reference,u,16,true):direction==='SHORT'?divide(l,reference,16,true):
        neutralRatio(l,u,n,type,pricePrecision,reference,trigger===''?lastPrice:reference,windowCount);
      const value=divide(multiply(multiply(multiply(coefficient,m),lev),ratio),add(n,decimal('1')),pricePrecision+quantityPrecision);
      return text(value);
    } catch{return null;}
  }
  function cap(qty,count,minQty,upper,lower,type,precision,maxPrice) {
    try {
      if(!Number.isInteger(precision) || precision<0 || precision>20 ||
        typeof count!=='string' || !/^[1-9][0-9]{0,4}$/.test(count) || Number(count)<2 || Number(count)>10000)return null;
      const q=decimal(qty),n=decimal(count),minimum=decimal(minQty),u=decimal(upper),l=decimal(lower),maximum=decimal(maxPrice);
      if(compare(l,u)>=0)return null;
      const quotient=divide(q,minimum),limit=down(compare(quotient,maximum)<0?quotient:maximum,precision);
      let result;
      if(type==='ARITH') {
        const step=divide(subtract(u,l),n,32);
        const moves=down(divide(subtract(limit,u),step),0);
        result=add(u,multiply(step,moves));
      } else if(type==='GEO') {
        const ratio=gridRatio(u,l,n);
        const logRatio=decimal(String(Math.log(number(ratio))),true);
        const logLimit=decimal(String(Math.log(number(divide(limit,u)))),true);
        const moves=down(divide(logLimit,logRatio),0);
        // Module 63064.Pl parses a Kotlin Int; accepting a larger move count would
        // produce a result where the official calculation rejects its input.
        if(moves.n< -2147483648n || moves.n>2147483647n)return null;
        result=decimal(String(number(u)*Math.pow(number(ratio),number(moves))),true);
      } else return null;
      return result.n>0n?text(down(result,8)):null;
    } catch{return null;}
  }
  function bounds(args,filters) {
    try {
      const tick=decimal(filters.tick),minimum=decimal(filters.min_price),maximum=decimal(filters.max_price);
      if(compare(minimum,maximum)>=0)return null;
      const t=number(tick),precision=t>1?0:Math.ceil(Math.abs(Math.log10(t)));
      const value=cap(args.qty,args.count,filters.min_quantity,args.upper,args.lower,args.type,precision,filters.max_price);
      if(value===null)return null;
      return Object.freeze({minimum:text(minimum),maximum:text(down(decimal(value),precision)),tick:text(tick)});
    } catch{return null;}
  }
  function editorBounds(snapshot,market,coefficients,now) {
    try {
      const marketKeys=['symbol','tick','min_quantity','min_price','max_price','quantity_step','price_precision','mark','last','observed_at'];
      if(!market || Object.keys(market).length!==marketKeys.length || marketKeys.some(k=>!Object.hasOwn(market,k)) ||
        !coefficients || Object.keys(coefficients).sort().join(',')!=='adjust,window_count' ||
        snapshot.symbol!==market.symbol || snapshot.provider_status!=='WORKING' ||
        !Number.isSafeInteger(now) || !Number.isSafeInteger(market.observed_at) || now-market.observed_at< -5000 || now-market.observed_at>20000)return null;
      const t=snapshot.trailing,p=snapshot.protection,i=snapshot.investment;
      if(!t || !p || !i || typeof t.up!=='boolean' || typeof t.down!=='boolean' || (!t.up && !t.down) ||
        !Number.isInteger(i.initial_leverage) || i.initial_leverage<1 || i.initial_leverage>125 ||
        typeof i.total_adjustment!=='string' || !/^-?(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/.test(i.total_adjustment))return null;
      decimal(i.initial_value);
      // 45337 uses Number inputs, decimal division/addition, and toNumber between
      // stages. This is the website's preview input, not the exact investment ledger.
      const initial=number(divide(decimal(String(Number(i.initial_value)),true),decimal(String(i.initial_leverage)),20,true));
      const margin=number(add(decimal(String(initial),true),decimal(String(Number(i.total_adjustment)),true)));
      if(!Number.isFinite(margin) || margin<=0)return null;
      const step=number(decimal(market.quantity_step)),quantityPrecision=step>1?0:Math.ceil(Math.abs(Math.log10(step)));
      const qty=quoteQuantity(p.direction,text(decimal(String(margin),true)),t.lower,t.upper,market.mark,p.trigger_price,String(t.count),
        String(i.initial_leverage),coefficients.adjust,quantityPrecision,market.price_precision,t.type,market.last,coefficients.window_count);
      if(qty===null)return null;
      return bounds({qty,count:String(t.count),upper:t.upper,lower:t.lower,type:t.type},market);
    } catch{return null;}
  }
  function validate(current,draft,rules) {
    try {
      if(!draft || Object.keys(draft).sort().join(',')!=='down_price,up_price' ||
        typeof current.up!=='boolean' || typeof current.down!=='boolean' || (!current.up && !current.down))return false;
      // User prices never inherit the source calculator's 38-digit truncation.
      const lower=exactPrice(current.lower),upper=exactPrice(current.upper),minimum=exactPrice(rules.minimum),maximum=exactPrice(rules.maximum),tick=exactPrice(rules.tick);
      if(compare(lower,upper)>=0 || compare(minimum,maximum)>=0)return false;
      for(const [enabled,key,bound,up] of [[current.up,'up_price',upper,true],[current.down,'down_price',lower,false]]) {
        if(!enabled){if(draft[key]!=='')return false;continue;}
        const v=exactPrice(draft[key]),[x,y]=align(v,tick);
        if(x%y!==0n || (up ? compare(v,bound)<=0 || compare(v,maximum)>0 : compare(v,bound)>=0 || compare(v,minimum)<0))return false;
      }
      return true;
    } catch{return false;}
  }
  window.__elonBinanceDecimalV1=Object.freeze({decimal,add,subtract,multiply,divide,compare,down,text,number,minimum,maximum,gridRatio});
  window.__elonBinanceTrailingRulesV1=Object.freeze({cap,quoteQuantity,bounds,editorBounds,validate});
})();
