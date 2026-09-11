'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, CID, SID, PATH } = require('./fixtures/chatgpt-web-private-conversation-share.cjs');
const contract = require('../android/app/src/main/assets/chatgpt_web_private_conversation_share_contract.js');
const share = require('../android/app/src/main/assets/chatgpt_web_private_conversation_share.js');
const management = require('../android/app/src/main/assets/chatgpt_web_private_shared_links.js');
const LIST = '/backend-api/shared_textdocs', ID = 'synthetic_canvas-01';
const row = (id = ID) => ({ shared_textdoc_id: id, conversation_id: CID,
  created_at: '2026-09-11T10:00:00Z', name: 'Private name must stay out of receipts' });

function setup() {
  const f = fixture(); let rows = [row()], now = 1000, metadata = {}, failDelete = false, readbackPartial = false;
  f.page.crypto = { getRandomValues: bytes => { bytes.fill(31); return bytes; } };
  f.page.__elonChatGptPrivateJsonRequest.request = async (_, url, init, limits) => {
    f.requests.push({ url, init, limits });
    if (url === LIST) return { payload: { shared_textdocs: rows, ...metadata } };
    if (url === '/backend-api/shared_conversations?order=created') return {
      payload: { items: [{ id: SID, conversation_id: CID }], total: 1 } };
    assert.equal(init.method, 'DELETE');
    assert.equal(url, '/backend-api/textdoc/shared/' + ID);
    rows = rows.filter(r => r.shared_textdoc_id !== ID);
    if (readbackPartial) metadata = { has_more: true };
    if (failDelete) throw new Error('timeout');
    return { status: 204 };
  };
  const api = share.create(f.page, { contract, management, now: () => now, loadRuntime: f.loadRuntime });
  const command = (value, confirmed = false) => new Promise(resolve => api.handle('share_conversation',
    { value: JSON.stringify(value), selected: confirmed }, (_, ok, detail) => resolve({ ok, detail,
      data: detail.startsWith('{') ? JSON.parse(detail) : null }), () => f.snapshot));
  return Object.assign(f, { command, list: () => command({ operation: 'list_account', resource: 'canvas' }),
    revoke: data => command({ operation: 'revoke_account', resource: 'canvas', id: ID, ticket: data.ticket }, true),
    setRows: value => { rows = value; }, setMetadata: value => { metadata = value; }, advance: ms => { now += ms; },
    failDelete: () => { failDelete = true; }, partialReadback: () => { readbackPartial = true; } });
}

test('canvas listing uses the observed endpoint without composer or conversation modules', async () => {
  const f = setup(); f.setLoaded(false); f.snapshot.composerReady = false;
  f.setRows([row(), { ...row('orphan_canvas'), conversation_id: null }]);
  const r = await f.list();
  assert.equal(r.ok, true); assert.equal(r.data.schema, 'elon.canvas_shares.v1'); assert.equal(r.data.complete, true);
  assert.deepEqual(r.data.items, [{ id: ID, path: PATH, createdAt: row().created_at },
    { id: 'orphan_canvas', path: null, createdAt: row().created_at }]);
  assert.equal(r.detail.includes('Private name'), false);
  assert.equal((await f.list()).ok, true); assert.equal(f.requests.length, 1);
  assert.equal(f.requests[0].url, LIST); assert.equal(f.requests[0].init.method, 'GET');
  assert.equal(f.requests[0].init.body, undefined);
  assert.equal(f.requests[0].init.headers.cookie, undefined);
  assert.equal(f.requests[0].init.headers['chatgpt-sentinel-proof-token'], undefined);
  f.advance(60001); await f.list(); assert.equal(f.requests.length, 2);
});

test('conversation and canvas caches and selection tickets cannot be mixed', async () => {
  const f = setup(); const canvas = (await f.list()).data;
  const conversation = (await f.command({ operation: 'list_account' })).data;
  assert.notEqual(canvas.ticket, conversation.ticket);
  assert.equal((await f.command({ operation: 'revoke_account', resource: 'canvas', id: ID,
    ticket: conversation.ticket }, true)).ok, false);
  assert.equal((await f.command({ operation: 'revoke', path: PATH, id: SID, ticket: canvas.ticket }, true)).ok, false);
  assert.equal((await f.list()).data.ticket, canvas.ticket);
  assert.equal(f.requests.length, 2);
});

test('canvas revoke consumes one exact ticket and removes sharing only after complete readback', async () => {
  const f = setup(); f.setRows([row(), row('untouched')]);
  const listed = (await f.list()).data;
  assert.equal((await f.command({ operation: 'revoke_account', resource: 'canvas', id: ID,
    ticket: listed.ticket })).ok, false);
  const result = await f.revoke(listed);
  assert.equal(result.ok, true); assert.equal(result.detail, 'share_link_revoked');
  assert.deepEqual(f.requests.map(r => [r.init.method, r.url]), [['GET', LIST],
    ['DELETE', '/backend-api/textdoc/shared/' + ID], ['GET', LIST]]);
  assert.equal(f.requests[1].init.body, undefined);
  assert.deepEqual((await f.list()).data.items.map(i => i.id), ['untouched']);
  assert.equal((await f.revoke(listed)).ok, false);
  assert.equal(f.requests.filter(r => r.init.method === 'DELETE').length, 1);
});

for (const drift of ['account', 'document', 'expired', 'selection']) test('canvas stale selection: ' + drift, async () => {
  const f = setup(); const selected = (await f.list()).data;
  if (drift === 'account') f.headers.Authorization = 'Bearer another-synthetic-account';
  if (drift === 'document') f.page.__elonChatGptDocumentToken = 'doc_other';
  if (drift === 'expired') f.advance(120001);
  if (drift === 'selection') { f.advance(60001); f.setRows([]); await f.list(); }
  assert.equal((await f.revoke(selected)).ok, false);
  assert.equal(f.requests.filter(r => r.init.method === 'DELETE').length, 0);
});

for (const failure of ['timeout', 'partial_readback']) test('uncertain canvas deletion is not replayed: ' + failure, async () => {
  const f = setup(); const selected = (await f.list()).data;
  if (failure === 'timeout') f.failDelete(); else f.partialReadback();
  const result = await f.revoke(selected);
  assert.equal(result.ok, false); assert.equal(result.detail, 'share_revoke_unconfirmed');
  assert.equal((await f.revoke(selected)).ok, false);
  assert.equal((await f.list()).ok, true, 'read-only reconciliation remains available');
  assert.equal(f.requests.filter(r => r.init.method === 'DELETE').length, 1);
});

for (const invalid of [null, {}, [row(), row()], [{ ...row(), shared_textdoc_id: '../all' }],
  [{ ...row(), shared_textdoc_id: 123 }], [{ ...row(), conversation_id: 'foreign' }],
  [{ ...row(), created_at: 'yesterday' }], Array(1001).fill(row())]) {
  test('malformed canvas collection cannot become an empty success: ' + JSON.stringify(invalid)?.slice(0, 80), async () => {
    const f = setup(); f.setRows(invalid); assert.equal((await f.list()).ok, false);
  });
}

test('workspace or server-partial canvas responses remain explicitly partial', async () => {
  const f = setup(); f.setRows([{ ...row(), workspace_id: 'other' }]);
  const r = await f.list(); assert.equal(r.ok, true); assert.equal(r.data.complete, false); assert.deepEqual(r.data.items, []);
  assert.equal((await f.revoke(r.data)).ok, false);
  for (const metadata of [{ total: 3 }, { has_more: true }, { next_cursor: 'opaque' }]) {
    f.advance(60001); f.setRows([]); f.setMetadata(metadata);
    assert.equal((await f.list()).data.complete, false);
  }
});

test('canvas pages reuse one bounded snapshot without inventing HTTP cursor parameters', async () => {
  const f = setup(); f.setRows(Array.from({ length: 102 }, (_, i) => row('a'.repeat(124) + i)));
  const first = (await f.list()).data;
  assert.equal(first.items.length, 100); assert.equal(first.nextOffset, 100);
  assert.ok(JSON.stringify(first).length < 32000);
  const next = await f.command({ operation: 'list_account', resource: 'canvas', offset: 100, ticket: first.ticket });
  assert.equal(next.ok, true); assert.equal(next.data.items.length, 2); assert.equal(next.data.nextOffset, null);
  assert.equal(f.requests.length, 1);
});

for (const input of [{ operation: 'list_account', resource: 'post' },
  { operation: 'list', resource: 'canvas', path: PATH },
  { operation: 'revoke_account', resource: 'canvas', path: PATH },
  { operation: 'list_account', resource: 'canvas', offset: 100 }]) {
  test('invalid canvas scope is rejected before network: ' + JSON.stringify(input), async () => {
    const f = setup(); assert.equal((await f.command(input, true)).ok, false); assert.equal(f.requests.length, 0);
  });
}
