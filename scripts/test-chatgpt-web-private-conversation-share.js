'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const contract = require(path.join(assets, 'chatgpt_web_private_conversation_share_contract.js'));
const share = require(path.join(assets, 'chatgpt_web_private_conversation_share.js'));
const CID = '11111111-1111-4111-8111-111111111111';
const NODE = '22222222-2222-4222-8222-222222222222';
const NEXT = '33333333-3333-4333-8333-333333333333';
const SID = '44444444-4444-4444-8444-444444444444';
const PATH = '/c/' + CID;
const LINK = 'https://chatgpt.com/share/' + SID;
const MODERATION = { has_been_auto_blocked: false, has_been_auto_moderated: false, has_been_blocked: false };

function fixture() {
  const requests = [];
  const headers = { Authorization: 'Bearer synthetic-test-only', 'chatgpt-account-id': 'synthetic-personal',
    'chatgpt-sentinel-proof-token': 'must-not-replay', cookie: 'must-not-export' };
  const thread = { isLoading: false, is_do_not_remember: false, contextScopes: [], leaf: NODE, node: NODE, gizmo: null };
  const account = { isQuorum: () => false, isWorkspaceAccount: () => false };
  const modules = {
    shared: { H3: () => true, mq: () => account, wV: selector => selector({ personal: true }),
      SV: { isPersonalWorkspace: value => value.personal }, XM: id => id === CID ? thread : null,
      HM: { getGizmoId: t => t.gizmo, getCurrentLeafId: t => t.leaf, hasNode: (t, id) => [NODE, NEXT].includes(id) } },
    conversation: { AGt: t => t.node },
  };
  const snapshot = { url: 'https://chatgpt.com' + PATH, composerReady: true, streaming: false, attachments: [] };
  let loaded = true;
  const page = {
    location: { origin: 'https://chatgpt.com', href: snapshot.url }, __elonChatGptDocumentToken: 'doc_share_test',
    __elonChatGptPrivateConversationMutationsEnabled: true,
    __elonChatGptPrivateTransport: { copySameOriginRequestHeaders: () => headers },
    performance: { getEntriesByName: () => loaded ? [{}] : [] }, document: { querySelector: () => null },
    setTimeout, clearTimeout,
    __elonChatGptPrivateJsonRequest: { request: async (_page, url, init, limits) => {
      requests.push({ url, init, limits });
      if (init.method === 'POST') return { payload: { share_id: SID, share_url: LINK, current_node_id: NODE,
        is_visible: true, is_public: false, is_anonymous: true, title: 'Synthetic fixture', moderation_state: MODERATION } };
      return { payload: { moderation_state: MODERATION } };
    } },
  };
  const loadRuntime = async url => url.includes('4813494d') ? modules.shared : modules.conversation;
  const api = share.create(page, { contract, loadRuntime });
  return { page, requests, headers, thread, modules, account, snapshot, api,
    start: (confirmed = true) => api.start(PATH, confirmed, () => snapshot),
    setLoaded: value => { loaded = value; }, loadRuntime };
}

test('confirmed full conversation creates then publishes exact selected node with no proof replay', async () => {
  const f = fixture(), result = await f.start();
  assert.deepEqual(result, { ok: true, code: 'share_link_ready', attempted: true, url: LINK });
  assert.equal(f.requests.length, 2);
  const [create, publish] = f.requests;
  assert.equal(create.url, '/backend-api/share/create');
  assert.equal(create.init.method, 'POST');
  assert.deepEqual(JSON.parse(create.init.body), { current_node_id: NODE, conversation_id: CID, is_anonymous: true });
  assert.equal(publish.url, '/backend-api/share/' + SID);
  assert.equal(publish.init.method, 'PATCH');
  assert.deepEqual(JSON.parse(publish.init.body), { title: 'Synthetic fixture', is_public: true,
    is_visible: true, is_anonymous: true, current_node_id: NODE });
  for (const { init, limits } of f.requests) {
    assert.equal(init.credentials, 'include'); assert.equal(init.redirect, 'error');
    assert.equal(init.headers.cookie, undefined); assert.equal(init.headers['chatgpt-sentinel-proof-token'], undefined);
    assert.equal(limits.timeoutMs, 7000); assert.equal(limits.maxBytes, 256 * 1024);
  }
});

test('no request or runtime load without native confirmation', async () => {
  const f = fixture();
  assert.equal((await f.start(false)).code, 'user_confirmation_required');
  assert.equal(f.requests.length, 0);
});

test('validated current-branch result reused without a second write', async () => {
  const f = fixture();
  await f.start(); await f.start();
  assert.equal(f.requests.length, 2);
});

for (const [name, mutate] of [
  ['different conversation', f => { f.page.location.href = 'https://chatgpt.com/c/' + NEXT; }],
  ['temporary route', f => { f.page.location.href += '?temporary-chat=true'; }],
  ['temporary store', f => { f.thread.is_do_not_remember = true; }],
  ['project store', f => { f.thread.gizmo = 'g-p-synthetic'; }],
  ['shared project owner', f => { f.thread.sharedProjectConversationOwner = { id: 'synthetic' }; }],
  ['shared project continuation', f => { f.thread.continuingFromSharedProjectConversationId = NEXT; }],
  ['health scope', f => { f.thread.contextScopes = ['HEALTH']; }],
  ['loading store', f => { f.thread.isLoading = true; }],
  ['business workspace', f => { f.account.isWorkspaceAccount = () => true; }],
  ['quorum account', f => { f.account.isQuorum = () => true; }],
  ['unknown personal scope', f => { f.modules.shared.wV = () => undefined; }],
  ['logged out', f => { f.modules.shared.H3 = () => false; }],
  ['runtime missing', f => { f.setLoaded(false); }],
  ['runtime export drift', f => { delete f.modules.conversation.AGt; }],
  ['invalid selected node', f => { f.thread.node = 'optimistic'; }],
  ['missing identity', f => { delete f.headers.Authorization; }],
  ['streaming', f => { f.snapshot.streaming = true; }],
  ['dictation pending', f => { f.snapshot.dictationCapturePending = true; }],
  ['snapshot missing', f => { f.snapshot.url = ''; }],
  ['feature switch off', f => { f.page.__elonChatGptPrivateConversationMutationsEnabled = false; }],
]) test(name + ' cannot authorize a public share write', async () => {
  const f = fixture(); mutate(f);
  const result = await f.start();
  assert.equal(result.ok, false); assert.equal(result.attempted, false); assert.equal(result.url, undefined);
  assert.equal(f.requests.length, 0);
});

test('existing native draft is not submitted or cleared by sharing', async () => {
  const f = fixture(); f.snapshot.draft = 'Unsent synthetic draft';
  assert.equal((await f.start()).ok, true);
  assert.equal(f.snapshot.draft, 'Unsent synthetic draft');
});

for (const phase of ['create', 'publish']) test('uncertain ' + phase + ' is not replayed or reported as success', async () => {
  const f = fixture(), request = f.page.__elonChatGptPrivateJsonRequest.request;
  f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
    const result = await request(...args);
    if ((phase === 'create') === (args[2].method === 'POST')) throw new Error('timeout');
    return result;
  };
  const result = await f.start();
  assert.equal(result.code, 'share_result_unconfirmed'); assert.equal(result.url, undefined);
  assert.equal((await f.start()).code, 'share_cooldown');
  assert.equal(f.requests.length, phase === 'create' ? 1 : 2);
});

for (const [name, mutate] of [
  ['invalid response node', value => { value.current_node_id = 'invalid'; }],
  ['share url mismatch', value => { value.share_url = 'https://chatgpt.com/share/' + NEXT; }],
  ['external url', value => { value.share_url = 'https://example.invalid/share/' + SID; }],
  ['url credentials', value => { value.share_url = 'https://user@chatgpt.com/share/' + SID; }],
  ['unknown visibility', value => { delete value.is_visible; }],
  ['deleted link', value => { value.is_visible = false; }],
  ['named identity', value => { value.is_anonymous = false; }],
  ['blocked creation', value => { value.moderation_state = { has_been_blocked: true }; }],
]) test(name + ' never reaches publish or exposes a URL', async () => {
  const f = fixture(), request = f.page.__elonChatGptPrivateJsonRequest.request;
  f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
    const result = await request(...args); mutate(result.payload); return result;
  };
  assert.equal((await f.start()).ok, false); assert.equal(f.requests.length, 1);
});

test('an existing link to an older node is updated to the selected current branch', async () => {
  const f = fixture(), request = f.page.__elonChatGptPrivateJsonRequest.request;
  f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
    const result = await request(...args);
    if (args[2].method === 'POST') {
      Object.assign(result.payload, { current_node_id: NEXT, already_exists: true, is_public: true });
    }
    return result;
  };
  assert.equal((await f.start()).ok, true);
  assert.equal(JSON.parse(f.requests[1].init.body).current_node_id, NODE);
});

test('renaming the conversation invalidates a recent share result', async () => {
  const f = fixture(); await f.start();
  f.thread.title = 'Renamed synthetic fixture';
  assert.equal((await f.start()).ok, true);
  assert.equal(f.requests.length, 4);
});

for (const moderation of [null, { has_been_blocked: 'false' }, { has_been_blocked: true }, { ...MODERATION, has_been_auto_moderated: true }]) {
  test('publication moderation must be confirmed before exposing URL: ' + JSON.stringify(moderation), async () => {
    const f = fixture(), request = f.page.__elonChatGptPrivateJsonRequest.request;
    f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
      const result = await request(...args);
      if (args[2].method === 'PATCH') result.payload.moderation_state = moderation;
      return result;
    };
    const result = await f.start(); assert.equal(result.ok, false); assert.equal(result.url, undefined);
  });
}

test('an explicit empty moderation object follows the official non-blocked predicate', async () => {
  const f = fixture(), request = f.page.__elonChatGptPrivateJsonRequest.request;
  f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
    const response = await request(...args);
    if (args[2].method === 'PATCH') response.payload.moderation_state = {};
    return response;
  };
  assert.equal((await f.start()).ok, true);
});

for (const [name, change] of [
  ['branch', f => { f.thread.leaf = NEXT; f.thread.node = NEXT; }],
  ['document', f => { f.page.__elonChatGptDocumentToken = 'doc_next_test'; }],
  ['identity', f => { f.headers.Authorization = 'Bearer changed-synthetic-test'; }],
  ['route', f => { f.page.location.href = 'https://chatgpt.com/'; }],
]) test(name + ' drift while create is in flight prevents publish and result distribution', async () => {
  const f = fixture(), request = f.page.__elonChatGptPrivateJsonRequest.request;
  f.page.__elonChatGptPrivateJsonRequest.request = async (...args) => {
    const response = await request(...args); change(f); return response;
  };
  const result = await f.start();
  assert.equal(result.code, 'share_result_unconfirmed'); assert.equal(f.requests.length, 1);
});

test('only one share transaction can run at a time', async () => {
  const f = fixture();
  const first = f.start(), second = await f.start();
  assert.equal(second.code, 'share_busy'); assert.equal((await first).ok, true);
});

test('delete and metadata mutation ownership exclude sharing', async () => {
  const f = fixture(); f.page.__elonChatGptPrivateConversationDelete = { busy: () => true };
  assert.equal((await f.start()).code, 'share_busy');
  f.page.__elonChatGptPrivateConversationDelete = null;
  f.page.__elonChatGptPrivateConversationMutation = { state: () => ({ state: 'busy' }) };
  assert.equal((await f.start()).code, 'share_busy'); assert.equal(f.requests.length, 0);
});

test('production command receipt carries only the validated result URL', async () => {
  const f = fixture();
  const receipt = await new Promise(resolve => {
    assert.equal(f.api.handle('share_conversation', { value: PATH, selected: true }, (...args) => resolve(args), () => f.snapshot), true);
  });
  assert.deepEqual(receipt, ['share_conversation', true, 'share_link_ready:' + LINK]);
  assert.ok(receipt[2].length < 160);
  assert.equal(f.api.handle('not_share', {}, () => {}), false);
});

test('share modules parse in the same concatenated asset scope and are registered', () => {
  const source = ['chatgpt_web_private_conversation_share_contract.js', 'chatgpt_web_private_conversation_share.js']
    .map(file => fs.readFileSync(path.join(assets, file), 'utf8')).join('\n');
  const f = fixture();
  vm.runInNewContext(source, { window: f.page, URL, setTimeout, clearTimeout });
  assert.equal(f.page.__elonChatGptPrivateConversationShare.version, 1);
  assert.equal(typeof f.page.__elonChatGptPrivateConversationShare.start, 'function');
  const owned = f.page.__elonChatGptPrivateConversationShare;
  vm.runInNewContext(source, { window: f.page, URL, setTimeout, clearTimeout });
  assert.equal(f.page.__elonChatGptPrivateConversationShare, owned);
});

test('production bundle includes the share contract before transport and command routing', () => {
  const kotlin = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  const names = [...kotlin.matchAll(/"(chatgpt_web_[a-z_]+\.js)"/g)].map(match => match[1]);
  const before = names.indexOf('chatgpt_web_private_conversation_share_contract.js');
  const after = names.indexOf('chatgpt_web_private_conversation_share.js');
  assert.ok(before >= 0 && after > before);
  new vm.Script(names.map(file => fs.readFileSync(path.join(assets, file), 'utf8')).join('\n'));
  const mutation = fs.readFileSync(path.join(assets, 'chatgpt_web_private_conversation_mutation.js'), 'utf8');
  assert.match(mutation, /PrivateConversationShare\?\.handle\(action, command, respond, readSnapshot\)/);
  assert.match(kotlin, /fun shareConversation[\s\S]*?action = "share_conversation"[\s\S]*?selected = true/);
});
