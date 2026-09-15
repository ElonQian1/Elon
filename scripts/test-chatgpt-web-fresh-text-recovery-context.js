'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture: ownerFixture } = require('./fixtures/chatgpt-private-text-input');
const { account, uid, memory, fixture: journalFixture } = require('./fixtures/chatgpt-fresh-text-journal');
const moduleApi = require('../android/app/src/main/assets/chatgpt_web_fresh_text_recovery_context');
const tick = () => new Promise(resolve => setImmediate(resolve));
const project = 'g-p-' + 'a'.repeat(32);

function fixture() {
  const f = ownerFixture(); f.identity(account); f.tree.is_do_not_remember = false;
  const bindings = f.page.__elonChatGptPrivateRuntimeBindings, oldPeek = bindings.peek, oldLoad = bindings.load;
  const forbidComposer = role => { if (role === 'composer') throw Error('unexpected_editor_dependency'); };
  bindings.peek = role => { forbidComposer(role); return oldPeek(role); };
  bindings.load = role => { forbidComposer(role); return oldLoad(role); };
  Object.assign(f.props, { isDisabled: true, isComposerSubmissionReady: false, currentLeafId: null });
  f.shared.HM.getCurrentMessage = () => null;
  f.conversation.Nrn = () => { throw Error('unexpected_model_dependency'); };
  f.page.localStorage = memory();
  const controller = new AbortController();
  return { ...f, bindings, controller, capture: () => moduleApi.create(f.page).capture(controller.signal) };
}

test('read-only context captures a committed conversation without editor, model or parent readiness', async () => {
  const f = fixture(), binding = await f.capture();
  assert.equal(binding.conversationId, f.selected.serverId$());
  assert.equal(binding.recoveryIdentity(), account);
  assert.equal(binding.owns(), true);
  assert.equal(binding.runtime, f.conversation);
  assert.equal(binding.draft, undefined);
  assert.deepEqual(f.edits, []);
  assert.equal(f.page.localStorage.length, 0);
});

test('recovery scope follows committed conversation and account without reading the editor', () => {
  const f = fixture(), context = moduleApi.create(f.page), before = context.stamp();
  assert.ok(before);
  f.selected.id = 'fixture-new-selection'; assert.notEqual(context.stamp(), before);
  f.identity(JSON.stringify(['other-user', 'other-account'])); assert.notEqual(context.stamp(), before);
  assert.deepEqual(f.edits, []);
});

for (const phase of ['armed', 'inFlight', 'takeoverActive']) {
  test(`native voice ${phase} defers text journal recovery without touching the media session`, async () => {
    const f = fixture();
    f.page.__elonChatGptPrivateVoiceRelay = { state: () => JSON.stringify({ [phase]: true }) };
    await assert.rejects(f.capture(), /context_unavailable/);
    assert.equal(f.page.localStorage.length, 0);
  });
}

for (const [name, mutate] of [
  ['document', f => { f.page.document = {}; }],
  ['document token', f => { f.page.__elonChatGptDocumentToken = 'doc_replaced'; }],
  ['route', f => { f.page.location.href = 'https://chatgpt.com/c/' + uid(6); }],
  ['account', f => { f.identity(JSON.stringify(['other-user', 'other-account'])); }],
  ['profile', f => { f.bindings.state = () => ({ profile_id: 'web_20260915' }); }],
  ['committed root', f => { f.root.stateNode.current = {}; }],
  ['controller', f => { f.props.composerController = {}; }],
  ['registry', f => { f.shared.canvasConversations = () => []; }],
  ['ambiguous registry', f => { f.shared.canvasConversations = () => [f.selected, f.selected]; }],
  ['project scope', f => { f.shared.HM.getGizmoId = () => project; }],
  ['privacy', f => { f.tree.is_do_not_remember = true; }],
  ['official request', f => { f.shared.Fl = () => true; }],
  ['native voice relay', f => { f.page.__elonChatGptPrivateVoiceRelay = { state: () => JSON.stringify({ takeoverActive: true }) }; }],
  ['cancelled command', f => { f.controller.abort(); }],
]) {
  test(`${name} changes during runtime loading invalidate the recovery owner`, async () => {
    const f = fixture(); let resolve;
    f.bindings.load = role => {
      assert.equal(role, 'conversation'); return new Promise(done => { resolve = done; });
    };
    const capture = f.capture(), rejected = assert.rejects(capture, /context_changed/);
    await tick(); mutate(f); resolve(f.conversation); await rejected;
    assert.deepEqual(f.edits, []);
  });
}

test('fresh home owner is available to report an unresolved ID without guessing a conversation', async () => {
  const f = fixture(); f.page.location.href = 'https://chatgpt.com/';
  f.selected.serverId$ = () => null; f.props.isNewThread = true;
  const binding = await f.capture();
  assert.equal(binding.conversationId, null); assert.equal(binding.newConversation, true);
  assert.equal(binding.owns(), true);
});

test('an owned project retains its exact scope and rejects an unrelated route', async () => {
  const f = fixture(); f.shared.HM.getGizmoId = () => project;
  f.tree.mode = { kind: 'gizmo_interaction', gizmo_id: project };
  f.page.location.href = `https://chatgpt.com/g/${project}/c/${f.selected.serverId$()}`;
  assert.equal((await f.capture()).projectId, project);
  f.page.location.href = f.page.location.href.replace(project, 'g-p-' + 'b'.repeat(32));
  await assert.rejects(f.capture(), /context_unavailable/);
});

test('temporary and history-disabled chats never need a persisted recovery owner', async () => {
  const f = fixture(); f.page.location.href = 'https://chatgpt.com/?temporary-chat=true';
  assert.equal(await f.capture(), null);
  f.page.location.href = 'https://chatgpt.com/c/' + f.selected.serverId$();
  f.shared.textHistoryDisabled = () => true;
  assert.equal(await f.capture(), null);
  assert.equal(f.page.localStorage.length, 0);
});

test('an unknown runtime or absent account is not treated as an empty journal', async () => {
  const f = fixture(); f.bindings.observed = () => false;
  await assert.rejects(f.capture(), /runtime_unavailable/);
  f.bindings.observed = () => true; f.identity(null);
  await assert.rejects(f.capture(), /recovery_identity_unavailable/);
});

test('provider-side streaming makes a previously captured read owner ineligible', async () => {
  const f = fixture(), binding = await f.capture();
  f.shared.Fx = () => ({ value: f.shared.v7.STREAMING });
  assert.equal(binding.owns(), false); assert.equal(binding.recoveryIdentity(), null);
});

test('production registers the read-only recovery context before the sender', () => {
  const source = require('node:fs').readFileSync(require('node:path').join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebAdapterAssets.kt'), 'utf8');
  const entries = [...source.matchAll(/"(chatgpt_web_[a-z_]+\.js)"/g)].map(match => match[1]);
  const module = 'chatgpt_web_fresh_text_recovery_context.js';
  assert.equal(entries.filter(value => value === module).length, 1);
  assert.ok(entries.indexOf(module) < entries.indexOf('chatgpt_web_fresh_text_transaction.js'));
});

test('the real journal accepts the read-only context and settles only exact terminal history', async () => {
  const f = fixture(), j = journalFixture();
  (await j.prepare()).persist();
  f.selected.serverId$ = () => j.binding.conversationId;
  f.page.location.href = 'https://chatgpt.com/c/' + j.binding.conversationId;
  f.conversation.textHydrateHistory = j.binding.runtime.textHydrateHistory;
  const binding = await f.capture();
  assert.equal(binding.current, undefined);
  assert.equal(await j.api.recoverSelected(binding, f.controller.signal), 1);
  assert.equal(j.rows().length, 0);
  assert.deepEqual(j.calls.map(call => call.kind), ['history', 'apply']);
  assert.deepEqual(f.edits, []);
});
