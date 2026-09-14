/* Fixed wallet summary reader. Observed request context stays inside this document. */
(() => {
  'use strict';
  window.__elonBinanceWalletFactoryV1 = ({context, prove, fetch, emit}) => {
    const PATH='/bapi/asset/v2/private/asset-service/wallet/balance?needBalanceDetail=true&quoteAsset=USDT';
    const ID=/^[a-f0-9]{64}$/, DECIMAL=/^-?(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/;
    let epoch=0, pending=null;
    const fail=code=>{throw new Error(code);};
    const digest=async value=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(value))),x=>x.toString(16).padStart(2,'0')).join('');
    function decode(value) {
      if(!value || value.code!=='000000' || value.success!==true)fail('business_failed');
      if(!Array.isArray(value.data) || value.data.length>64)fail('response_invalid');
      const seen=new Set();
      return value.data.map(wallet=>{
        if(!wallet || !/^[A-Z][A-Z0-9_]{0,47}$/.test(wallet.accountType) || typeof wallet.activate!=='boolean' || seen.has(wallet.accountType))fail('response_invalid');
        seen.add(wallet.accountType);
        if(wallet.balance!=null && (typeof wallet.balance!=='string' || !DECIMAL.test(wallet.balance)))fail('response_invalid');
        return {type:wallet.accountType,active:wallet.activate,balance:wallet.balance??null};
      });
    }
    function query(q) {
      if(!q || !ID.test(q.request) || !['identify','read'].includes(q.kind))return false;
      const keys=q.kind==='read'?['account','kind','request']:['kind','request'];
      if(Object.keys(q).sort().join()!==keys.join() || (q.kind==='read'&&!ID.test(q.account)))return false;
      if(pending)return pending.request===q.request;
      const ctx=context();if(!ctx?.headers)return false;
      const mine=++epoch,controller=new AbortController();pending={request:q.request,controller};
      const active=()=>epoch===mine&&!controller.signal.aborted;
      const send=event=>{if(active())emit({schema:'yilong.binance_wallet_observation.v1',request:q.request,...event});};
      const timer=setTimeout(()=>{if(epoch===mine){emit({schema:'yilong.binance_wallet_observation.v1',request:q.request,kind:'error',error:'timeout'});reset();}},30_000);
      (async()=>{
        const before=await prove(ctx.headers);if(!active())return;
        if(!before || !/^[0-9]{1,20}$/.test(before.account) || !['primary','sub','unknown'].includes(before.account_kind))fail('identity_unverified');
        if(q.kind==='identify'){send({kind:'identity',...before});return;}
        if(await digest(before.account)!==q.account)fail('account_changed');
        if(!active())return;
        const response=await fetch(PATH,{method:'GET',headers:ctx.headers,credentials:'same-origin',redirect:'error',cache:'no-store',signal:controller.signal});
        if(response.status!==200)fail(response.status===429?'rate_limited':'http_failed');
        const text=await response.text();if(text.length>1024*1024)fail('response_too_large');
        const wallets=decode(JSON.parse(text));if(!active())return;
        const after=await prove(ctx.headers);if(!after || after.account!==before.account || after.account_kind!==before.account_kind)fail('account_changed');
        send({kind:'wallet',...after,quote_asset:'USDT',wallets});
      })().catch(error=>send({kind:'error',error:['account_changed','identity_unverified','rate_limited','http_failed','business_failed','response_invalid','response_too_large'].includes(error?.message)?error.message:'read_failed'}))
        .finally(()=>{clearTimeout(timer);if(epoch===mine)pending=null;});
      return true;
    }
    function reset(){epoch++;pending?.controller.abort();pending=null;}
    return Object.freeze({query,reset});
  };
})();
