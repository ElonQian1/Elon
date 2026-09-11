'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, CID } = require('./fixtures/chatgpt-web-private-conversation-share.cjs');
const contract = require('../android/app/src/main/assets/chatgpt_web_private_conversation_share_contract.js');
const share = require('../android/app/src/main/assets/chatgpt_web_private_conversation_share.js');
const management = require('../android/app/src/main/assets/chatgpt_web_private_shared_links.js');
const content = require('../android/app/src/main/assets/chatgpt_web_private_canvas_content.js');
const ID = 'synthetic_canvas-01', LIST = '/backend-api/shared_textdocs', GET = '/backend-api/textdoc/shared/' + ID;
const body = () => ({ shared_textdoc_id: ID, name: 'Synthetic document', content: '# Synthetic\n\n<script>inert()</script>',
  textdoc_type: 'document', version: 2, access: 'public', is_moderation_blocked: false, is_anonify_api_key_detected: false });

function setup() {
  const f = fixture(); let now = 1000, doc = body(), blocked, hook = () => {}, call = 0;
  f.page.crypto = { getRandomValues: bytes => { bytes.fill(31); return bytes; } };
  f.page.__elonChatGptPrivateCanvasContent = content;
  f.page.__elonChatGptPrivateJsonRequest.request = async (_, url, init, limits) => {
    f.requests.push({ url, init, limits });
    if (url === LIST) return { payload: { shared_textdocs: [{ shared_textdoc_id: ID, conversation_id: CID }] } };
    assert.equal(url, GET); assert.equal(init.method, 'GET');
    if (blocked) throw new Error(blocked);
    await hook();
    return { payload: { shared_textdoc: doc } };
  };
  const api = share.create(f.page, { contract, management, now: () => now, loadRuntime: f.loadRuntime });
  const command = value => new Promise(resolve => {
    const events = [];
    api.handle('share_conversation', { value: JSON.stringify(value), requestId: 'mcp_' + (++call).toString(36) },
      (_, ok, detail) => resolve({ ok, detail, events }), () => f.snapshot, event => events.push(event));
  });
  return Object.assign(f, { api, command,
    list: async () => JSON.parse((await command({ resource: 'canvas', operation: 'list_account' })).detail),
    read: ticket => command({ resource: 'canvas', operation: 'read_account', id: ID, ticket }),
    setDoc: next => { doc = next; }, advance: ms => { now += ms; },
    setError: error => { blocked = error; }, setHook: value => { hook = value; } });
}

test('read uses a bound private GET and an independent display event, never content in receipt', async () => {
  const f = setup(); f.setLoaded(false); f.snapshot.composerReady = false;
  const ticket = (await f.list()).ticket;
  const result = await f.read(ticket);
  assert.equal(result.ok, true); assert.equal(result.detail, 'share_canvas_ready');
  assert.equal(result.events.length, 1);
  assert.deepEqual(result.events[0], { type: 'canvas_shared_content', version: 1, requestId: 'mcp_2',
    id: ID, title: body().name, content: body().content, documentType: 'document', documentVersion: 2, access: 'public' });
  assert.deepEqual(f.requests.map(r => [r.url, r.init.method]), [[LIST, 'GET'], [GET, 'GET']]);
  assert.equal(f.requests[1].init.body, undefined); assert.equal(f.requests[1].init.headers.cookie, undefined);
  assert.equal(f.requests[1].init.headers['chatgpt-sentinel-proof-token'], undefined);
  assert.ok(f.requests[1].limits.timeoutMs <= 7000); assert.ok(f.requests[1].limits.maxBytes <= 1024 * 1024);
});

test('hot reopen uses the cached content; expiry re-reads without consuming the list ticket', async () => {
  const f = setup(), ticket = (await f.list()).ticket;
  await f.read(ticket); const again = await f.read(ticket);
  assert.equal(again.events[0].requestId, 'mcp_3'); assert.equal(f.requests.length, 2);
  f.advance(60001); f.setDoc({ ...body(), version: 3 });
  assert.equal((await f.read(ticket)).events[0].documentVersion, 3); assert.equal(f.requests.length, 3);
});

for (const drift of ['account', 'document', 'ticket', 'expired', 'id', 'path', 'resource']) {
  test('rejects wrong canvas owner or scope before content read: ' + drift, async () => {
    const f = setup(), ticket = (await f.list()).ticket;
    const input = { operation: 'read_account', resource: 'canvas', id: ID, ticket };
    if (drift === 'account') f.headers.Authorization = 'Bearer synthetic-new';
    if (drift === 'document') f.page.__elonChatGptDocumentToken = 'doc_new';
    if (drift === 'expired') f.advance(120001);
    if (drift === 'id') input.id = 'not-in-list';
    if (drift === 'ticket') input.ticket = 'not-bound';
    if (drift === 'path') input.path = '/c/' + CID;
    if (drift === 'resource') input.resource = 'conversation';
    const result = await f.command(input);
    assert.equal(result.ok, false); assert.deepEqual(result.events, []); assert.equal(f.requests.length, 1);
  });
}

test('identity drift during the response cannot publish data or poison the cache', async () => {
  const f = setup(), ticket = (await f.list()).ticket;
  f.setHook(() => { f.page.__elonChatGptDocumentToken = 'doc_replaced'; });
  const result = await f.read(ticket);
  assert.equal(result.ok, false); assert.equal(result.detail, 'share_context_changed');
  assert.deepEqual(result.events, []);
});

for (const patch of [{ shared_textdoc_id: 'other' }, { content: null }, { content: 'x'.repeat(131073) },
  { name: '' }, { name: 'x'.repeat(513) }, { name: 'bad\nname' }, { textdoc_type: 'unrecognized' },
  { version: 0 }, { version: '2' }, { version: 1.5 }, { version: Number.MAX_SAFE_INTEGER + 1 },
  { is_moderation_blocked: true }, { is_moderation_blocked: undefined },
  { is_anonify_api_key_detected: true }, { is_anonify_api_key_detected: 'false' },
  { access: 'workspace' }, { access: 'private' }]) {
  test('unknown/restricted/oversize content is not silently shown: ' + Object.keys(patch)[0] + ':' + String(Object.values(patch)[0]).slice(0, 14), async () => {
    const f = setup(), ticket = (await f.list()).ticket;
    f.setDoc({ ...body(), ...patch });
    const result = await f.read(ticket);
    assert.equal(result.ok, false); assert.deepEqual(result.events, []);
    assert.match(result.detail, /^share_canvas_/);
  });
}

test('optional version is represented as unknown, not an invented editable version', async () => {
  const f = setup(), ticket = (await f.list()).ticket;
  f.setDoc({ ...body(), version: undefined, is_anonify_api_key_detected: undefined, textdoc_type: 'code/tsx' });
  const result = await f.read(ticket);
  assert.equal(result.ok, true); assert.equal(result.events[0].documentVersion, null);
  assert.equal(result.events[0].documentType, 'code/tsx');
});

test('failed reads do not disable the capability or trigger writes; explicit retry can succeed', async () => {
  const f = setup(), ticket = (await f.list()).ticket;
  f.setError('timeout'); const failed = await f.read(ticket);
  assert.equal(failed.ok, false); assert.deepEqual(failed.events, []);
  f.setError(null); assert.equal((await f.read(ticket)).ok, true);
  assert.ok(f.requests.every(r => r.init.method === 'GET'));
});

test('read cannot succeed without the native display event channel', async () => {
  const f = setup(), ticket = (await f.list()).ticket;
  let receipt;
  f.api.handle('share_conversation', { value: JSON.stringify({ operation: 'read_account', resource: 'canvas', id: ID, ticket }) },
    (_, ok, detail) => { receipt = { ok, detail }; }, () => f.snapshot);
  assert.deepEqual(receipt, { ok: false, detail: 'share_canvas_unavailable' }); assert.equal(f.requests.length, 1);
});
