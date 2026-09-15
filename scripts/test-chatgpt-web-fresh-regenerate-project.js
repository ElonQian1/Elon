'use strict';
const test = require('node:test'), assert = require('node:assert/strict'), crypto = require('node:crypto');
const { projectFixture, PROJECT, CID, UID, AID, OTHER } = require('./fixtures/chatgpt-fresh-regeneration');
const requests = require('../android/app/src/main/assets/chatgpt_web_fresh_text_request');
const history = require('../android/app/src/main/assets/chatgpt_web_fresh_text_reconcile').create();
const enabled = { allowExistingProjects: true };

for (const path of ['/c/' + CID, '/g/' + PROJECT + '/c/' + CID, '/g/' + PROJECT + '-fixture/c/' + CID]) {
  test('existing project retry retains project and original question on route ' + path.split('/').length, async () => {
    const f = projectFixture(path);
    await assert.rejects(f.capture(), /scope_unsupported/);
    const b = await f.capture(enabled);
    assert.equal(b.projectId, PROJECT); assert.equal(b.parentId, UID); assert.equal(b.current(), true);
    assert.equal(b.projectUserId, f.retry.identity.userId);
    const request = requests.create({ crypto }).create(b, f.command);
    const mode = { kind: 'gizmo_interaction', gizmo_id: PROJECT };
    assert.equal(request.userMessageId, UID);
    assert.deepEqual(request.preparationBody().conversation_mode, mode);
    assert.deepEqual(request.securityMetadata().conversationMode, mode);
    assert.equal(request.preparationBody().action, 'next');
    const sent = request.consume({ conduit_token: 'synthetic-conduit' }, { chatReq: { token: 'synthetic-proof' } },
      value => ({ 'openai-sentinel-chat-requirements-token': value.token }), b.current);
    assert.equal(sent.body.action, 'variant'); assert.equal(sent.body.parent_message_id, UID);
    assert.deepEqual(sent.body.conversation_mode, mode); assert.equal('messages' in sent.body, false);
    assert.equal('project_instructions' in sent.body, false); assert.equal('gizmo' in sent.body.conversation_mode, false);
    assert.equal(f.retry.calls.length, 0);
  });
}

test('project flag and admission share the same bounded scope without sending', async () => {
  const f = projectFixture();
  assert.equal((await f.inspect()).code, 'scope_unsupported');
  assert.equal((await f.inspect(enabled)).code, 'ready');
  f.page.__elonChatGptFreshRegenerationProjectsEnabled = true;
  assert.equal((await f.inspect()).code, 'ready');
  f.page.__elonChatGptFreshRegenerationProjectsEnabled = false;
  assert.equal((await f.inspect()).code, 'scope_unsupported');
  assert.equal(f.retry.calls.length, 0);
});

test('read-only project admission recognizes but never consumes the one-command trial', async () => {
  const f = projectFixture(), controls = []; let armed = true;
  f.page.__elonChatGptFreshTextTransaction = { trialControl(control) {
    controls.push(control); return { armed };
  } };
  assert.equal((await f.inspect()).code, 'ready');
  assert.equal(armed, true); assert.deepEqual(controls, ['state']);
  assert.equal(Object.hasOwn(f.page, '__elonChatGptFreshRegenerationProjectsEnabled'), false);
  armed = false;
  assert.equal((await f.inspect()).code, 'scope_unsupported');
  assert.equal(f.retry.calls.length, 0);
});

for (const [name, mutate] of [
  ['foreign project route', f => { f.page.location.href = f.binding.href = 'https://chatgpt.com/g/g-p-' + 'b'.repeat(32) + '/c/' + CID; }],
  ['another owner', f => { f.tree.sharedProjectConversationOwner.id = 'other-user'; }],
  ['shared continuation', f => { f.tree.continuingFromSharedProjectConversationId = OTHER; }],
  ['restricted project', f => { f.tree.contextScopes = ['HEALTH']; }],
  ['business agent', f => { f.shared.textBusinessContext = () => ({ businessAgentId: 'synthetic' }); }],
  ['locked project', f => { f.shared.textProjectHeaders = () => ({ 'x-openai-locked-chats-pin': 'synthetic-only' }); }],
  ['loading project', f => { f.tree.isLoading = true; }],
  ['private mode', f => { f.tree.is_do_not_remember = true; }],
  ['file parent', f => { f.user.message.metadata.attachments = [{ id: 'file-synthetic' }]; }],
  ['tool parent', f => { f.user.message.metadata.system_hints = ['search']; }],
  ['search answer', f => { f.parent.metadata.map_search_parameters = {}; }]
]) test(name + ' is not authorized by the plain project retry option', async () => {
  const f = projectFixture(); mutate(f);
  await assert.rejects(f.capture(enabled)); assert.equal(f.retry.calls.length, 0);
});

for (const [name, mutate] of [
  ['project move', f => { f.tree.mode.gizmo_id = 'g-p-' + 'b'.repeat(32); }],
  ['route change', f => { f.page.location.href = 'https://chatgpt.com/c/' + OTHER; }],
  ['account change', f => { f.retry.identity.accountId = OTHER; }],
  ['project scope change', f => { f.tree.contextScopes = ['HEALTH']; }]
]) test(name + ' invalidates captured retry and late native projection', async () => {
  const f = projectFixture(), b = await f.capture(enabled); mutate(f);
  assert.equal(b.current(), false); assert.equal(b.owns(), false);
  assert.equal(b.observePayload({ message: f.reply() }), false);
  assert.equal(b.canReconcile(UID), false); assert.equal(b.canStop(UID), false);
});

test('project retry history must match project, account and this stream variant', async () => {
  const f = projectFixture(), b = await f.capture(enabled);
  assert.equal(b.observePayload({ message: f.reply() }), true);
  assert.equal(history.ownsResponse(f.history(), b, UID), true);
  for (const mutate of [
    value => { delete value.gizmo_id; }, value => { value.gizmo_id = 'g-p-' + 'b'.repeat(32); },
    value => { value.owner.user_id = 'other-user'; }, value => { value.context_scopes = ['HEALTH']; },
    value => { value.shared_project_conversation_owner = {}; }
  ]) {
    const value = f.history(); mutate(value); assert.equal(history.ownsResponse(value, b, UID), false);
  }
  assert.equal(history.ownsResponse(f.history(OTHER), b, UID), false);
  f.apply(f.history(), true);
  assert.equal(b.selectVerifiedReply(UID, AID), true);
  assert.deepEqual(f.branchSelections, [AID]);
});
