/* Fixed read-only reference operation. All network dependencies are captured by
 * the host adapter; callers cannot supply URLs, headers, scripts or trade bodies. */
(() => {
  'use strict';
  const keys=['symbol','direction','lower','upper','count','leverage','spacing','triggerPrice','trailingUp','trailingDown','marginType'];
  const decimal=v=>typeof v==='string' && /^(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/.test(v) && /[1-9]/.test(v);
  const int=(v,low,high)=>Number.isSafeInteger(v) && v>=low && v<=high;
  function valid(input,market,version) {
    const inputKeys=version===2?[...keys,'margin']:keys;
    return input && Object.keys(input).sort().join(',')===inputKeys.slice().sort().join(',') && inputKeys.every(k=>typeof input[k]==='string') &&
      (version===1 || input.margin==='' || decimal(input.margin)) &&
      /^[A-Z0-9]{1,24}USDT$/.test(input.symbol) && ['LONG','SHORT','NEUTRAL'].includes(input.direction) &&
      ['ISOLATED','CROSSED'].includes(input.marginType) && ['ARITH','GEO'].includes(input.spacing) &&
      ['trailingUp','trailingDown'].every(k=>['true','false'].includes(input[k])) &&
      decimal(input.lower) && decimal(input.upper) && Number(input.lower)<Number(input.upper) &&
      (input.triggerPrice==='' || decimal(input.triggerPrice)) &&
      (input.count==='' || /^[1-9][0-9]{0,4}$/.test(input.count) && Number(input.count)<=10000) &&
      /^[1-9][0-9]{0,2}$/.test(input.leverage) && Number(input.leverage)<=125 &&
      market && Object.keys(market).sort().join(',')===(version===2?'last,mark,minNotional,minQty,observedAt,pricePrecision,qtyPrecision,tick':'mark,minNotional,minQty,observedAt,qtyPrecision,tick') &&
      (version===1 || int(market.pricePrecision,0,20) && (market.last==='' || decimal(market.last))) &&
      ['mark','minNotional','minQty','tick'].every(k=>decimal(market[k])) && int(market.qtyPrecision,0,20) &&
      int(market.observedAt,1,Number.MAX_SAFE_INTEGER) && Date.now()-market.observedAt>=-5000 && Date.now()-market.observedAt<=120000;
  }
  function configuration(value) {
    if(value?.success!==true || value.code!=='000000')throw Error('configuration_unavailable');
    const c=value.data;
    if(!c || !['minGridCount','maxGridCount','maxTrailingGridCount','windowCount'].every(k=>int(c[k],1,10000)) ||
      c.minGridCount<2 || c.minGridCount>c.maxGridCount || c.minGridCount>c.maxTrailingGridCount ||
      !['priceDiffBuffer','adjustCoef','trailingCoef'].every(k=>typeof c[k]==='number' && Number.isFinite(c[k]) && c[k]>0) ||
      c.adjustCoef>1)throw Error('configuration_unavailable');
    return c;
  }
  window.__elonBinanceCreateReferenceFactoryV1=(deps,version=1)=>{
    if(![1,2].includes(version))throw Error('unsupported_reference_version');
    const schema=`yilong.binance_create_reference.v${version}`;
    let generation=0,current=null,config=null,fees=null;
    const status=(request,account,symbol,state)=>({schema,request,account,symbol,status:state});
    return Object.freeze({
      start(token,request,account,input,market) {
        if(!/^doc_[a-z0-9_]{3,80}$/.test(token) || !/^[a-f0-9]{64}$/.test(request) || !/^[a-f0-9]{64}$/.test(account) || !valid(input,market,version))return false;
        input=JSON.parse(JSON.stringify(input));market=JSON.parse(JSON.stringify(market));
        const ticket=++generation,base=status(request,account,input.symbol,'pending');current={...base,token};
        (async()=>{try {
          const h=deps.headers();await deps.identity(account,h);
          if(!config || Date.now()-config.at<0 || Date.now()-config.at>=300000)config={value:configuration(await deps.configuration(h)),at:Date.now()};
          if(!fees || fees.account!==account || fees.symbol!==input.symbol || fees.token!==token || Date.now()-fees.at<0 || Date.now()-fees.at>=60000) {
            const v=await deps.commission(h,input.symbol),fee=v?.data?.makerCommission;
            if(v?.success!==true || v.code!=='000000' || typeof fee!=='number' || !Number.isFinite(fee) || fee<=-1000000 || fee>=1000000)throw Error('commission_unavailable');
            fees={account,symbol:input.symbol,token,value:String(fee/1000000),at:Date.now()};
          }
          const c=config.value,fee=fees.value,api=window.__elonBinanceCreateRulesV1;
          const trailing=input.trailingUp==='true'||input.trailingDown==='true';
          const maximum=api.maximumCount(input.upper,input.lower,input.spacing,market.tick,fee,
            String(trailing?c.maxTrailingGridCount:c.maxGridCount),Number(market.tick)>0.00001?'1':String(c.priceDiffBuffer));
          if(!int(maximum,0,10000))throw Error('calculation_unavailable');
          const count=Number(input.count),countValid=input.count!=='' && count>=c.minGridCount && count<=maximum;
          // The website uses tick precision for minimum margin, but exchange
          // pricePrecision for quantity. These differ for some contracts.
          const precision=window.__elonBinanceDecimalV1.decimal(market.tick).s;
          const minimum=countValid?api.minimumMargin(input.direction,market.minQty,market.minNotional,input.lower,input.upper,
            market.mark,input.triggerPrice,input.count,input.leverage,String(c.adjustCoef),market.qtyPrecision,precision,
            input.spacing,trailing,String(c.trailingCoef),c.windowCount):'';
          if(countValid && !decimal(minimum))throw Error('calculation_unavailable');
          let economics={};
          if(version===2) {
            const calculator=window.__elonBinanceCreateEconomicsV1;
            const profit=countValid?calculator.profit(input.lower,input.upper,input.count,fee,input.spacing):['',''];
            if(!profit)throw Error('calculation_unavailable');
            const math=window.__elonBinanceDecimalV1;
            const quantityStatus=!countValid?'count_unavailable':input.margin===''?'margin_required':
              math.compare(math.decimal(input.margin),math.decimal(minimum))<0?'margin_below_minimum':'ready';
            const quantity=quantityStatus==='ready'?calculator.quantity(input.direction,input.margin,input.lower,input.upper,
              market.mark,input.triggerPrice,input.count,input.leverage,String(c.adjustCoef),market.qtyPrecision,market.pricePrecision,
              input.spacing,trailing,market.last,c.windowCount):'';
            if(quantityStatus==='ready' && !decimal(quantity))throw Error('calculation_unavailable');
            economics={profit_min:profit[0],profit_max:profit[1],quantity,quantity_unit:trailing?'USDT':input.symbol.slice(0,-4),quantity_status:quantityStatus};
          }
          await deps.identity(account,h);
          if(ticket!==generation)return;
          if(Date.now()-market.observedAt>120000)throw Error('market_expired');
          current={...base,token,status:'ready',minimum_count:c.minGridCount,maximum_count:maximum,minimum_margin:minimum,
            observed_at:market.observedAt,source:`binance_create_rules_v${version}`,code:maximum<c.minGridCount?'range_too_narrow':input.count===''?'count_required':countValid?'':'count_outside_range',...economics};
        }catch(error){if(ticket===generation){fees=null;current={...base,token,status:'unavailable'};}}
        finally {if(ticket===generation)window.ElonBinanceReference?.postMessage(JSON.stringify({token,request}));}})();
        return true;
      },
      read(token,request,account) {
        if(!current || current.token!==token || current.request!==request || current.account!==account)return null;
        const {token:_,...result}=current;
        if(result.status==='ready' && (Date.now()-result.observed_at>120000 || Date.now()-result.observed_at< -5000))return status(request,account,result.symbol,'expired');
        return result;
      },
      cancel(){generation++;current=null;fees=null;}
    });
  };
})();
