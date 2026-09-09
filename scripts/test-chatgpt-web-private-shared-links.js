'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, CID, NEXT, SID, PATH } = require('./fixtures/chatgpt-web-private-conversation-share.cjs');
const contract = require('../android/app/src/main/assets/chatgpt_web_private_conversation_share_contract.js');
const share = require('../android/app/src/main/assets/chatgpt_web_private_conversation_share.js');
let management;
try { management = require('../android/app/src/main/assets/chatgpt_web_private_shared_links.js'); } catch (_) {}
const LIST = '/backend-api/shared_conversations?order=created';

function setup() {
  const f = fixture();
  let now = 1000, rows = [{ id: SID, conversation_id: CID, title: 'Not exported',
    create_time: '2026-09-01T10:00:00Z', workspace_id: null }], total;
  f.page.crypto = { getRandomValues: bytes => { bytes.fill(23); return bytes; } };
  f.page.__elonChatGptPrivateJsonRequest.request = async (_, url, init, limits) => {
    f.requests.push({ url, init, limits });
    if (url === LIST) return { payload: { items: rows, total: total ?? rows.length } };
    assert.equal(init.method, 'DELETE');
    assert.equal(url, '/backend-api/share/' + SID);
    rows = rows.filter(row => row.id !== SID);
    return { ok: true, status: 204 };
  };
  assert.ok(management, 'private shared-link management module must exist');
  f.api = share.create(f.page, { contract, management, loadRuntime: f.loadRuntime, now: () => now });
  f.command = (value, confirmed = false) => new Promise(resolve => {
    assert.equal(f.api.handle('share_conversation', { value: JSON.stringify(value), selected: confirmed },
      (_, ok, detail) => resolve({ ok, detail, data: detail.startsWith('{') ? JSON.parse(detail) : null }), () => f.snapshot), true);
  });
  f.list = () => f.command({ operation: 'list', path: PATH });
  f.revoke = data => f.command({ operation: 'revoke', path: PATH, id: SID, ticket: data.ticket }, true);
  return Object.assign(f, { setRows: value => { rows = value; }, setTotal: value => { total = value; },
    advance: ms => { now += ms; } });
}

test('lists without publication or DOM readiness and reuses bounded cache', async () => {
  const f = setup(); f.snapshot.composerReady = false;
  const first = await f.list(); assert.equal(first.ok, true);
  assert.equal(first.data.schema, 'elon.conversation_shares.v1');
  assert.equal(first.data.path, PATH); assert.equal(first.data.complete, true);
  assert.deepEqual(first.data.items, [{ id: SID, createdAt: '2026-09-01T10:00:00Z' }]);
  assert.equal(first.detail.includes('Not exported'), false);
  assert.equal((await f.list()).ok, true); assert.equal(f.requests.length, 1);
  const request = f.requests[0]; assert.equal(request.url, LIST); assert.equal(request.init.method, 'GET');
  assert.equal(request.init.credentials, 'include'); assert.equal(request.init.redirect, 'error');
  assert.equal(request.init.headers.cookie, undefined);
  assert.equal(request.init.headers['chatgpt-sentinel-proof-token'], undefined);
  assert.equal(request.init.body, undefined);
  f.advance(60001); await f.list(); assert.equal(f.requests.length, 2);
});

test('authenticated read does not depend on loaded website conversation modules', async () => {
  const f = setup(); f.setLoaded(false); f.snapshot.composerReady = false;
  const result = await f.list();
  assert.equal(result.ok, true);
  assert.deepEqual(result.data.items, [{ id: SID, createdAt: '2026-09-01T10:00:00Z' }]);
  assert.deepEqual(f.requests.map(request => request.init.method), ['GET']);
  assert.equal((await f.list()).ok, true);
  assert.equal(f.requests.length, 1, 'cache-first read does not load a conversation module');
});

test('server-bound link revocation works without conversation modules but public creation stays guarded', async () => {
  const f = setup(); f.setLoaded(false);
  const result = await f.list();
  assert.equal(result.ok, true);
  assert.equal((await f.revoke(result.data)).detail, 'share_link_revoked');
  assert.equal((await f.api.start(PATH, true, () => f.snapshot)).ok, false);
  assert.deepEqual(f.requests.map(request => request.init.method), ['GET', 'DELETE', 'GET']);
});

test('unsupported workspace links are partial instead of a falsely complete empty list', async () => {
  const f = setup(); f.setLoaded(false);
  f.setRows([{ id: SID, conversation_id: CID, workspace_id: 'workspace', create_time: null }]);
  const result = await f.list();
  assert.equal(result.ok, true);
  assert.deepEqual(result.data.items, []);
  assert.equal(result.data.complete, false);
  assert.equal((await f.revoke(result.data)).detail, 'share_selection_expired');
  assert.deepEqual(f.requests.map(request => request.init.method), ['GET']);
});

test('filters other conversations and workspace links and reports partial coverage', async () => {
  const f = setup(); f.setRows([{ id: SID, conversation_id: NEXT, create_time: null },
    { id: NEXT, conversation_id: CID, workspace_id: 'workspace', create_time: null }]); f.setTotal(3);
  const result = await f.list(); assert.equal(result.ok, true);
  assert.deepEqual(result.data.items, []); assert.equal(result.data.complete, false);
});

test('revoke needs selected ticket and explicit confirmation; deletes link only, then readbacks', async () => {
  const f = setup(); const listed = (await f.list()).data;
  assert.equal((await f.command({ operation: 'revoke', path: PATH, id: SID, ticket: listed.ticket })).ok, false);
  assert.equal(f.requests.length, 1);
  const result = await f.revoke(listed); assert.equal(result.ok, true); assert.equal(result.detail, 'share_link_revoked');
  assert.deepEqual(f.requests.map(r => r.init.method), ['GET', 'DELETE', 'GET']);
  assert.equal(f.requests[1].init.body, undefined); assert.equal(f.requests[1].limits.mode, 'none');
  const after = await f.list(); assert.equal(after.data.items.length, 0);
  assert.equal(f.requests.filter(r => r.init.method === 'DELETE').length, 1);
});

for (const scenario of ['account', 'document', 'ticket', 'expired', 'selection']) {
  test('rejects stale revoke before writing: ' + scenario, async () => {
    const f = setup(); const data = (await f.list()).data;
    if (scenario === 'account') f.headers.Authorization = 'Bearer changed-account-value';
    if (scenario === 'document') f.page.__elonChatGptDocumentToken = 'doc_changed';
    if (scenario === 'ticket') data.ticket = 'sl_forged';
    if (scenario === 'expired') f.advance(120001);
    if (scenario === 'selection') f.setRows([]);
    if (scenario === 'selection') { f.advance(60001); await f.list(); }
    assert.equal((await f.revoke(data)).ok, false);
    assert.equal(f.requests.some(r => r.init.method === 'DELETE'), false);
  });
}

for (const payload of [{}, { items: [], total: -1 }, { items: [], total: 0.5 },
  { items: [{ id: 'bad', conversation_id: CID }], total: 1 },
  { items: [{ id: SID, conversation_id: CID, create_time: 'not-a-date' }], total: 1 }]) {
  test('does not turn malformed list into empty success: ' + JSON.stringify(payload), async () => {
    const f = setup(); f.page.__elonChatGptPrivateJsonRequest.request = async () => ({ payload });
    assert.equal((await f.list()).ok, false);
  });
}

test('identity change during GET discards the result', async () => {
  const f = setup(); const request = f.page.__elonChatGptPrivateJsonRequest.request;
  f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
    const result = await request(...args); f.headers['chatgpt-account-id'] = 'other'; return result;
  };
  assert.equal((await f.list()).ok, false);
});

for (const failure of ['timeout', 'partial', 'still-present']) {
  test('uncertain revoke never repeats a write: ' + failure, async () => {
    const f = setup(); const data = (await f.list()).data;
    const original = f.page.__elonChatGptPrivateJsonRequest.request;
    f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
      if (args[2].method === 'DELETE') {
        f.requests.push({ url: args[1], init: args[2] });
        if (failure === 'timeout') throw new Error('timeout');
        if (failure === 'partial') { f.setRows([]); f.setTotal(9); }
        return { ok: true, status: 204 };
      }
      return original(...args);
    };
    const result = await f.revoke(data); assert.equal(result.ok, false);
    assert.equal(result.detail, 'share_revoke_unconfirmed');
    await f.revoke(data); assert.equal(f.requests.filter(r => r.init.method === 'DELETE').length, 1);
  });
}

test('single flight and account rejection remain scoped', async () => {
  const f = setup(); let release;
  f.page.__elonChatGptPrivateJsonRequest.request = () => new Promise(resolve => { release = resolve; });
  const first = f.list();
  for (let i = 0; i < 20 && !release; i++) await new Promise(resolve => setImmediate(resolve));
  assert.equal((await f.list()).detail, 'share_busy');
  release({ payload: { items: [], total: 0 } }); assert.equal((await first).ok, true);
  f.headers.Authorization = ''; assert.equal((await f.list()).ok, false);
});

test('revoking a link invalidates the existing create-result cache', async () => {
  const f = fixture(); const original = f.page.__elonChatGptPrivateJsonRequest.request;
  f.page.crypto = { getRandomValues: bytes => { bytes.fill(29); return bytes; } };
  let removed = false;
  f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
    const [, url, init] = args;
    if (url === LIST) return { payload: { total: removed ? 0 : 1,
      items: removed ? [] : [{ id: SID, conversation_id: CID, create_time: null }] } };
    if (init.method === 'DELETE') { removed = true; return { ok: true, status: 204 }; }
    return original(...args);
  };
  const api = share.create(f.page, { contract, management, loadRuntime: f.loadRuntime });
  const create = () => api.start(PATH, true, () => f.snapshot);
  assert.equal((await create()).ok, true); assert.equal((await create()).ok, true);
  assert.equal(f.requests.filter(r => r.init.method === 'POST').length, 1);
  const listed = await api.start({ operation: 'list', path: PATH }, false);
  assert.equal((await api.start({ operation: 'revoke', path: PATH, id: SID, ticket: listed.data.ticket }, true)).ok, true);
  assert.equal((await create()).ok, true);
  assert.equal(f.requests.filter(r => r.init.method === 'POST').length, 2);
});

test('invalid management commands cannot fall through to public creation', async () => {
  const f = setup();
  for (const input of [{ operation: 'delete_all', path: PATH }, { operation: 'list', path: '/c/../all' },
    { operation: 'revoke', path: PATH, id: SID, ticket: 'invalid' }]) {
    assert.equal((await f.command(input, true)).ok, false);
  }
  let receipt;
  f.api.handle('share_conversation', { value: '{broken', selected: true }, (...args) => { receipt = args; });
  assert.deepEqual(receipt, ['share_conversation', false, 'share_invalid_selection']);
  assert.equal(f.requests.length, 0);
});

test('uncertain publication invalidates the old empty list and allows read-only reconciliation', async () => {
  const f = fixture(); let created = false, reads = 0;
  f.page.crypto = { getRandomValues: bytes => { bytes.fill(29); return bytes; } };
  f.page.__elonChatGptPrivateJsonRequest.request = async (_, url, init) => {
    if (url === LIST) { reads++; return { payload: { total: created ? 1 : 0,
      items: created ? [{ id: SID, conversation_id: CID, create_time: null }] : [] } }; }
    assert.equal(init.method, 'POST'); created = true; throw new Error('timeout');
  };
  const api = share.create(f.page, { contract, management, loadRuntime: f.loadRuntime });
  const list = () => api.start({ operation: 'list', path: PATH }, false);
  assert.equal((await list()).data.items.length, 0);
  assert.equal((await api.start(PATH, true, () => f.snapshot)).code, 'share_result_unconfirmed');
  assert.equal((await list()).data.items[0].id, SID);
  assert.equal(reads, 2);
  assert.equal((await api.start({ operation: 'list_account' }, false)).data.items[0].id, SID);
  assert.equal(reads, 2);
  assert.equal((await api.start(PATH, true, () => f.snapshot)).code, 'share_cooldown');
});

test('account list is a read-only projection of the same authenticated collection without runtime or DOM', async () => {
  const f = setup(); f.setLoaded(false); f.snapshot.composerReady = false;
  f.setRows([{ id: SID, conversation_id: NEXT, title: 'Never export this title', create_time: null },
    { id: NEXT, conversation_id: CID, create_time: null }]);
  const result = await f.command({ operation: 'list_account' });
  assert.equal(result.ok, true);
  assert.equal(result.data.schema, 'elon.account_shares.v1');
  assert.equal(result.data.offset, 0); assert.equal(result.data.nextOffset, null);
  assert.equal(result.data.complete, true);
  assert.deepEqual(result.data.items, [{ id: SID, path: '/c/' + NEXT, createdAt: null },
    { id: NEXT, path: PATH, createdAt: null }]);
  assert.equal(result.detail.includes('Never export'), false);
  assert.deepEqual(f.requests.map(r => [r.url, r.init.method]), [[LIST, 'GET']]);
  assert.equal((await f.list()).ok, true); assert.equal(f.requests.length, 1);
});

test('account pages traverse one fixed snapshot without guessing server pagination parameters', async () => {
  const f = setup();
  f.setRows(Array.from({ length: 205 }, (_, i) => ({
    id: 'aaaaaaaa-aaaa-4aaa-8aaa-' + i.toString(16).padStart(12, '0'), conversation_id: CID, create_time: null,
  })));
  const first = (await f.command({ operation: 'list_account' })).data;
  assert.ok(first); assert.equal(first.items.length, 100); assert.equal(first.nextOffset, 100);
  f.advance(60001);
  const second = (await f.command({ operation: 'list_account', offset: 100, ticket: first.ticket })).data;
  assert.equal(second.items.length, 100); assert.equal(second.nextOffset, 200);
  const third = (await f.command({ operation: 'list_account', offset: 200, ticket: first.ticket })).data;
  assert.equal(third.items.length, 5); assert.equal(third.nextOffset, null);
  const previous = (await f.command({ operation: 'list_account', offset: 0, ticket: first.ticket })).data;
  assert.deepEqual(previous, first); assert.equal(f.requests.length, 1);
  assert.equal(new Set([...first.items, ...second.items, ...third.items].map(row => row.id)).size, 205);
  for (const page of [first, second, third]) assert.ok(JSON.stringify(page).length <= 18000);
  f.advance(60000);
  assert.equal((await f.command({ operation: 'list_account', offset: 100, ticket: first.ticket })).detail,
    'share_selection_expired');
  assert.equal(f.requests.length, 1);
});

test('account selection revokes only its server-bound source conversation and leaves current route untouched', async () => {
  const f = setup(); f.setRows([{ id: SID, conversation_id: NEXT, create_time: null }]);
  const before = f.page.location.href;
  const list = (await f.command({ operation: 'list_account' })).data;
  assert.ok(list);
  assert.equal((await f.command({ operation: 'revoke', path: PATH, id: SID, ticket: list.ticket }, true)).ok, false);
  const row = list.items[0];
  const result = await f.command({ operation: 'revoke', path: row.path, id: row.id, ticket: list.ticket }, true);
  assert.equal(result.detail, 'share_link_revoked'); assert.equal(f.page.location.href, before);
  assert.deepEqual(f.requests.map(r => r.init.method), ['GET', 'DELETE', 'GET']);
});

test('account list excludes workspace links and cannot claim a complete personal-only result', async () => {
  const f = setup(); f.setRows([{ id: SID, conversation_id: CID, workspace_id: 'workspace', create_time: null }]);
  const result = await f.command({ operation: 'list_account' });
  assert.equal(result.ok, true); assert.equal(result.data.complete, false);
  assert.deepEqual(result.data.items, []);
  assert.equal((await f.revoke(result.data)).ok, false);
  f.advance(60001); f.setRows([]); f.setTotal(20);
  assert.equal((await f.command({ operation: 'list_account' })).data.complete, false);
});

test('account pagination rejects invented, stale and cross-document selections before network access', async () => {
  const f = setup();
  for (const value of [{ offset: 100 }, { offset: -1 }, { offset: 1 }, { offset: 1000 },
    { offset: '0' }, { offset: 1.2 }, { path: PATH }, { ticket: 'sl_forged' }]) {
    assert.equal((await f.command({ operation: 'list_account', ...value })).ok, false);
  }
  assert.equal(f.requests.length, 0);
  const first = (await f.command({ operation: 'list_account' })).data;
  assert.ok(first);
  f.page.__elonChatGptDocumentToken = 'doc_changed';
  assert.equal((await f.command({ operation: 'list_account', offset: 0, ticket: first.ticket })).ok, false);
  assert.equal(f.requests.length, 1);
});
