'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { fixture, CID, NODE, NEXT, PATH } = require('./fixtures/chatgpt-web-private-conversation-share.cjs');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const contract = require(path.join(assets, 'chatgpt_web_private_conversation_share_contract.js'));
const share = require(path.join(assets, 'chatgpt_web_private_conversation_share.js'));
const file = path.join(assets, 'chatgpt_web_private_project_conversation_share.js');
const project = fs.existsSync(file) ? require(file) : null;
const PID = 'g-p-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';
const OTHER = 'g-p-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb';
const TARGET = '/g/' + PID + '/c/' + CID;
const OWNER = 'user-synthetic-member';
const link = (owner = OWNER, slug = PID + '-fixture') =>
  'https://chatgpt.com/g/' + slug + '/shared/c/' + CID + '?owner_user_id=' + owner;

function member() {
  const f = fixture();
  f.thread.gizmo = PID;
  f.account.normalizedAccountUserId = OWNER;
  f.modules.shared.Pz = { Private: 'private', User: 'user', Group: 'group', Email: 'email',
    WorkspaceLink: 'workspace_link', Workspace: 'workspace', Link: 'link', Marketplace: 'marketplace' };
  f.modules.shared.nz = (id, payload, owner) => 'https://chatgpt.com/g/' +
    (payload.gizmo.short_url ?? payload.gizmo.id) + '/shared/c/' + id + '?owner_user_id=' + owner;
  const payload = { gizmo: { id: PID, short_url: PID + '-fixture', share_recipient: 'user' } };
  let afterRead = () => {};
  f.page.__elonChatGptPrivateJsonRequest.request = async (_page, url, init, limits) => {
    f.requests.push({ url, init, limits });
    await afterRead();
    return { payload };
  };
  f.api = share.create(f.page, { contract, loadRuntime: f.loadRuntime, project });
  f.start = (confirmed = true, target = TARGET) => f.api.start(target, confirmed, () => f.snapshot);
  return Object.assign(f, { payload, afterRead: action => { afterRead = action; } });
}

test('member share uses one bounded GET and official URL builder, never public creation', async () => {
  const f = member();
  assert.deepEqual(await f.start(), { ok: true, code: 'project_share_link_ready', attempted: false, url: link() });
  assert.equal(f.requests.length, 1);
  const { url, init, limits } = f.requests[0];
  assert.equal(url, '/backend-api/gizmos/' + PID);
  assert.equal(init.method, 'GET'); assert.equal(init.credentials, 'same-origin');
  assert.equal(init.redirect, 'error'); assert.equal(init.body, undefined);
  assert.equal(limits.timeoutMs, 7000); assert.equal(limits.maxBytes, 1024 * 1024);
  assert.equal(init.headers.cookie, undefined);
  assert.equal(init.headers['chatgpt-sentinel-proof-token'], undefined);
});

test('member selection accepts the canonical or slugged project URL for this exact thread', async () => {
  for (const route of [TARGET, TARGET.replace(PID, PID + '-fixture')]) {
    const f = member(); f.page.location.href = f.snapshot.url = 'https://chatgpt.com' + route;
    assert.equal((await f.start()).url, link());
  }
});

test('member link binds official shared owner while keeping the current server conversation', async () => {
  const f = member(); f.thread.sharedProjectConversationOwner = { id: 'user-synthetic-original' };
  f.thread.continuingFromSharedProjectConversationId = NEXT;
  assert.equal((await f.start()).url, link('user-synthetic-original'));
});

test('short URL is optional and legacy nonempty sharing subjects prove member scope', async () => {
  const f = member(); delete f.payload.gizmo.short_url; delete f.payload.gizmo.share_recipient;
  f.payload.gizmo.sharing = { subjects: [{ type: 'user' }] };
  assert.equal((await f.start()).url, link(OWNER, PID));
});

test('same context reuses the member link without a second metadata read', async () => {
  const f = member(); assert.equal((await f.start()).ok, true);
  assert.equal((await f.start()).ok, true); assert.equal(f.requests.length, 1);
  f.page.__elonChatGptDocumentToken = 'doc_project_next';
  assert.equal((await f.start()).ok, true); assert.equal(f.requests.length, 2);
});

test('cache expiry requires a fresh permission read', async () => {
  const f = member(); await f.start();
  const now = Date.now;
  try { Date.now = () => now() + 61000; await f.start(); } finally { Date.now = now; }
  assert.equal(f.requests.length, 2);
});

test('no member request without confirmation and no implicit public consent for a project', async () => {
  const f = member(); assert.equal((await f.start(false)).code, 'user_confirmation_required');
  assert.equal((await f.start(true, PATH)).ok, false); assert.equal(f.requests.length, 0);
});

for (const [name, alter] of Object.entries({
  workspace: f => { f.account.isWorkspaceAccount = () => true; },
  quorum: f => { f.account.isQuorum = () => true; },
  signedOut: f => { f.modules.shared.H3 = () => false; },
  missingOwner: f => { delete f.account.normalizedAccountUserId; },
  privateTokenMissing: f => { delete f.headers.Authorization; },
  temporary: f => { f.thread.is_do_not_remember = true; },
  wrongProject: f => { f.thread.gizmo = OTHER; },
  health: f => { f.thread.contextScopes = ['HEALTH']; },
  loading: f => { f.thread.isLoading = true; },
  noRuntime: f => { f.setLoaded(false); },
  noBuilder: f => { delete f.modules.shared.nz; },
  badEnums: f => { f.modules.shared.Pz.Private = 'unknown'; },
  streaming: f => { f.snapshot.streaming = true; },
  dictation: f => { f.snapshot.dictationCapturePending = true; },
  external: f => { f.page.location.href = f.snapshot.url = 'https://example.com' + PATH; },
  otherRoute: f => { f.page.location.href = f.snapshot.url = 'https://chatgpt.com/g/' + OTHER + '/c/' + CID; },
})) test('member precondition rejects ' + name + ' without issuing a request', async () => {
  const f = member(); alter(f); assert.equal((await f.start()).ok, false); assert.equal(f.requests.length, 0);
});

for (const [name, alter] of Object.entries({
  privateProject: g => { g.share_recipient = 'private'; },
  noMembers: g => { delete g.share_recipient; g.sharing = { subjects: [] }; },
  malformedRecipient: g => { g.share_recipient = {}; },
  unknownRecipient: g => { g.share_recipient = 'unrecognized'; },
  wrongId: g => { g.id = OTHER; },
  wrongSlug: g => { g.short_url = OTHER + '-fixture'; },
  querySlug: g => { g.short_url = PID + '?other=1'; },
  healthProject: g => { g.gizmo_snorlax_type = 'potion'; },
})) test('readback rejects ' + name + ' without any public or permission write', async () => {
  const f = member(); alter(f.payload.gizmo); const result = await f.start();
  assert.equal(result.ok, false); assert.equal(result.attempted, false);
  assert.equal(f.requests.length, 1); assert.equal(f.requests[0].init.method, 'GET');
});

for (const [name, alter] of Object.entries({
  document: f => { f.page.__elonChatGptDocumentToken = 'doc_changed'; },
  account: f => { f.headers.Authorization = 'Bearer synthetic-other-user'; },
  owner: f => { f.thread.sharedProjectConversationOwner = { id: 'user-other-owner' }; },
  normalizedUser: f => { f.account.normalizedAccountUserId = 'user-other-account'; },
  project: f => { f.thread.gizmo = OTHER; },
  branch: f => { f.thread.leaf = NEXT; },
  route: f => { f.page.location.href = 'https://chatgpt.com/c/' + NEXT; },
  inheritedConversation: f => { f.thread.continuingFromSharedProjectConversationId = NEXT; },
})) test('late metadata cannot publish after ' + name + ' changes', async () => {
  const f = member(); f.afterRead(() => alter(f)); const result = await f.start();
  assert.equal(result.ok, false); assert.equal(result.url, undefined); assert.equal(f.requests.length, 1);
});

test('official builder output is validated rather than passed through', async () => {
  for (const bad of ['https://example.com/', link() + '&token=secret', link().replace(CID, NEXT),
    link().replace(OWNER, 'user-other'), link().replace('/shared/c/', '/c/')]) {
    const f = member(); f.modules.shared.nz = () => bad;
    assert.equal((await f.start()).url, undefined);
  }
});

test('one active read blocks duplicate share and failure never falls into public POST', async () => {
  const f = member(); let release, entered;
  const ready = new Promise(resolve => { entered = resolve; });
  f.afterRead(() => { entered(); return new Promise(resolve => { release = resolve; }); });
  const pending = f.start();
  await Promise.race([ready, pending.then(() => { throw new Error('metadata_read_not_started'); })]);
  assert.equal((await f.start()).code, 'share_busy'); release(); assert.equal((await pending).ok, true);
  const failed = member(); failed.afterRead(() => { throw new Error('http_403'); });
  const result = await failed.start(); assert.equal(result.ok, false); assert.equal(result.attempted, false);
  assert.equal(failed.requests.length, 1); assert.equal(failed.requests[0].init.method, 'GET');
});

test('member command carries a distinct audience receipt', async () => {
  const f = member();
  const receipt = await new Promise(resolve => f.api.handle('share_conversation',
    { value: TARGET, selected: true }, (...args) => resolve(args), () => f.snapshot));
  assert.deepEqual(receipt, ['share_conversation', true, 'project_share_link_ready:' + link()]);
});

test('production bundle registers member scope before the shared command owner', () => {
  const kotlin = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  const names = [...kotlin.matchAll(/"(chatgpt_web_[a-z_]+\.js)"/g)].map(match => match[1]);
  const index = names.indexOf('chatgpt_web_private_project_conversation_share.js');
  assert.ok(index > names.indexOf('chatgpt_web_private_conversation_share_contract.js'));
  assert.ok(index < names.indexOf('chatgpt_web_private_conversation_share.js'));
  new (require('node:vm').Script)(names.map(name => fs.readFileSync(path.join(assets, name), 'utf8')).join('\n'));
});
