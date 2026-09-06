'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const directory = path.join(__dirname, '../tools/binance-grid-observer');
const sanitizerSource = fs.readFileSync(path.join(directory, 'sanitize.js'), 'utf8');
const observerSource = fs.readFileSync(path.join(directory, 'observer.js'), 'utf8');
const S = require(path.join(directory, 'sanitize.js'));
const ROUTE = '/bapi/futures/v1/private/future-grid/query';
const SESSION = 'synthetic_session_00000001';
const clean = value => JSON.parse(JSON.stringify(value));
const deferred = () => { let resolve, reject; const promise = new Promise((a, b) => { resolve = a; reject = b; }); return { promise, resolve, reject }; };
async function flush() { for (let i = 0; i < 15; i++) await Promise.resolve(); }

test('browser injection preserves a page-owned CommonJS module export', () => {
  const pageExport = { existingLibrary: true };
  const context = vm.createContext({ URL, TextEncoder, window: {}, module: { exports: pageExport } });
  vm.runInContext(sanitizerSource, context);
  assert.equal(context.module.exports, pageExport);
  assert.equal(typeof context.BinanceGridSanitizer.sanitizeObservation, 'function');
});

function fixture(options = {}) {
  const calls = [], requests = [], messages = [], timers = new Map(), events = new Map();
  let clock = 1000, timerId = 0, shapeCalls = 0;
  const location = new URL(options.page || 'https://www.binance.com/en/futures/BTCUSDT');
  const document = { baseURI: options.base || location.href };
  class FakeXHR {
    constructor() { this.listeners = new Map(); this.responseType = ''; this.status = 200; this.responseText = '{"code":"0","data":{"symbol":"private-canary"}}'; this.headerCalls = []; }
    open() {
      calls.push({ kind: 'open', receiver: this, args: [...arguments] });
      if (this.openError) throw this.openError;
      this.responseURL = new URL(String(arguments[1]), document.baseURI).href;
      return 'open-result';
    }
    send() {
      calls.push({ kind: 'send', receiver: this, args: [...arguments] });
      if (this.sendError) throw this.sendError;
      if (this.synchronous) this.complete();
      return 'send-result';
    }
    addEventListener(type, callback) { if (!this.listeners.has(type)) this.listeners.set(type, new Set()); this.listeners.get(type).add(callback); }
    removeEventListener(type, callback) { this.listeners.get(type)?.delete(callback); }
    complete() { for (const callback of [...(this.listeners.get('loadend') || [])]) callback.call(this); }
    getResponseHeader(name) { this.headerCalls.push(name); return 'application/json'; }
  }
  const context = {
    URL, Request, Uint8Array, TextEncoder, TextDecoder, Promise, console, location, document,
    Date: class extends Date { static now() { return clock; } },
    setTimeout(fn, ms) { const id = ++timerId; timers.set(id, { at: clock + ms, fn }); return id; },
    clearTimeout(id) { timers.delete(id); },
    addEventListener(name, fn) { if (!events.has(name)) events.set(name, []); events.get(name).push(fn); },
    postMessage(message, origin) { messages.push({ message: clean(message), origin }); },
    fetch: function () {
      calls.push({ kind: 'fetch', receiver: this, args: [...arguments] });
      if (options.fetchError) throw options.fetchError;
      if (options.coerceUrl) String(arguments[0]);
      const request = deferred(); requests.push(request); return request.promise;
    },
    XMLHttpRequest: FakeXHR,
    history: { pushState(_state, _title, url) { location.href = new URL(url, location.href).href; }, replaceState(_state, _title, url) { location.href = new URL(url, location.href).href; } },
  };
  context.top = options.subframe ? {} : context;
  context.window = context;
  vm.createContext(context);
  vm.runInContext(sanitizerSource, context);
  const actualSanitizer = context.BinanceGridSanitizer;
  context.BinanceGridSanitizer = Object.freeze({ ...actualSanitizer,
    shapeFromJson(text) { shapeCalls++; return actualSanitizer.shapeFromJson(text); } });
  vm.runInContext(observerSource, context);
  const api = context.__binanceGridObserverV1;
  function advance(ms) {
    clock += ms;
    for (const [id, timer] of [...timers]) { if (timer.at <= clock && timers.delete(id)) timer.fn(); }
  }
  function dispatch(name) { for (const fn of events.get(name) || []) fn(); }
  return { context, api, calls, requests, messages, advance, dispatch, shapes: () => shapeCalls, stop: () => api.stop() };
}

function response(text = '{"code":"0","data":{"symbol":"private-canary","amount":"912.45"}}', options = {}) {
  const counts = { clones: 0, originalReads: 0, cancels: 0, headers: [] };
  const gate = deferred();
  let index = 0;
  const chunks = options.chunks || [new TextEncoder().encode(text)];
  const value = {
    status: options.status || 200,
    url: options.url || 'https://www.binance.com' + ROUTE,
    headers: { get(key) { counts.headers.push(key); return options.type || 'application/json'; } },
    text() { counts.originalReads++; throw new Error('original body read'); },
    clone() {
      counts.clones++;
      if (options.cloneError) throw new Error('clone-failed-private-canary');
      return { body: { getReader() { return {
        read() { return options.hang ? gate.promise : Promise.resolve(index < chunks.length ? { done: false, value: chunks[index++] } : { done: true }); },
        cancel() { counts.cancels++; return Promise.resolve(); },
      }; } } };
    },
  };
  return { value, counts, gate };
}

function observation(responseShape = S.shapeOf({ data: { symbol: 'never-export-this' } })) {
  return { schema_version: S.SCHEMA_VERSION, method: 'POST', path: ROUTE, status: 200,
    requestShape: null, responseShape, outcome: 'json' };
}

test('static path templates discard dynamic values, queries and excluded operations', () => {
  const path = S.normalizePath('https://www.binance.com/bapi/futures/v1/private/grid/private-canary/query?token=private-canary#private-canary');
  assert.equal(path, '/bapi/futures/v1/private/grid/{segment}/query');
  assert.equal(S.normalizePath(path), path);
  for (const input of ['https://evil.test/grid/query', '/api/login/grid', '/grid/withdraw', '/grid/wallet',
    '/grid/recommendations', '/grid/leaderboard', '/strategy/copy', '/grid/marketplace', '/grid/follow', '/api/spot/list']) {
    assert.equal(S.normalizePath(input), null, input);
  }
});

test('shape removes all scalar values and unknown dynamic keys and is canonical', () => {
  const shape = S.shapeOf({ data: { symbol: 'private-canary', amount: 912.45, 'private-canary-key': { token: 'private-canary' } }, code: '0' });
  const json = JSON.stringify(shape);
  assert(!json.includes('private-canary'));
  assert(!json.includes('912.45'));
  assert.equal(shape.fields.data.unknownFields, true);
  assert.equal(shape.fields.data.fields.symbol.type, 'string');
  assert.equal(JSON.stringify(S.shapeOf({ code: '0', data: true })), JSON.stringify(S.shapeOf({ data: false, code: 'anything' })));
  assert.equal(S.shapeOf([1, 2, 3, 4]).items.length, 3);
});

test('bounded shape traversal remains a valid observation at node/depth limits', () => {
  let nested = { code: 'private-canary' };
  for (let i = 0; i < 20; i++) nested = { data: [nested, nested, nested], rows: [nested, nested, nested] };
  const shape = S.shapeOf(nested);
  const sanitized = S.sanitizeObservation(observation(shape));
  assert(sanitized);
  assert(Buffer.byteLength(JSON.stringify(sanitized)) <= S.limits.observationBytes);
  assert(JSON.stringify(shape).includes('truncated'));
  assert.equal(S.shapeFromJson('x'.repeat(S.limits.bodyBytes + 1)), null);
  assert.equal(S.shapeFromJson('{malformed-private-canary}'), null);
});

test('worker sanitizer strictly reconstructs schema without page-controlled extra values', () => {
  const good = observation();
  assert(S.sanitizeObservation(good));
  for (const bad of [
    { ...good, cookie: 'private-canary' }, { ...good, path: '/grid/private-canary' },
    { ...good, status: '200' }, { ...good, method: 'private-canary' },
    { ...good, responseShape: { type: 'string', value: 'private-canary' } },
    { ...good, responseShape: { type: 'object', fields: { 'private-canary': { type: 'string' } }, unknownFields: true } },
    { ...good, schema_version: 'future' }, { ...good, outcome: 'network_error' },
  ]) assert.equal(S.sanitizeObservation(bad), null);
  let reads = 0;
  const accessor = { ...good };
  Object.defineProperty(accessor, 'status', { get() { reads++; return 200; } });
  assert.equal(S.sanitizeObservation(accessor), null);
  assert.equal(reads, 0);
});

test('activation permits contract/grid pages only in the same-origin top frame', () => {
  for (const page of ['https://www.binance.com/zh-CN/trading-bots/futures/grid', 'https://www.binance.com/en/futures/BTCUSDT']) {
    const h = fixture({ page }); assert.equal(h.api.start(SESSION).active, true); h.stop();
  }
  for (const options of [{ page: 'https://www.binance.com/en/trading-bots/spot/grid' },
    { page: 'https://evil.test/en/futures/BTCUSDT' }, { subframe: true }]) {
    const h = fixture(options); assert.equal(h.api.start(SESSION).active, false);
  }
});

test('fetch returns the original promise and response and preserves arguments/receiver', async () => {
  const h = fixture(); h.api.start(SESSION);
  const receiver = { caller: 'original' };
  const init = { method: 'POST', body: '{"symbol":"private-canary","signature":"private-canary"}' };
  const returned = h.context.fetch.call(receiver, ROUTE, init);
  assert.equal(returned, h.requests[0].promise);
  assert.equal(h.calls[0].receiver, receiver);
  assert.equal(h.calls[0].args[1], init);
  const reply = response(); h.requests[0].resolve(reply.value);
  assert.equal(await returned, reply.value); await flush();
  assert.equal(reply.counts.originalReads, 0);
  assert.equal(reply.counts.clones, 1);
  assert.deepEqual(reply.counts.headers, ['content-type']);
  assert.equal(h.messages.length, 1);
  assert.equal(h.messages[0].origin, S.ORIGIN);
  assert(!JSON.stringify(h.messages).includes('private-canary'));
  assert.equal(h.messages[0].message.observation.requestShape.unknownFields, true);
  h.stop();
});

test('fetch synchronous exceptions and rejection objects are unchanged', async () => {
  const error = new Error('private-canary');
  const throwing = fixture({ fetchError: error }); throwing.api.start(SESSION);
  assert.throws(() => throwing.context.fetch(ROUTE), value => value === error);
  assert.equal(throwing.messages.length, 0); throwing.stop();
  const h = fixture(); h.api.start(SESSION);
  const promise = h.context.fetch(ROUTE);
  h.requests[0].reject(error);
  await assert.rejects(promise, value => value === error); await flush();
  assert.equal(h.messages[0].message.observation.outcome, 'network_error');
  assert(!JSON.stringify(h.messages).includes('private-canary')); h.stop();
});

test('stopped/excluded requests do not inspect bodies or coerce custom URLs twice', () => {
  const h = fixture({ coerceUrl: true });
  h.context.fetch(ROUTE, { body: '{"symbol":"private-canary"}' });
  h.api.start(SESSION);
  h.context.fetch('/api/auth/login', { body: '{"password":"private-canary"}' });
  const xhr = new h.context.XMLHttpRequest(); xhr.open('POST', '/api/auth/login'); xhr.send('{"password":"private-canary"}');
  let conversions = 0;
  const url = { toString() { conversions++; return ROUTE; } };
  h.context.fetch(url); assert.equal(conversions, 1);
  xhr.open('POST', url); assert.equal(conversions, 2);
  h.stop(); xhr.send('{"symbol":"private-canary"}');
  assert.equal(h.shapes(), 0);
  assert.equal(h.messages.length, 0);
});

test('request body streams/accessors/FormData-like objects are never inspected', async () => {
  const h = fixture(); h.api.start(SESSION);
  let getters = 0;
  const init = { method: 'POST' };
  Object.defineProperty(init, 'body', { get() { getters++; throw new Error('body read'); } });
  h.context.fetch(ROUTE, init);
  const request = new Request('https://www.binance.com' + ROUTE, { method: 'POST', body: 'private-canary' });
  h.context.fetch(request);
  assert.equal(request.bodyUsed, false); assert.equal(getters, 0); assert.equal(h.shapes(), 0);
  h.requests.forEach(pending => pending.resolve(response().value)); await flush();
  assert(h.messages.every(entry => entry.message.observation.requestShape === null)); h.stop();
});

test('relative request paths use document baseURI rather than the origin root', async () => {
  const h = fixture({ base: 'https://www.binance.com/en/futures/' }); h.api.start(SESSION);
  h.context.fetch('future-grid/query'); h.requests[0].resolve(response(undefined, { url: 'https://www.binance.com/en/futures/future-grid/query' }).value);
  await flush();
  assert.equal(h.messages[0].message.observation.path, '/{segment}/futures/future-grid/query'); h.stop();
});

test('fetch observer concurrency is capped before cloning and inspecting request shapes', async () => {
  const h = fixture(); h.api.start(SESSION);
  const replies = [];
  for (let i = 0; i < 8; i++) { h.context.fetch(ROUTE, { body: '{}' }); replies.push(response()); }
  assert.equal(h.shapes(), 4);
  h.requests.forEach((pending, i) => pending.resolve(replies[i].value)); await flush();
  assert.equal(replies.reduce((sum, reply) => sum + reply.counts.clones, 0), 4);
  assert.equal(h.messages.length, 4); h.stop();
});

test('oversize and timeout cancel only clone readers without reading original body', async () => {
  for (const mode of ['too_large', 'timeout']) {
    const h = fixture(); h.api.start(SESSION); h.context.fetch(ROUTE);
    const reply = response('', mode === 'too_large' ? { chunks: [new Uint8Array(S.limits.bodyBytes + 1)] } : { hang: true });
    h.requests[0].resolve(reply.value); await flush();
    if (mode === 'timeout') h.advance(S.limits.readMs);
    await flush(); assert.equal(h.messages[0].message.observation.outcome, mode);
    assert(reply.counts.cancels > 0); assert.equal(reply.counts.originalReads, 0); h.stop();
  }
});

test('stop/new generation/navigation/expiry discard late responses and never recapture old requests', async () => {
  for (const action of ['stop', 'replace', 'navigate', 'expire', 'pagehide']) {
    const h = fixture(); h.api.start(SESSION); h.context.fetch(ROUTE);
    if (action === 'stop') h.stop();
    if (action === 'replace') h.api.start('synthetic_session_00000002');
    if (action === 'navigate') h.context.history.pushState({}, '', '/en/futures/ETHUSDT');
    if (action === 'expire') h.advance(S.limits.lifetimeMs);
    if (action === 'pagehide') h.dispatch('pagehide');
    h.requests[0].resolve(response().value); await flush();
    assert.equal(h.messages.length, 0, action); h.stop();
  }
  const h = fixture(); h.context.fetch(ROUTE); h.api.start(SESSION);
  h.requests[0].resolve(response().value); await flush(); assert.equal(h.messages.length, 0); h.stop();
});

test('XHR preserves open/send return values, arguments and original listeners', () => {
  const h = fixture(); h.api.start(SESSION); const xhr = new h.context.XMLHttpRequest();
  let originalEvents = 0; xhr.addEventListener('loadend', () => originalEvents++);
  assert.equal(xhr.open('POST', ROUTE, true), 'open-result');
  const body = '{"symbol":"private-canary"}'; assert.equal(xhr.send(body), 'send-result');
  xhr.complete(); assert.equal(originalEvents, 1);
  assert.equal(h.calls.find(call => call.kind === 'send').args[0], body);
  assert.equal(h.calls.find(call => call.kind === 'send').receiver, xhr);
  assert.equal(h.messages.length, 1); assert.deepEqual(xhr.headerCalls, ['content-type']);
  assert(!JSON.stringify(h.messages).includes('private-canary')); h.stop();
});

test('XHR synchronous completion, thrown errors and late completion preserve caller behavior', () => {
  const h = fixture(); h.api.start(SESSION);
  const sync = new h.context.XMLHttpRequest(); sync.synchronous = true; sync.open('GET', ROUTE, false); sync.send();
  assert.equal(h.messages.length, 1);
  const failed = new h.context.XMLHttpRequest(); const error = new Error('private-canary'); failed.open('GET', ROUTE); failed.sendError = error;
  assert.throws(() => failed.send(), value => value === error);
  failed.complete(); assert.equal(h.messages.length, 1);
  const late = new h.context.XMLHttpRequest(); late.open('GET', ROUTE); late.send(); h.stop(); late.complete();
  assert.equal(h.messages.length, 1);
});
