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
  let detailHeaders = null;
  let listContext = null, refreshing = false;
  const known = new Set();
  const details = new Map();
  const originalFetch = window.fetch;
  const reports = window.__elonBinanceReportsFactoryV1?.({
    fetch: (url, init) => originalFetch.call(window, url, init),
    known: value => known.has(value),
    context: () => detailHeaders && account ? {headers: detailHeaders, ...account} : null,
    prove: headers => proveIdentity(headers),
    emit: event => { if (token) window.ElonBinanceRead?.postMessage(JSON.stringify({...event, token})); }
  });
  delete window.__elonBinanceReportsFactoryV1;
  const diagnostic = window.__elonBinanceDiagnosticsV1;
  function scalar(value, pattern) {
    const text = typeof value === 'string' ? value : Number.isSafeInteger(value) ? String(value) : '';
    if (!pattern.test(text)) throw new Error('invalid_field');
    return text;
  }
  const id = value => scalar(value, /^[0-9]{1,20}$/);
  const optional = (value, pattern) => value == null ? null : scalar(value, pattern);
  function metrics(value) {
    const decimal = /^-?(0|[1-9][0-9]{0,29})(\.[0-9]{1,20})?$/;
    const mapping = {initialNotional:'gridInitialValue',investment:'strategyAmount',matchedPnl:'matchedPnl',
      fundingFee:'fundingFee',fee:'fee',adjustmentAmount:'totalAdjustmentAmount',perGridQty:'perGridQty',
      perGridQuoteQty:'perGridQuoteQty',triggerPrice:'triggerPrice',stopUpper:'stopUpperLimit',stopLower:'stopLowerLimit',
      stopTpPnl:'stopTpPnl',stopSlPnl:'stopSlPnl',trailingUpPrice:'trailingUpLimitPrice',trailingDownPrice:'trailingDownLimitPrice'};
    const result = {};
    for (const [key, source] of Object.entries(mapping)) result[key] = optional(value[source], decimal);
    for (const [key, source] of Object.entries({closeOnStop:'cps',autoAddMargin:'autoAddMargin',trailingUp:'trailingUp',trailingDown:'trailingDown'})) {
      if (value[source] != null && typeof value[source] !== 'boolean') throw new Error('invalid_flag');
      result[key] = value[source] ?? null;
    }
    result.matchedCount = optional(value.matchedCount, /^(0|[1-9][0-9]{0,15})$/);
    result.ended = optional(value.endTime, /^(0|[1-9][0-9]{0,15})$/);
    result.marginType = optional(value.marginType, /^(CROSSED|ISOLATED)$/);
    result.orderCurrency = optional(value.orderCurrency, /^(BASE|QUOTE)$/);
    return result;
  }
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
      created: optional(value.createTime ?? (value.bookTime > 0 ? value.bookTime : null), /^[1-9][0-9]{0,15}$/),
      metrics: metrics(value)
    };
  }
  function emit(event) {
    if (!token) {
      // An identity refresh is not a newer list. Keep the verified pending list for this exact account.
      const samePendingList = event.kind === 'identity' && current?.kind === 'list' &&
        current.account === event.account && current.account_kind === event.account_kind;
      if (event.kind !== 'detail' && !samePendingList) current = event;
      return;
    }
    window.ElonBinanceRead?.postMessage(JSON.stringify({schema: 'yilong.binance_observation.v1', token, ...event}));
  }
  function relevant(t) {
    if (!t) return false;
    if (t.kind === 'identity') return t.seq === latestIdentity;
    if (t.kind === 'list') return t.seq === latestList && t.seq >= listFloor;
    return t.epoch === listEpoch && t.account === account?.account && known.has(t.id) && details.get(t.id) === t.seq;
  }
  function fail(t, error) {
    if (!relevant(t)) return;
    diagnostic?.failure(t.kind, error?.message);
    known.clear(); details.clear(); reports?.reset(); detailHeaders = null; listContext = null; account = null; listEpoch++; listFloor = latestList + 1;
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
      known.clear(); details.clear(); reports?.reset(); detailHeaders = null; listContext = null; listEpoch++;
    }
    account = next;
    emit({kind: 'identity', ...next});
    return next;
  }
  function requestHeaders(input, init) {
    try {
      const headers = new Headers(init?.headers === undefined ? input?.headers : init.headers);
      // Never turn a fixed read into a method-override request.
      if (['x-http-method-override', 'x-method-override', 'x-http-method'].some(key => headers.has(key))) return null;
      return headers;
    } catch (_) { return null; }
  }
  async function proveIdentity(headers) {
    if (!headers) throw new Error('identity_unverified');
    const seq = ++identitySequence; latestIdentity = seq;
    const controller = new AbortController(), timeout = setTimeout(() => controller.abort(), 10_000);
    try {
      diagnostic?.request('identity');
      const response = await originalFetch.call(window, IDENTITY, {method: 'GET', headers,
        credentials: 'same-origin', redirect: 'error', cache: 'no-store', signal: controller.signal});
      diagnostic?.response('identity', response.status);
      const text = await response.clone().text();
      if (seq !== latestIdentity) return null;
      return applyIdentity(seq, body(response.status, text));
    } catch (error) {
      if (['response_failed', 'business_failed'].includes(error?.message)) throw new Error('identity_' + error.message);
      throw error;
    } finally { clearTimeout(timeout); }
  }
  function target(url, method) {
    try {
      const u = new URL(url, location.href);
      if (u.origin !== location.origin) return null;
      // Known from source research, but its response contract is not accepted as v2 evidence.
      if (u.pathname === '/bapi/futures/v1/private/future/grid/query-open-grids' && method === 'GET') {
        diagnostic?.request('legacy_list'); return null;
      }
      if (u.pathname === IDENTITY && !u.search && method === 'GET') {
        diagnostic?.request('identity');
        latestIdentity = ++identitySequence; return {kind: 'identity', seq: latestIdentity};
      }
      if (u.pathname === LIST && method === 'POST') {
        diagnostic?.request('list');
        latestList = ++sequence; return {kind: 'list', seq: latestList};
      }
      if (u.pathname === DETAIL && method === 'GET') {
        const key = id(u.searchParams.get('strategyId')), seq = ++sequence;
        diagnostic?.request('detail');
        details.set(key, seq);
        return {kind: 'detail', id: key, seq, epoch: listEpoch, account: account?.account};
      }
    } catch (_) { /* A non-target website request is not our transport. */ }
    return null;
  }
  async function consume(t, status, text) {
    diagnostic?.response(t.kind, status);
    if (!relevant(t)) return;
      const data = body(status, text);
      if (t.kind === 'identity') { applyIdentity(t.seq, data); return; }
      if (t.kind === 'list') {
        if (!Array.isArray(data) || data.length > 500) throw new Error('list_invalid');
        const rows = data.map(row);
        if (new Set(rows.map(x => x.id)).size !== rows.length) throw new Error('duplicate');
        const proof = await proveIdentity(t.headers);
        if (!proof || !relevant(t)) return;
        if (rows.some(x => x.account !== proof.account)) throw new Error('account_mismatch');
        listEpoch++; details.clear(); known.clear(); rows.forEach(x => known.add(x.id));
        detailHeaders = t.headers;
        // Replay only a body actually observed on this exact read endpoint. It never leaves the page.
        if (t.capturedBody) listContext = {headers: t.headers, body: t.requestBody, account: proof.account};
        emit({kind: 'list', ...proof, rows, coverage: 'observed_response_only'});
      } else {
        const value = row(data);
        if (value.id !== t.id) throw new Error('detail_mismatch');
        const proof = await proveIdentity(t.headers);
        if (!proof || !relevant(t)) return;
        if (value.account != null && value.account !== proof.account) throw new Error('account_mismatch');
        emit({kind: 'detail', ...proof, row: value});
      }
  }
  window.fetch = function(input, init) {
    const url = typeof input === 'string' || input instanceof URL ? String(input) : input?.url;
    const method = String(init?.method || input?.method || 'GET').toUpperCase();
    const t = target(url, method);
    // Retain only this observed request's context inside the page; never send it through the bridge.
    if (t) t.headers = requestHeaders(input, init);
    if (t?.kind === 'list' && (typeof input === 'string' || input instanceof URL)) captureBody(t, init?.body);
    return originalFetch.apply(this, arguments).then(response => {
      if (t) response.clone().text().then(text => consume(t, response.status, text)).catch(error => fail(t, error));
      return response;
    }, error => { fail(t, error); throw error; });
  };
  const originalOpen = XMLHttpRequest.prototype.open;
  const originalSend = XMLHttpRequest.prototype.send;
  const originalSetRequestHeader = XMLHttpRequest.prototype.setRequestHeader;
  const targets = new WeakMap();
  XMLHttpRequest.prototype.open = function(method, url) {
    targets.set(this, {method: String(method).toUpperCase(), url: String(url), headers: new Headers()});
    return originalOpen.apply(this, arguments);
  };
  XMLHttpRequest.prototype.setRequestHeader = function(name, value) {
    const result = originalSetRequestHeader.apply(this, arguments);
    targets.get(this)?.headers.append(name, value);
    return result;
  };
  XMLHttpRequest.prototype.send = function() {
    const data = targets.get(this), t = data && target(data.url, data.method);
    if (t) t.headers = requestHeaders(null, data);
    if (t?.kind === 'list') captureBody(t, arguments[0]);
    if (t) this.addEventListener('loadend', () => {
      try { consume(t, this.status, this.responseType === 'json' ? JSON.stringify(this.response) : this.responseText).catch(error => fail(t, error)); }
      catch (error) { fail(t, error); }
    }, {once: true});
    return originalSend.apply(this, arguments);
  };
  function captureBody(t, value) {
    if (value == null) { t.capturedBody = true; t.requestBody = undefined; return; }
    if (typeof value !== 'string' || value.length > 4096) return;
    try {
      const parsed = JSON.parse(value);
      if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return;
      t.capturedBody = true; t.requestBody = value;
    } catch (_) { /* Unknown encoding cannot be replayed. */ }
  }
  window.__elonBinanceReadV1 = Object.freeze({
    report(query) { return !!token && reports?.query(query) === true; },
    bind(value) {
      if (!/^doc_[a-z0-9_]{3,80}$/.test(value)) return;
      token = value;
      if (current) { const pending = current; current = null; emit(pending); }
      return true;
    },
    refresh() {
      if (!token || !listContext || listContext.account !== account?.account) return false;
      if (refreshing) return true;
      refreshing = true;
      const controller = new AbortController(), timer = setTimeout(() => controller.abort(), 15000);
      window.fetch(LIST, {method:'POST', headers:listContext.headers, body:listContext.body,
        credentials:'same-origin',redirect:'error',cache:'no-store',signal:controller.signal})
        .catch(() => {}).finally(() => { clearTimeout(timer); refreshing = false; });
      return true;
    },
    detail(value) {
      if (!token || !detailHeaders || !known.has(value) || !/^[0-9]{1,20}$/.test(value)) return false;
      window.fetch(DETAIL + '?strategyId=' + encodeURIComponent(value), {method: 'GET', headers: detailHeaders,
        credentials: 'same-origin', redirect: 'error', cache: 'no-store'}).catch(() => {});
      return true;
    }
  });
})();
