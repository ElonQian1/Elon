'use strict';
const assert = require('node:assert/strict');
const { test } = require('node:test');
const { webcrypto } = require('node:crypto');
const browser = require('../android/app/src/main/assets/chatgpt_web_private_directory_browser.js');
const pages = require('../android/app/src/main/assets/chatgpt_web_private_directory_pages.js');
const json = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');
const deferred = () => { let resolve; const promise = new Promise(r => { resolve = r; }); return { promise, resolve }; };
const flush = () => new Promise(resolve => setImmediate(resolve));
const row = id => ({ id: String(id), title: 'Fixture ' + id, owner: {} });
const response = value => new Response(JSON.stringify(value));
const normalize = (_, items) => items.map(({ id, title }) => ({ id, title, path: '/c/' + id }));

function fixture(total = 280) {
  const calls = [];
  let time = 0, identityRequests = 0;
  const headers = { Authorization: 'Bearer synthetic-browser-fixture', 'chatgpt-account-id': 'synthetic-account' };
  const root = {
    location: { origin: 'https://chatgpt.com' }, __elonChatGptDocumentToken: 'doc_browser_fixture',
    crypto: webcrypto, AbortController, setTimeout, clearTimeout, performance: { now: () => time },
    __elonChatGptPrivateDirectoryPages: pages, __elonChatGptPrivateJsonRequest: json,
    __elonChatGptPrivateTransport: {
      copySameOriginRequestHeaders: () => ({ ...headers }),
      acquireSameOriginRequestHeaders: async () => { identityRequests++; return { ...headers }; },
    },
  };
  const f = { root, headers, calls, advance: ms => { time += ms; }, identities: () => identityRequests };
  f.fetcher = async url => {
    const offset = Number(new URL(url, root.location.origin).searchParams.get('offset'));
    return response({ offset, limit: 28, total,
      items: Array.from({ length: Math.max(0, Math.min(28, total - offset)) }, (_, n) => row(offset + n)) });
  };
  f.fetch = (url, options) => { calls.push({ url, method: options.method }); return f.fetcher(url, options); };
  f.api = browser.create(root, f.fetch, normalize);
  return f;
}

test('explicit full-page browsing crosses the old 200-row boundary without dropping overflow', async () => {
  const f = fixture(), ids = [];
  let handle = '', count = 0, value;
  do {
    assert.equal(f.calls.length, count, 'no background prefetch or whole-account loop');
    value = await f.api.read('conversations', handle);
    assert.equal(value.ok, true);
    if (count === 7) assert.deepEqual(value.items.map(item => Number(item.id)), Array.from({ length: 28 }, (_, n) => 196 + n));
    ids.push(...value.items.map(item => Number(item.id)));
    count++;
    handle = value.nextHandle;
  } while (handle);
  assert.equal(value.complete, true);
  assert.deepEqual(ids, Array.from({ length: 280 }, (_, n) => n));
  assert.equal(f.calls.length, 10);
  assert(f.calls.every(call => call.method === 'GET'));
});

for (const scope of ['projects', 'g-p-fixture']) {
  test(scope + ' follows opaque page cursors beyond the recent catalog cap', async () => {
    const f = fixture(), total = scope === 'projects' ? 65 : 250, limit = scope === 'projects' ? 20 : 28;
    f.fetcher = async url => {
      const cursor = new URL(url, 'https://chatgpt.com').searchParams.get('cursor');
      const offset = cursor === null || cursor === '0' ? 0 : Number(cursor.split('|')[1]);
      const items = Array.from({ length: Math.min(limit, total - offset) }, (_, n) => scope === 'projects'
        ? { gizmo: { gizmo: { id: 'g-p-' + (offset + n), display: { name: 'Project ' + (offset + n) } } }, conversations: null }
        : row(offset + n));
      return response({ items, cursor: offset + limit < total ? 'opaque/+?&=|' + (offset + limit) : null });
    };
    let handle = '', value;
    const ids = [];
    do {
      value = await f.api.read(scope, handle);
      assert.equal(value.ok, true);
      ids.push(...value.items.map(item => item.id));
      handle = value.nextHandle;
      assert(!JSON.stringify(value).includes('opaque/+?&='), 'raw cursor stays in the page identity owner');
    } while (handle);
    assert.equal(value.complete, true);
    assert.equal(new Set(ids).size, total);
  });
}

test('duplicate taps share one pending request and cached revisits need neither identity acquisition nor fetch', async () => {
  const f = fixture(), pending = deferred();
  f.fetcher = () => pending.promise;
  const a = f.api.read('conversations'), b = f.api.read('conversations');
  assert.equal(a, b);
  await flush();
  pending.resolve(response({ items: [row('a')], offset: 0, limit: 28, total: 1 }));
  const first = await a;
  const again = await f.api.read('conversations', first.handle);
  assert.equal(again.cached, true);
  assert.equal(f.calls.length, 1);
  assert.equal(f.identities(), 1);
  again.items[0].title = 'Modified caller value';
  assert.equal((await f.api.read('conversations', first.handle)).items[0].title, 'Fixture a');
});

test('a cached page selection supersedes a pending later selection', async () => {
  const f = fixture(), first = await f.api.read('conversations'), pending = deferred();
  f.fetcher = () => pending.promise;
  const next = f.api.read('conversations', first.nextHandle);
  await flush();
  assert.equal((await f.api.read('conversations', first.handle)).cached, true);
  pending.resolve(response({ items: [row('late')], offset: 28, limit: 28, total: 29 }));
  assert.equal((await next).ok, false);
});

test('a new scope supersedes an old request without cancelling other directory components', async () => {
  const f = fixture(), pending = deferred();
  f.fetcher = url => url.includes('/gizmos/') ? response({ items: [], cursor: null }) : pending.promise;
  const old = f.api.read('conversations');
  await flush();
  const next = await f.api.read('projects');
  assert.equal(next.complete, true);
  pending.resolve(response({ items: [row('late')], offset: 0, limit: 28, total: 1 }));
  assert.equal((await old).ok, false);
});

test('raw cursors, wrong-scope tickets and forged handles cannot choose request paths', async () => {
  const f = fixture(), first = await f.api.read('conversations');
  for (const handle of ['28', '/backend-api/conversations?offset=28', 'dp_' + '0'.repeat(32)]) {
    assert.equal((await f.api.read('conversations', handle)).code, 'directory_page_expired');
  }
  assert.equal((await f.api.read('projects', first.nextHandle)).code, 'directory_page_expired');
  assert.equal((await f.api.read('../other')).code, 'directory_scope_invalid');
  assert.equal(f.calls.length, 1);
});

for (const change of ['account', 'document', 'transport', 'origin', 'cancel']) {
  test('a pending page and its tickets cannot survive ' + change, async () => {
    const f = fixture(), first = await f.api.read('conversations'), pending = deferred();
    f.fetcher = () => pending.promise;
    const next = f.api.read('conversations', first.nextHandle);
    await flush();
    if (change === 'account') f.headers['chatgpt-account-id'] = 'replacement';
    if (change === 'document') f.root.__elonChatGptDocumentToken = 'doc_replacement';
    if (change === 'transport') f.root.__elonChatGptPrivateTransport = { ...f.root.__elonChatGptPrivateTransport };
    if (change === 'origin') f.root.location.origin = 'https://example.com';
    if (change === 'cancel') f.api.cancel();
    pending.resolve(response({ items: [row('late')], offset: 28, limit: 28, total: 29 }));
    const result = await next;
    assert.equal(result.ok, false);
    assert.equal(result.items, undefined);
    assert.equal((await f.api.read('conversations', first.handle)).ok, false);
    assert.equal(f.calls.length, 2);
  });
}

test('unknown pagination is partial, not terminal, and does not guess a next offset', async () => {
  const f = fixture();
  f.fetcher = async () => response({ items: [row('a')] });
  const result = await f.api.read('conversations');
  assert.equal(result.ok, true);
  assert.equal(result.complete, false);
  assert.equal(result.nextHandle, null);
  assert.equal(result.code, 'directory_page_unknown');
  assert.equal(f.calls.length, 1);
});

test('a confirmed empty terminal page is distinguishable from missing pagination', async () => {
  const result = await fixture(0).api.read('conversations');
  assert.equal(result.complete, true);
  assert.deepEqual(result.items, []);
});

test('a cyclic opaque cursor stops without issuing a request to the earlier page again', async () => {
  const f = fixture();
  f.fetcher = async url => {
    const token = new URL(url, 'https://chatgpt.com').searchParams.get('cursor');
    return response({ items: [row(token)], cursor: token === '0' ? 'a' : token === 'a' ? 'b' : 'a' });
  };
  let next = '';
  for (let n = 0; n < 2; n++) next = (await f.api.read('g-p-fixture', next)).nextHandle;
  assert.equal((await f.api.read('g-p-fixture', next)).code, 'directory_cursor_stalled');
  assert.equal(f.calls.length, 3);
});

test('failed next-page reads preserve the exact waypoint for an explicit retry', async () => {
  const f = fixture(), first = await f.api.read('conversations'), fetcher = f.fetcher;
  f.fetcher = async () => new Response('{}', { status: 503 });
  assert.equal((await f.api.read('conversations', first.nextHandle)).ok, false);
  assert.equal(f.calls.length, 2, 'no silent automatic retries');
  f.fetcher = fetcher;
  assert.equal((await f.api.read('conversations', first.nextHandle)).items[0].id, '28');
  assert.equal(f.calls[1].url, f.calls[2].url);
});

test('row-cache eviction retains a valid waypoint and refetches the full page', async () => {
  const f = fixture(2000), first = await f.api.read('conversations');
  let next = first.nextHandle;
  for (let n = 0; n < 20; n++) next = (await f.api.read('conversations', next)).nextHandle;
  const result = await f.api.read('conversations', first.handle);
  assert.equal(result.ok, true);
  assert.equal(result.cached, false);
  assert.equal(result.items.length, 28);
  assert.equal(result.items[0].id, '0');
  assert.equal(f.calls.length, 22);
});

test('expired and evicted waypoints are explicit failures, never a restart at offset zero', async () => {
  const f = fixture(2000), first = await f.api.read('conversations');
  let next = first.nextHandle;
  for (let n = 0; n < 33; n++) next = (await f.api.read('conversations', next)).nextHandle;
  assert.equal((await f.api.read('conversations', first.handle)).code, 'directory_page_expired');
  f.advance(600001);
  assert.equal((await f.api.read('conversations', next)).code, 'directory_page_expired');
  assert.equal(f.calls.length, 34);
});

test('invalid mapping and oversized pages cannot drop rows then advance a cursor', async () => {
  const f = fixture();
  const dropsRow = browser.create(f.root, f.fetch, () => []);
  assert.equal((await dropsRow.read('conversations')).code, 'directory_response_invalid');
  f.fetcher = async () => response({ items: Array.from({ length: 201 }, (_, n) => row(n)), cursor: 'next' });
  assert.equal((await f.api.read('g-p-fixture')).code, 'directory_page_too_large');
});

test('unknown errors and provider credentials never enter public results', async () => {
  const f = fixture();
  const bad = browser.create(f.root, f.fetch, () => { throw new Error('directory_sensitive_' + f.headers.Authorization); });
  assert.deepEqual(await bad.read('conversations'), { ok: false, code: 'directory_page_failed' });
  const result = JSON.stringify(await f.api.read('conversations'));
  assert(!result.includes('synthetic-browser-fixture'));
  assert(!result.includes('synthetic-account'));
  assert(!result.includes('offset='));
});

test('identity acquisition does not need a DOM composer or an already populated identity cache', async () => {
  const f = fixture();
  const known = { ...f.headers };
  delete f.headers.Authorization;
  f.root.__elonChatGptPrivateTransport.acquireSameOriginRequestHeaders = async () => {
    Object.assign(f.headers, known);
    return { ...known };
  };
  assert.equal((await f.api.read('conversations')).ok, true);
  assert.equal(f.root.document, undefined);
});
