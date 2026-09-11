/* User-confirmed create transport. This module has no MCP or arbitrary request entry point. */
(() => {
  'use strict';
  if (window !== window.top || location.origin !== 'https://www.binance.com' || window.__elonBinanceCreateV1) return;
  const LIST = '/bapi/futures/v2/private/future/grid/query-open-grids';
  const CREATE = '/bapi/futures/v2/private/future/grid/place-grid';
  const INFO = '/bapi/accounts/v1/private/account/get-user-base-info';
  const COEF = '/bapi/futures/v1/public/future/common/grid/coef';
  const DETAIL = '/bapi/futures/v1/private/future/grid/query-grid-detail';
  const UPDATE = '/bapi/futures/v1/private/future/grid/update-grid';
  const CLOSE = '/bapi/futures/v1/private/future/grid/close-grid';
  const INVEST = '/bapi/futures/v1/private/future/grid/update-grid-investment';
  const RANGE = '/bapi/futures/v1/private/future/grid/update-grid-range';
  const fetch = window.fetch;
  let context = null, contextAt = 0, prepared = null, sequence = 0, activeSend = null;
  const consumed = new Set();
  const validId = v => typeof v === 'string' && /^[0-9]{1,20}$/.test(v);
  const scalarId = v => validId(v) ? v : Number.isSafeInteger(v) && v >= 0 ? String(v) : null;
  const safeCode = v => ['response_unrecognized','account_unverified','account_changed','configuration_unavailable',
    'session_context_expired','configuration_changed','detail_unverified','cancelled_before_send',
    'settings_changed','settings_unavailable','not_working','no_change','close_mode_changed','investment_unavailable','range_unavailable','protection_unavailable'].includes(v) ? v : 'network_unavailable';
  const businessCode = v => typeof v === 'string' && (/^[0-9]{1,12}$/.test(v) || v === 'symbolInTwap') ? v : 'business_rejected';
  function keys(v, expected) {
    return v && typeof v === 'object' && !Array.isArray(v) && Object.keys(v).sort().join(',') === expected.sort().join(',');
  }
  function validPayload(p) {
    const required = ['symbol','direction','marginType','gridType','gridLowerLimit','gridUpperLimit','gridInitialValue',
      'leverage','gridCount','cos','cps','orderCurrency'];
    const extra = ['autoInitPos','autoAddMargin','triggerPrice','triggerType','stopLowerLimit','stopUpperLimit','stopTpPnl','stopSlPnl','stopTriggerType','tpslCps',
      'trailingUp','trailingDown','trailingUpLimitPrice','trailingDownLimitPrice','trailingStopUpperLimit','trailingStopLowerLimit'];
    if (!p || typeof p !== 'object' || Array.isArray(p) || !required.every(k => Object.hasOwn(p,k)) ||
      Object.keys(p).some(k => !required.includes(k) && !extra.includes(k))) return false;
    const decimal = v => typeof v === 'string' && /^(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/.test(v) && /[1-9]/.test(v);
    for (const key of ['triggerPrice','stopLowerLimit','stopUpperLimit','stopTpPnl','stopSlPnl','trailingUpLimitPrice','trailingDownLimitPrice']) {
      if (key in p && !decimal(p[key])) return false;
    }
    for (const key of ['autoAddMargin','tpslCps','trailingUp','trailingDown','trailingStopUpperLimit','trailingStopLowerLimit']) {
      if (key in p && typeof p[key] !== 'boolean') return false;
    }
    if (('triggerPrice' in p) !== ('triggerType' in p)) return false;
    if ('triggerType' in p && !['MARK_PRICE','CONTRACT_PRICE'].includes(p.triggerType)) return false;
    const priceStop = 'stopLowerLimit' in p || 'stopUpperLimit' in p;
    const amountStop = 'stopTpPnl' in p || 'stopSlPnl' in p;
    if(priceStop && amountStop) return false;
    const stop = priceStop || amountStop;
    if (stop !== ('stopTriggerType' in p) || (stop && !['MARK_PRICE','CONTRACT_PRICE'].includes(p.stopTriggerType))) return false;
    if ('tpslCps' in p && !stop) return false;
    const trailing = p.trailingUp === true || p.trailingDown === true;
    if (trailing && (typeof p.trailingUp !== 'boolean' || typeof p.trailingDown !== 'boolean' ||
      typeof p.trailingStopUpperLimit !== 'boolean' || typeof p.trailingStopLowerLimit !== 'boolean')) return false;
    if (('trailingUpLimitPrice' in p && !p.trailingUp) || ('trailingDownLimitPrice' in p && !p.trailingDown)) return false;
    return typeof p.symbol === 'string' && /^[A-Z0-9]{1,24}USDT$/.test(p.symbol) &&
      ['LONG','SHORT','NEUTRAL'].includes(p.direction) && ['ISOLATED','CROSSED'].includes(p.marginType) &&
      ['ARITH','GEO'].includes(p.gridType) && ['gridLowerLimit','gridUpperLimit','gridInitialValue'].every(k => decimal(p[k])) &&
      Number.isInteger(p.leverage) && p.leverage >= 1 && p.leverage <= 125 &&
      Number.isInteger(p.gridCount) && p.gridCount >= 2 && p.gridCount <= 10000 &&
      p.cos === true && typeof p.cps === 'boolean' &&
      (p.direction === 'NEUTRAL' ? !('autoInitPos' in p) : typeof p.autoInitPos === 'boolean') &&
      p.orderCurrency === (trailing ? 'QUOTE' : 'BASE');
  }
  function scope(token, id, account) {
    return /^doc_[a-z0-9_]{3,80}$/.test(token) && /^[a-f0-9]{32}$/.test(id) && /^[a-f0-9]{64}$/.test(account);
  }
  function emit(p, kind, values = {}) {
    window.ElonBinanceCreate?.postMessage(JSON.stringify({schema:p.mode === 'manage' ? 'yilong.binance_manage_event.v1' : 'yilong.binance_create_event.v1',
      token:p.token, attempt:p.id, kind, ...values}));
  }
  function capture(url, method, headers) {
    try {
      const u = new URL(url, location.href);
      if (u.origin !== location.origin || u.pathname !== LIST || u.search || method !== 'POST') return;
      const h = new Headers(headers);
      if (['x-http-method-override','x-method-override','x-http-method'].some(k => h.has(k))) {
        context = null; return;
      }
      h.set('content-type','application/json');
      context = h; contextAt = Date.now();
    } catch (_) { context = null; }
  }
  window.fetch = function(input, init) {
    capture(typeof input === 'string' || input instanceof URL ? String(input) : input?.url,
      String(init?.method || input?.method || 'GET').toUpperCase(), init?.headers === undefined ? input?.headers : init.headers);
    return fetch.apply(this, arguments);
  };
  const originalOpen = XMLHttpRequest.prototype.open, originalSend = XMLHttpRequest.prototype.send;
  const originalHeader = XMLHttpRequest.prototype.setRequestHeader, xhr = new WeakMap();
  XMLHttpRequest.prototype.open = function(method,url) {
    xhr.set(this,{method:String(method).toUpperCase(),url:String(url),headers:new Headers()});
    return originalOpen.apply(this,arguments);
  };
  XMLHttpRequest.prototype.setRequestHeader = function(k,v) {
    const result = originalHeader.apply(this,arguments); xhr.get(this)?.headers.append(k,v); return result;
  };
  XMLHttpRequest.prototype.send = function() {
    const t = xhr.get(this); if(t) capture(t.url,t.method,t.headers); return originalSend.apply(this,arguments);
  };
  async function request(path, headers, body) {
    const controller = new AbortController(), timer = setTimeout(() => controller.abort(), 20_000);
    try {
      const response = await fetch.call(window,path,{method:body ? 'POST' : 'GET',headers,
        credentials:'same-origin',redirect:'error',cache:'no-store',signal:controller.signal,
        ...(body ? {body:JSON.stringify(body)} : {})});
      const text = await response.clone().text();
      if(response.status !== 200 || text.length > 262144) throw Error('response_unrecognized');
      return JSON.parse(text);
    } finally { clearTimeout(timer); }
  }
  async function identity(expected, headers) {
    const v = await request(INFO,headers), d = v.data, uid = scalarId(d?.userId);
    if(v.code !== '000000' || v.success !== true || !uid || typeof d.subUser !== 'boolean' ||
      (d.subUser && d.parentUser === true)) throw Error('account_unverified');
    const hash = Array.from(new Uint8Array(await window.crypto.subtle.digest('SHA-256',new TextEncoder().encode(uid))))
      .map(b => b.toString(16).padStart(2,'0')).join('');
    if(hash !== expected) throw Error('account_changed');
    return uid;
  }
  async function coefficient(headers) {
    const v = await request(COEF,headers), n = v.data?.windowCount;
    if(v.code !== '000000' || v.success !== true || !Number.isInteger(n) || n < 1 || n > 10000) throw Error('configuration_unavailable');
    return n;
  }
  function headers() {
    if(!context || Date.now()-contextAt < 0 || Date.now()-contextAt >= 300_000) throw Error('session_context_expired');
    return new Headers(context);
  }
  function clientId() {
    // Same public manual-flow shape and 32-character bound; uniqueness is not a server idempotency promise.
    const entropy = window.crypto.getRandomValues(new Uint32Array(1))[0] % 1000000 + 1;
    return `manual_mm_web_${entropy}${Date.now()}`.slice(0,32);
  }
  window.__elonBinanceCreateV1 = Object.freeze({
    prepare(token,id,account,payload) {
      if(!scope(token,id,account) || !validPayload(payload) || consumed.has(id) || consumed.size >= 16 || !context || activeSend) return false;
      const p = {token,id,account,payload:JSON.parse(JSON.stringify(payload)),seq:++sequence,expires:0};
      prepared = null;
      (async () => {
        try {
          const h = headers(); await identity(account,h); const count = await coefficient(h);
          if(p.seq !== sequence) return;
          p.count = count; p.client = clientId(); p.expires = Date.now()+60_000; prepared = p;
          emit(p,'prepared',{client_id:p.client,window_count:count});
        } catch(e) { if(p.seq === sequence) emit(p,'prepare_failed',{code:safeCode(e?.message)}); }
      })();
      return true;
    },
    cancel() { sequence++; prepared = null; if(activeSend) activeSend.cancelled = true; },
    submit(token,id) {
      const p = prepared;
      if(!p || p.mode || p.token !== token || p.id !== id || consumed.has(id) || Date.now() >= p.expires) return false;
      consumed.add(id); prepared = null; sequence++;
      activeSend = p;
      (async () => {
        let dispatched = false;
        try {
          const h = headers();
          if(await coefficient(h) !== p.count) throw Error('configuration_changed');
          // Identity is the last network check immediately before dispatch, after the potentially slow config read.
          await identity(p.account,h);
          if(p.cancelled) throw Error('cancelled_before_send');
          const body = {...p.payload,clientStrategyId:p.client,
            ...(!p.payload.trailingUp && !p.payload.trailingDown ? {slideWindow:p.payload.gridCount > p.count} : {})};
          dispatched = true;
          const v = await request(CREATE,h,body);
          if(v.success === false && typeof v.code === 'string' && v.code !== '000000') {
            emit(p,'rejected',{code:businessCode(v.code)}); return;
          }
          const d = v.data, id = scalarId(d?.strategyId);
          if(v.success !== true || v.code !== '000000' || !id || d.clientStrategyId !== p.client ||
            typeof d.strategyStatus !== 'string' || !/^[A-Z][A-Z0-9_]{0,63}$/.test(d.strategyStatus)) throw Error('response_unrecognized');
          emit(p,'accepted',{strategy_id:id,client_id:p.client,provider_status:d.strategyStatus});
        } catch(e) { emit(p,dispatched ? 'unknown' : 'not_sent',{code:safeCode(e?.message)}); }
        finally { if(activeSend === p) activeSend = null; }
      })();
      return true;
    },
    detail(token,id,account,strategyId) {
      if(!scope(token,id,account) || !validId(strategyId) || !context) return false;
      const p = {token,id};
      (async () => {
        try {
          const h = headers(), uid = await identity(account,h);
          const v = await request(DETAIL+'?strategyId='+encodeURIComponent(strategyId),h), d = v.data;
          await identity(account,h);
          if(v.code !== '000000' || v.success !== true || scalarId(d?.strategyId) !== strategyId ||
            (d.rootUserId != null && scalarId(d.rootUserId) !== uid) ||
            typeof d.strategyStatus !== 'string' || !/^[A-Z][A-Z0-9_]{0,63}$/.test(d.strategyStatus)) throw Error('detail_unverified');
          emit(p,'detail',{strategy_id:strategyId,provider_status:d.strategyStatus});
        } catch(e) { emit(p,'detail_failed',{code:safeCode(e?.message)}); }
      })();
      return true;
    }
  });
  // Management shares the observed request context, one prepared slot and consumed IDs with creation.
  // Only exact, already observed contracts are supported; no update-then-close chain exists.
  function manageId(id) { return validId(id) && /^[1-9][0-9]*$/.test(id) && Number.isSafeInteger(Number(id)); }
  function fundingSnapshot(d) {
    const decimal=v=>typeof v==='string' && /^-?(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/.test(v);
    const value=typeof d.gridInitialValue==='string'?d.gridInitialValue:Number.isSafeInteger(d.gridInitialValue)?String(d.gridInitialValue):null;
    const adjustment=typeof d.totalAdjustmentAmount==='string'?d.totalAdjustmentAmount:Number.isSafeInteger(d.totalAdjustmentAmount)?String(d.totalAdjustmentAmount):null;
    const leverage=typeof d.initialLeverage==='string' && /^[1-9][0-9]{0,2}$/.test(d.initialLeverage)?Number(d.initialLeverage):d.initialLeverage;
    if(!decimal(value) || value.startsWith('-') || !/[1-9]/.test(value) || !decimal(adjustment) ||
      !Number.isInteger(leverage) || leverage<1 || leverage>125) return null;
    return {initial_value:value,initial_leverage:leverage,total_adjustment:adjustment};
  }
  function validInvestment(amount) {
    if(typeof amount!=='string' || !/^(0|[1-9][0-9]{0,3})(\.[0-9]{1,8})?$/.test(amount) || !/[1-9]/.test(amount))return false;
    const [whole,fraction='']=amount.split('.');
    return BigInt(whole)*100000000n+BigInt(fraction.padEnd(8,'0'))<=200000000000n;
  }
  async function managementSnapshot(p,h) {
    const uid = await identity(p.account,h);
    const v = await request(DETAIL+'?strategyId='+encodeURIComponent(p.strategy),h), d = v.data;
    await identity(p.account,h);
    if(v.code !== '000000' || v.success !== true || scalarId(d?.strategyId) !== p.strategy ||
      (d.rootUserId != null && scalarId(d.rootUserId) !== uid) ||
      typeof d.symbol !== 'string' || !/^[A-Z0-9]{1,24}USDT$/.test(d.symbol) ||
      typeof d.strategyStatus !== 'string' || !/^[A-Z][A-Z0-9_]{0,63}$/.test(d.strategyStatus)) throw Error('detail_unverified');
    const flags=['cps','cos','sharing','trailingStopLowerLimit','trailingStopUpperLimit'];
    if(flags.some(k=>typeof d[k] !== 'boolean')) throw Error('settings_unavailable');
    return {strategy_id:p.strategy,symbol:d.symbol,provider_status:d.strategyStatus,
      ...Object.fromEntries(flags.map(k=>[k,d[k]])),investment:fundingSnapshot(d),range:window.__elonBinanceRangeV1?.snapshot(d) || null,
      protection:window.__elonBinanceProtectionV1?.snapshot(d) || null};
  }
  window.__elonBinanceManageV1 = Object.freeze({
    inspect(token,id,account,strategy) {
      if(!scope(token,id,account) || !manageId(strategy) || !context || activeSend) return false;
      const p={mode:'manage',token,id,account,strategy,seq:++sequence}; prepared=null;
      (async()=>{try {
        const snapshot=await managementSnapshot(p,headers());
        if(p.seq===sequence) emit(p,'detail',{snapshot});
      }catch(e){if(p.seq===sequence) emit(p,'read_failed',{code:safeCode(e?.message)});}})();
      return true;
    },
    prepare(token,id,account,strategy,action,cps) {
      if(!scope(token,id,account) || !manageId(strategy) || !['settings','close'].includes(action) ||
        typeof cps !== 'boolean' || !context || activeSend || consumed.has(id) || consumed.size>=16) return false;
      const p={mode:'manage',token,id,account,strategy,action,cps,seq:++sequence}; prepared=null;
      (async()=>{try {
        const snapshot=await managementSnapshot(p,headers());
        if(snapshot.provider_status!=='WORKING') throw Error('not_working');
        if(action==='settings' && snapshot.cps===cps) throw Error('no_change');
        if(action==='close' && snapshot.cps!==cps) throw Error('close_mode_changed');
        if(p.seq!==sequence) return;
        p.snapshot=snapshot;p.expires=Date.now()+60000;prepared=p;
        emit(p,'prepared',{snapshot,action,cps});
      }catch(e){if(p.seq===sequence) emit(p,'prepare_failed',{code:safeCode(e?.message)});}})();
      return true;
    },
    prepareInvestment(token,id,account,strategy,investmentDelta) {
      if(!scope(token,id,account) || !manageId(strategy) || !validInvestment(investmentDelta) ||
        !context || activeSend || consumed.has(id) || consumed.size>=16)return false;
      const p={mode:'manage',token,id,account,strategy,action:'investment',investmentDelta,seq:++sequence};prepared=null;
      (async()=>{try {
        const snapshot=await managementSnapshot(p,headers());
        if(snapshot.provider_status!=='WORKING')throw Error('not_working');
        if(!snapshot.investment)throw Error('investment_unavailable');
        if(p.seq!==sequence)return;
        p.cps=snapshot.cps;p.snapshot=snapshot;p.expires=Date.now()+60000;prepared=p;
        emit(p,'prepared',{snapshot,action:p.action,cps:p.cps,investment_delta:investmentDelta});
      }catch(e){if(p.seq===sequence)emit(p,'prepare_failed',{code:safeCode(e?.message)});}})();
      return true;
    },
    prepareRange(token,id,account,strategy,input) {
      const contract=window.__elonBinanceRangeV1,rangeDraft=contract?.draft(input);
      if(!scope(token,id,account) || !manageId(strategy) || !rangeDraft || !context || activeSend || consumed.has(id) || consumed.size>=16)return false;
      const p={mode:'manage',token,id,account,strategy,action:'range',rangeDraft,seq:++sequence};prepared=null;
      (async()=>{try {
        const snapshot=await managementSnapshot(p,headers());
        if(snapshot.provider_status!=='WORKING')throw Error('not_working');
        if(!snapshot.range || !snapshot.investment)throw Error('range_unavailable');
        if(!contract.changed(snapshot.range,rangeDraft))throw Error('no_change');
        if(p.seq!==sequence)return;
        p.cps=snapshot.cps;p.snapshot=snapshot;p.expires=Date.now()+60000;prepared=p;
        emit(p,'prepared',{snapshot,action:p.action,cps:p.cps,range_draft:rangeDraft});
      }catch(e){if(p.seq===sequence)emit(p,'prepare_failed',{code:safeCode(e?.message)});}})();
      return true;
    },
    prepareProtection(token,id,account,strategy,input) {
      const contract=window.__elonBinanceProtectionV1,protectionDraft=contract?.draft(input?.draft);
      if(!keys(input,['baseline','draft']) || !scope(token,id,account) || !manageId(strategy) || !protectionDraft || !context || activeSend || consumed.has(id) || consumed.size>=16)return false;
      const baseline=JSON.parse(JSON.stringify(input.baseline));
      const p={mode:'manage',token,id,account,strategy,action:'protection',protectionDraft,seq:++sequence};prepared=null;
      (async()=>{try {
        const snapshot=await managementSnapshot(p,headers());
        if(snapshot.provider_status!=='WORKING')throw Error('not_working');
        if(!snapshot.protection)throw Error('protection_unavailable');
        if(!contract.sameBaseline(snapshot,baseline))throw Error('settings_changed');
        const current=snapshot.protection;
        if(protectionDraft.mode==='CLEAR' && (protectionDraft.stop_type!==current.stop_type || protectionDraft.tpsl_cps!==current.tpsl_cps))throw Error('settings_changed');
        if(['lower','upper','tp','sl','stop_type','tpsl_cps'].every(k=>current[k]===protectionDraft[k]))throw Error('no_change');
        if(p.seq!==sequence)return;
        p.cps=snapshot.cps;p.snapshot=snapshot;p.expires=Date.now()+60000;prepared=p;
        emit(p,'prepared',{snapshot,action:p.action,cps:p.cps,protection_draft:protectionDraft});
      }catch(e){if(p.seq===sequence)emit(p,'prepare_failed',{code:safeCode(e?.message)});}})();
      return true;
    },
    cancel() { window.__elonBinanceCreateV1.cancel(); },
    submit(token,id) {
      const p=prepared;
      if(!p || p.mode!=='manage' || p.token!==token || p.id!==id || consumed.has(id) || activeSend || Date.now()>=p.expires) return false;
      consumed.add(id);prepared=null;sequence++;activeSend=p;
      (async()=>{let dispatched=false;try {
        const h=headers(), latest=await managementSnapshot(p,h);
        if(JSON.stringify(latest)!==JSON.stringify(p.snapshot)) throw Error('settings_changed');
        if(p.cancelled) throw Error('cancelled_before_send');
        const body=p.action==='protection'?window.__elonBinanceProtectionV1.body(p.strategy,latest,p.protectionDraft):
          p.action==='range'?window.__elonBinanceRangeV1.body(p.strategy,latest.symbol,latest.range,p.rangeDraft):
          p.action==='investment'?{strategyId:Number(p.strategy),symbol:latest.symbol,investmentDelta:p.investmentDelta}:
          p.action==='close' ? {strategyId:Number(p.strategy)} : {strategyId:Number(p.strategy),
          symbol:latest.symbol,cps:p.cps,sharing:latest.sharing,
          trailingStopLowerLimit:latest.trailingStopLowerLimit,trailingStopUpperLimit:latest.trailingStopUpperLimit};
        dispatched=true;
        const v=await request(p.action==='range'?RANGE:p.action==='investment'?INVEST:p.action==='close'?CLOSE:UPDATE,h,body),d=v.data;
        if(v.success===false && typeof v.code==='string' && v.code!=='000000') {emit(p,'rejected',{code:businessCode(v.code)});return;}
        if(p.action==='investment' || p.action==='range') {
          if(v.code!=='000000' || v.success!==true || d===false)throw Error('response_unrecognized');
          emit(p,'accepted',{strategy_id:p.strategy,provider_status:''});return;
        }
        if(v.code!=='000000' || v.success!==true || scalarId(d?.strategyId)!==p.strategy ||
          d.updateStatus!=='SUCCESS' || typeof d.strategyStatus!=='string' || !/^[A-Z][A-Z0-9_]{0,63}$/.test(d.strategyStatus)) throw Error('response_unrecognized');
        emit(p,'accepted',{strategy_id:p.strategy,provider_status:d.strategyStatus});
      }catch(e){emit(p,dispatched?'unknown':'not_sent',{code:safeCode(e?.message)});}
      finally{if(activeSend===p)activeSend=null;}})();
      return true;
    }
  });
})();
