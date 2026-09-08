/* Versioned, read-only transport. No credentials, arbitrary requests or trading commands. */
(() => {
  'use strict';
  if (window !== window.top || location.origin !== 'https://www.binance.com' || window.__elonBinanceReadV1) return;
  const LIST = '/bapi/futures/v2/private/future/grid/query-open-grids';
  const DETAIL = '/bapi/futures/v1/private/future/grid/query-grid-detail';
  // Observed on the authenticated Win subaccount page; never reuse the parent UID as this UID.
  const IDENTITY = '/bapi/accounts/v1/private/account/get-user-base-info';
  const MAX = 1024 * 1024;
  let token = '', sequence = 0, latestList = 0, listFloor = 0, listEpoch = 0, current = null;
  let identitySequence = 0, latestIdentity = 0, account = null;
  const known = new Set();
  const details = new Map();
  const originalFetch = window.fetch;
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
  function relevant(t) {
    if (!t) return false;
    if (t.kind === 'identity') return t.seq === latestIdentity;
    if (t.kind === 'list') return t.seq === latestList && t.seq >= listFloor;
    return t.epoch === listEpoch && t.account === account?.account && known.has(t.id) && details.get(t.id) === t.seq;
  }
  function fail(t) {
    if (!relevant(t)) return;
    known.clear(); details.clear(); account = null; listEpoch++; listFloor = latestList + 1;
    emit({kind: 'unavailable'});
  }
  function body(status, text) {
    if (status !== 200 || typeof text !== 'string' || text.length > MAX) throw new Error('response_failed');
    const value = JSON.parse(text);
    if (value.code !== '000000' || value.success !== true) throw new Error('business_failed');
    return value.data;
  }
  function applyIdentity(seq, data) {
    if (seq !== latestIdentity) return null;
    if (!data || typeof data.subUser !== 'boolean' || (data.subUser && data.parentUser === true)) throw new Error('identity_unverified');
    const next = {account: id(data.userId), account_kind: data.subUser ? 'sub' : data.parentUser === true ? 'primary' : 'unknown'};
    if (account?.account !== next.account || account?.account_kind !== next.account_kind) {
      known.clear(); details.clear(); listEpoch++;
    }
    account = next;
    emit({kind: 'identity', ...next});
    return next;
  }
  async function proveIdentity() {
    const seq = ++identitySequence; latestIdentity = seq;
    const controller = new AbortController(), timeout = setTimeout(() => controller.abort(), 10_000);
    try {
      const response = await originalFetch.call(window, IDENTITY, {method: 'GET', credentials: 'same-origin', signal: controller.signal});
      const text = await response.clone().text();
      if (seq !== latestIdentity) return null;
      return applyIdentity(seq, body(response.status, text));
    } finally { clearTimeout(timeout); }
  }
  function target(url, method) {
    try {
      const u = new URL(url, location.href);
      if (u.origin !== location.origin) return null;
      if (u.pathname === IDENTITY && method === 'GET') {
        latestIdentity = ++identitySequence; return {kind: 'identity', seq: latestIdentity};
      }
      if (u.pathname === LIST && method === 'POST') {
        latestList = ++sequence; return {kind: 'list', seq: latestList};
      }
      if (u.pathname === DETAIL && method === 'GET') {
        const key = id(u.searchParams.get('strategyId')), seq = ++sequence;
        details.set(key, seq);
        return {kind: 'detail', id: key, seq, epoch: listEpoch, account: account?.account};
      }
    } catch (_) { /* A non-target website request is not our transport. */ }
    return null;
  }
  async function consume(t, status, text) {
    if (!relevant(t)) return;
      const data = body(status, text);
      if (t.kind === 'identity') { applyIdentity(t.seq, data); return; }
      if (t.kind === 'list') {
        if (!Array.isArray(data) || data.length > 500) throw new Error('list_invalid');
        const rows = data.map(row);
        if (new Set(rows.map(x => x.id)).size !== rows.length) throw new Error('duplicate');
        const proof = await proveIdentity();
        if (!proof || !relevant(t)) return;
        if (rows.some(x => x.account !== proof.account)) throw new Error('account_mismatch');
        listEpoch++; details.clear(); known.clear(); rows.forEach(x => known.add(x.id));
        emit({kind: 'list', ...proof, rows, coverage: 'observed_response_only'});
      } else {
        const value = row(data);
        if (value.id !== t.id) throw new Error('detail_mismatch');
        const proof = await proveIdentity();
        if (!proof || !relevant(t)) return;
        if (value.account != null && value.account !== proof.account) throw new Error('account_mismatch');
        emit({kind: 'detail', ...proof, row: value});
      }
  }
  window.fetch = function(input, init) {
    const url = typeof input === 'string' || input instanceof URL ? String(input) : input?.url;
    const method = String(init?.method || input?.method || 'GET').toUpperCase();
    const t = target(url, method);
    return originalFetch.apply(this, arguments).then(response => {
      if (t) response.clone().text().then(text => consume(t, response.status, text)).catch(() => fail(t));
      return response;
    }, error => { fail(t); throw error; });
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
      try { consume(t, this.status, this.responseType === 'json' ? JSON.stringify(this.response) : this.responseText).catch(() => fail(t)); }
      catch (_) { fail(t); }
    }, {once: true});
    return originalSend.apply(this, arguments);
  };
  window.__elonBinanceReadV1 = Object.freeze({
    bind(value) {
      if (!/^doc_[a-z0-9_]{3,80}$/.test(value)) return;
      token = value;
      if (current) { const pending = current; current = null; emit(pending); }
      return true;
    },
    detail(value) {
      if (!token || !known.has(value) || !/^[0-9]{1,20}$/.test(value)) return false;
      window.fetch(DETAIL + '?strategyId=' + encodeURIComponent(value), {method: 'GET', credentials: 'same-origin'}).catch(() => {});
      return true;
    }
  });
})();
