/* Versioned, read-only transport. No credentials, arbitrary requests or trading commands. */
(() => {
  'use strict';
  if (window !== window.top || location.origin !== 'https://www.binance.com' || window.__elonBinanceReadV1) return;
  const LIST = '/bapi/futures/v2/private/future/grid/query-open-grids';
  const DETAIL = '/bapi/futures/v1/private/future/grid/query-grid-detail';
  const MAX = 1024 * 1024;
  let token = '', sequence = 0, latestList = 0, current = null;
  const known = new Set();
  function scalar(value, pattern) {
    const text = typeof value === 'string' ? value : Number.isSafeInteger(value) ? String(value) : '';
    if (!pattern.test(text)) throw new Error('invalid_field');
    return text;
  }
  const id = value => scalar(value, /^[0-9]{1,20}$/);
  const optional = (value, pattern) => value == null ? null : scalar(value, pattern);
  function row(value) {
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('invalid_row');
    return {
      id: id(value.strategyId),
      account: value.rootUserId == null ? null : id(value.rootUserId),
      symbol: scalar(value.symbol, /^[A-Z0-9]{1,24}USDT$/),
      status: scalar(value.strategyStatus, /^[A-Z][A-Z0-9_]{0,63}$/),
      direction: optional(value.direction, /^(LONG|SHORT|NEUTRAL)$/),
      spacing: optional(value.gridType, /^(ARITH|GEO)$/),
      lower: optional(value.gridLowerLimit, /^(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/),
      upper: optional(value.gridUpperLimit, /^(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/),
      count: optional(value.gridCount, /^[1-9][0-9]{0,5}$/),
      leverage: optional(value.initialLeverage, /^[1-9][0-9]{0,3}$/),
      profit: optional(value.gridProfit, /^-?(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/),
      created: optional(value.createTime, /^[1-9][0-9]{0,15}$/)
    };
  }
  function emit(event) {
    if (!token) { if (event.kind !== 'detail') current = event; return; }
    window.ElonBinanceRead?.postMessage(JSON.stringify({schema: 'yilong.binance_observation.v1', token, ...event}));
  }
  function fail() { known.clear(); emit({kind: 'unavailable'}); }
  function target(url, method) {
    try {
      const u = new URL(url, location.href);
      if (u.origin !== location.origin) return null;
      if (u.pathname === LIST && method === 'POST') return {kind: 'list', seq: ++sequence};
      if (u.pathname === DETAIL && method === 'GET') return {kind: 'detail', id: id(u.searchParams.get('strategyId'))};
    } catch (_) { /* A non-target website request is not our transport. */ }
    return null;
  }
  function consume(t, status, text) {
    if (!t || (t.kind === 'list' && t.seq < latestList)) return;
    if (t.kind === 'list') latestList = t.seq;
    try {
      if (status !== 200 || typeof text !== 'string' || text.length > MAX) throw new Error('response_failed');
      const body = JSON.parse(text);
      if (body.code !== '000000' || body.success !== true) throw new Error('business_failed');
      if (t.kind === 'list') {
        if (!Array.isArray(body.data) || body.data.length > 500) throw new Error('list_invalid');
        const rows = body.data.map(row);
        if (new Set(rows.map(x => x.id)).size !== rows.length) throw new Error('duplicate');
        known.clear(); rows.forEach(x => known.add(x.id));
        emit({kind: 'list', rows, coverage: 'observed_response_only'});
      } else if (known.has(t.id)) {
        const value = row(body.data);
        if (value.id !== t.id) throw new Error('detail_mismatch');
        emit({kind: 'detail', row: value});
      }
    } catch (_) { fail(); }
  }
  const originalFetch = window.fetch;
  window.fetch = function(input, init) {
    const url = typeof input === 'string' || input instanceof URL ? String(input) : input?.url;
    const method = String(init?.method || input?.method || 'GET').toUpperCase();
    const t = target(url, method);
    return originalFetch.apply(this, arguments).then(response => {
      if (t) response.clone().text().then(text => consume(t, response.status, text), fail);
      return response;
    }, error => { if (t) fail(); throw error; });
  };
  const originalOpen = XMLHttpRequest.prototype.open;
  const originalSend = XMLHttpRequest.prototype.send;
  const targets = new WeakMap();
  XMLHttpRequest.prototype.open = function(method, url) {
    targets.set(this, {method: String(method).toUpperCase(), url: String(url)});
    return originalOpen.apply(this, arguments);
  };
  XMLHttpRequest.prototype.send = function() {
    const data = targets.get(this), t = data && target(data.url, data.method);
    if (t) this.addEventListener('loadend', () => {
      try { consume(t, this.status, this.responseType === 'json' ? JSON.stringify(this.response) : this.responseText); }
      catch (_) { fail(); }
    }, {once: true});
    return originalSend.apply(this, arguments);
  };
  window.__elonBinanceReadV1 = Object.freeze({
    bind(value) {
      if (!/^doc_[a-z0-9_]{3,80}$/.test(value)) return;
      token = value;
      if (current) { const pending = current; current = null; emit(pending); }
    },
    detail(value) {
      if (!known.has(value) || !/^[0-9]{1,20}$/.test(value)) return false;
      window.fetch(DETAIL + '?strategyId=' + encodeURIComponent(value), {method: 'GET', credentials: 'same-origin'}).catch(() => {});
      return true;
    }
  });
})();
