'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, CID } = require('./fixtures/chatgpt-web-private-conversation-share.cjs');
const contract = require('../android/app/src/main/assets/chatgpt_web_private_conversation_share_contract.js');
const share = require('../android/app/src/main/assets/chatgpt_web_private_conversation_share.js');
const management = require('../android/app/src/main/assets/chatgpt_web_private_shared_links.js');
const content = require('../android/app/src/main/assets/chatgpt_web_private_canvas_content.js');
const ID = 'synthetic_canvas-01', LIST = '/backend-api/shared_textdocs', GET = '/backend-api/textdoc/shared/' + ID;
const doc = version => ({ shared_textdoc_id: ID, name: 'Synthetic canvas', content: 'Synthetic v' + version,
  textdoc_type: 'document', version, access: 'public', is_moderation_blocked: false, is_anonify_api_key_detected: false });

function setup() {
  const f = fixture(); let now = 1000, calls = 0, published = doc(2), postPatch = {}, hook = () => {};
  f.page.crypto = { getRandomValues: bytes => { bytes.fill(17); return bytes; } };
  f.page.__elonChatGptPrivateCanvasContent = content;
  f.page.__elonChatGptPrivateJsonRequest.request = async (_, url, init, limits) => {
    f.requests.push({ url, init, limits });
    await hook({ url, init, limits });
    if (url === LIST) return { payload: { shared_textdocs: [{ shared_textdoc_id: ID, conversation_id: CID }] } };
    if (url === GET && init.method === 'GET') return { payload: { shared_textdoc: published } };
    assert.equal(url, GET + '/update_to_latest'); assert.equal(init.method, 'POST');
    published = { ...doc(3), ...postPatch };
    return { payload: { shared_textdoc: published } };
  };
  const api = share.create(f.page, { contract, management, now: () => now, loadRuntime: f.loadRuntime });
  const command = (value, confirmed = false) => new Promise(resolve => {
    const events = [];
    api.handle('share_conversation', { value: JSON.stringify(value), selected: confirmed, requestId: 'mcp_' + (++calls).toString(36) },
      (_, ok, detail) => resolve({ ok, detail, events }), () => f.snapshot, event => events.push(event));
  });
  return Object.assign(f, { api, command,
    list: async () => JSON.parse((await command({ resource: 'canvas', operation: 'list_account' })).detail),
    update: (ticket, confirmed = true) => command({ resource: 'canvas', operation: 'update_account', id: ID, ticket }, confirmed),
    setDoc: value => { published = value; }, setPostPatch: value => { postPatch = value; },
    advance: ms => { now += ms; }, setHook: value => { hook = value; } });
}

test('confirmed native publish validates, writes once, reads back and emits only matching content', async () => {
  const f = setup(); f.setLoaded(false); f.snapshot.composerReady = false;
  const ticket = (await f.list()).ticket;
  const result = await f.update(ticket);
  assert.equal(result.ok, true); assert.equal(result.detail, 'share_canvas_updated');
  assert.equal(result.events.length, 1); assert.equal(result.events[0].content, doc(3).content);
  assert.equal(result.events[0].documentVersion, 3); assert.equal(result.events[0].requestId, 'mcp_2');
  assert.deepEqual(f.requests.map(r => [r.url, r.init.method]), [[LIST, 'GET'], [GET, 'GET'], [GET + '/update_to_latest', 'POST'], [GET, 'GET']]);
  const write = f.requests[2]; assert.equal(write.init.body, undefined); assert.equal(write.init.headers.cookie, undefined);
  assert.equal(write.init.redirect, 'error'); assert.ok(write.limits.timeoutMs <= 7000);
  assert.equal((await f.update(ticket)).ok, false);
  assert.equal(f.requests.filter(r => r.init.method === 'POST').length, 1);
});

for (const change of ['confirmation', 'account', 'document', 'ticket', 'id', 'resource', 'path', 'expired']) {
  test('publishing rejects stale or unconfirmed selection: ' + change, async () => {
    const f = setup(), ticket = (await f.list()).ticket;
    const input = { operation: 'update_account', resource: 'canvas', id: ID, ticket };
    if (change === 'account') f.headers.Authorization = 'Bearer synthetic-other';
    if (change === 'document') f.page.__elonChatGptDocumentToken = 'doc_other';
    if (change === 'ticket') input.ticket = 'sl_wrong';
    if (change === 'id') input.id = 'not-owned';
    if (change === 'resource') input.resource = 'conversation';
    if (change === 'path') input.path = '/c/' + CID;
    if (change === 'expired') f.advance(120001);
    const result = await f.command(input, change !== 'confirmation');
    assert.equal(result.ok, false); assert.deepEqual(result.events, []); assert.equal(f.requests.length, 1);
  });
}

for (const patch of [{ access: 'private' }, { access: 'workspace' }, { version: null },
  { is_moderation_blocked: true }, { is_anonify_api_key_detected: true }]) {
  test('live access and version are checked even after a successful cached read: ' + JSON.stringify(patch), async () => {
    const f = setup(), ticket = (await f.list()).ticket;
    await f.command({ operation: 'read_account', resource: 'canvas', id: ID, ticket });
    f.setDoc({ ...doc(2), ...patch });
    const result = await f.update(ticket);
    assert.equal(result.ok, false); assert.deepEqual(result.events, []);
    assert.equal(f.requests.filter(r => r.init.method === 'POST').length, 0);
  });
}

for (const failure of ['timeout', 'http_403', 'readback', 'version_regressed', 'content_changed', 'account', 'document']) {
  test('uncertain publish is never replayed or shown as success: ' + failure, async () => {
    const f = setup(), ticket = (await f.list()).ticket; let writes = 0;
    f.setHook(({ url, init }) => {
      if (init.method === 'POST') {
        writes += 1;
        if (failure === 'timeout' || failure === 'http_403') throw new Error(failure);
        if (failure === 'account') f.headers.Authorization = 'Bearer synthetic-other';
        if (failure === 'document') f.page.__elonChatGptDocumentToken = 'doc_changed';
      } else if (writes && url === GET) {
        if (failure === 'readback') throw new Error('timeout');
        if (failure === 'version_regressed') f.setDoc(doc(1));
        if (failure === 'content_changed') f.setDoc({ ...doc(3), content: 'Synthetic concurrent edit' });
      }
    });
    const result = await f.update(ticket);
    assert.equal(result.ok, false); assert.equal(result.detail, 'share_canvas_update_unconfirmed');
    assert.deepEqual(result.events, []);
    assert.equal((await f.update(ticket)).ok, false); assert.equal(writes, 1);
  });
}

test('one total deadline bounds all three requests below the native command timeout', async () => {
  const f = setup(), ticket = (await f.list()).ticket;
  f.setHook(({ url }) => { if (url !== LIST) f.advance(6500); });
  const result = await f.update(ticket);
  assert.equal(result.ok, false); assert.equal(result.detail, 'share_canvas_update_unconfirmed');
  assert.deepEqual(result.events, []); assert.ok(f.requests.at(-1).limits.timeoutMs <= 5000);
});

for (const patch of [{ version: 1 }, { version: null }, { shared_textdoc_id: 'foreign' },
  { access: 'private' }, { is_moderation_blocked: true }, { is_anonify_api_key_detected: true }]) {
  test('unconfirmed POST response never produces a success event: ' + JSON.stringify(patch), async () => {
    const f = setup(), ticket = (await f.list()).ticket;
    f.setPostPatch(patch);
    const result = await f.update(ticket);
    assert.equal(result.ok, false); assert.equal(result.detail, 'share_canvas_update_unconfirmed');
    assert.deepEqual(result.events, []);
    assert.equal(f.requests.filter(r => r.init.method === 'POST').length, 1);
  });
}

test('account transition during preflight and exhausted budget cannot send a POST', async () => {
  for (const drift of ['account', 'deadline']) {
    const f = setup(), ticket = (await f.list()).ticket;
    f.setHook(() => {
      if (drift === 'account') f.headers.Authorization = 'Bearer synthetic-changed';
      else f.advance(18000);
    });
    assert.equal((await f.update(ticket)).ok, false);
    assert.ok(f.requests.every(r => r.init.method === 'GET'));
  }
});

test('repeat taps join neither a second publish nor a concurrent revoke', async () => {
  const f = setup(), ticket = (await f.list()).ticket;
  let release, entered;
  const waiting = new Promise(resolve => { release = resolve; });
  const started = new Promise(resolve => { entered = resolve; });
  f.setHook(async ({ init }) => { if (init.method === 'POST') { entered(); await waiting; } });
  const first = f.update(ticket);
  await started;
  assert.equal((await f.update(ticket)).detail, 'share_busy');
  const revoke = await f.command({ operation: 'revoke_account', resource: 'canvas', id: ID, ticket }, true);
  assert.equal(revoke.detail, 'share_busy');
  release();
  assert.equal((await first).ok, true);
  assert.equal(f.requests.filter(r => r.init.method === 'POST').length, 1);
  assert.equal(f.requests.filter(r => r.init.method === 'DELETE').length, 0);
});

test('a missing display channel prevents even a confirmed publish', async () => {
  const f = setup(), ticket = (await f.list()).ticket; let result;
  f.api.handle('share_conversation', { value: JSON.stringify({ operation: 'update_account', resource: 'canvas', id: ID, ticket }), selected: true },
    (_, ok, detail) => { result = { ok, detail }; }, () => f.snapshot);
  assert.deepEqual(result, { ok: false, detail: 'share_canvas_unavailable' }); assert.equal(f.requests.length, 1);
});
