/* Account-scoped, read-only funds used by the website's UM grid creation flow. */
(() => {
  'use strict';
  const schema='yilong.binance_create_funds.v1';
  const id=v=>typeof v==='string' && /^[a-f0-9]{64}$/.test(v);
  function amount(value) {
    if(Number.isSafeInteger(value) && value>=0)value=String(value);
    if(typeof value!=='string' || !/^(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/.test(value))throw Error('balance_invalid');
    return value;
  }
  function data(value) {
    if(value?.success!==true || value.code!=='000000')throw Error('balance_unavailable');
    return value.data;
  }
  window.__elonBinanceCreateFundsFactoryV1=deps=>{
    let generation=0,current=null;
    const base=(request,account,status)=>({schema,request,account,status});
    return Object.freeze({
      start(token,request,account) {
        if(!/^doc_[a-z0-9_]{3,80}$/.test(token) || !id(request) || !id(account))return false;
        const ticket=++generation,started=Date.now();current={...base(request,account,'pending'),token};
        const alive=()=>{if(ticket!==generation || Date.now()-started<0 || Date.now()-started>30000)throw Error('expired');};
        (async()=>{try {
          const headers=deps.headers();await deps.identity(account,headers);alive();
          const mode=data(await deps.mode(headers));alive();
          if(typeof mode?.enable!=='boolean')throw Error('account_mode_unavailable');
          let available,source;
          if(mode.enable) {
            const rows=data(await deps.spot(headers));alive();
            if(!Array.isArray(rows) || rows.length>5000)throw Error('balance_invalid');
            const selected=rows.filter(row=>row?.asset==='USDT');
            if(selected.length!==1)throw Error('balance_invalid');
            available=amount(selected[0].free);source='portfolio_spot_free';
          } else {
            available=amount(data(await deps.futures(headers)));alive();source='futures_transferable';
          }
          await deps.identity(account,headers);alive();
          current={...base(request,account,'ready'),token,asset:'USDT',available,source,observed_at:Date.now()};
        }catch(_){if(ticket===generation)current={...base(request,account,'unavailable'),token};}})();
        return true;
      },
      read(token,request,account) {
        if(!current || current.token!==token || current.request!==request || current.account!==account)return null;
        const {token:_,...result}=current;
        if(result.status==='ready' && (Date.now()-result.observed_at>60000 || Date.now()-result.observed_at< -5000))return base(request,account,'expired');
        return result;
      },
      cancel(){generation++;current=null;}
    });
  };
})();
