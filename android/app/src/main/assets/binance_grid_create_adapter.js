/* User-confirmed create transport. This module has no MCP or arbitrary request entry point. */
(() => {
  'use strict';
  if (window !== window.top || location.origin !== 'https://www.binance.com' || window.__elonBinanceCreateV1) return;
  const LIST = '/bapi/futures/v2/private/future/grid/query-open-grids';
  const CREATE = '/bapi/futures/v2/private/future/grid/place-grid';
  const INFO = '/bapi/accounts/v1/private/account/get-user-base-info';
  const COEF = '/bapi/futures/v1/public/future/common/grid/coef';
  const DETAIL = '/bapi/futures/v1/private/future/grid/query-grid-detail';
  const fetch = window.fetch;
  let context = null, contextAt = 0, prepared = null, sequence = 0, activeSend = null;
  const consumed = new Set();
  const validId = v => typeof v === 'string' && /^[0-9]{1,20}$/.test(v);
  const scalarId = v => validId(v) ? v : Number.isSafeInteger(v) && v >= 0 ? String(v) : null;
  const safeCode = v => ['response_unrecognized','account_unverified','account_changed','configuration_unavailable',
    'session_context_expired','configuration_changed','detail_unverified','cancelled_before_send'].includes(v) ? v : 'network_unavailable';
  const businessCode = v => typeof v === 'string' && (/^[0-9]{1,12}$/.test(v) || v === 'symbolInTwap') ? v : 'business_rejected';
  function keys(v, expected) {
    return v && typeof v === 'object' && !Array.isArray(v) && Object.keys(v).sort().join(',') === expected.sort().join(',');
  }
  function validPayload(p) {
    if (!keys(p, ['symbol','direction','marginType','gridType','gridLowerLimit','gridUpperLimit','gridInitialValue',
      'leverage','gridCount','cos','cps','autoInitPos','orderCurrency'])) return false;
    const decimal = v => typeof v === 'string' && /^(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/.test(v) && /[1-9]/.test(v);
    return typeof p.symbol === 'string' && /^[A-Z0-9]{1,24}USDT$/.test(p.symbol) &&
      ['LONG','SHORT'].includes(p.direction) && ['ISOLATED','CROSSED'].includes(p.marginType) &&
      ['ARITH','GEO'].includes(p.gridType) && ['gridLowerLimit','gridUpperLimit','gridInitialValue'].every(k => decimal(p[k])) &&
      Number.isInteger(p.leverage) && p.leverage >= 1 && p.leverage <= 125 &&
      Number.isInteger(p.gridCount) && p.gridCount >= 2 && p.gridCount <= 10000 &&
      p.cos === true && typeof p.cps === 'boolean' && typeof p.autoInitPos === 'boolean' && p.orderCurrency === 'BASE';
  }
  function scope(token, id, account) {
    return /^doc_[a-z0-9_]{3,80}$/.test(token) && /^[a-f0-9]{32}$/.test(id) && /^[a-f0-9]{64}$/.test(account);
  }
  function emit(p, kind, values = {}) {
    window.ElonBinanceCreate?.postMessage(JSON.stringify({schema:'yilong.binance_create_event.v1',
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
      if(!scope(token,id,account) || !validPayload(payload) || consumed.has(id) || consumed.size >= 16 || !context) return false;
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
      if(!p || p.token !== token || p.id !== id || consumed.has(id) || Date.now() >= p.expires) return false;
      consumed.add(id); prepared = null; sequence++;
      activeSend = p;
      (async () => {
        let dispatched = false;
        try {
          const h = headers(); await identity(p.account,h);
          if(await coefficient(h) !== p.count) throw Error('configuration_changed');
          if(p.cancelled) throw Error('cancelled_before_send');
          const body = {...p.payload,clientStrategyId:p.client,slideWindow:p.payload.gridCount > p.count};
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
})();
