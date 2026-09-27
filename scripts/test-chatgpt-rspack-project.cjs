'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture } = require('./fixtures/chatgpt-rspack.cjs');
const project = 'g-p-' + 'a'.repeat(32);
const other = 'g-p-' + 'b'.repeat(32);
const conversation = '22222222-2222-4222-8222-222222222222';
function projectFixture(existing = false) {
  const f = fixture({ latest: 'current' });
  f.page.location = new URL('https://chatgpt.com/g/' + project + (existing ? '/c/' + conversation : '/project'));
  f.values.set(f.conversation.K, project);
  if (existing) f.values.set(f.conversation.i, conversation);
  return f;
}
for (const existing of [false, true]) {
  test('project send carries the committed identity, existing=' + existing, async () => {
    const f = projectFixture(existing);
    assert.equal((await f.submit.inspect(f.editor)).code, 'ready');
    assert.equal((await f.submit.submit(f.command).completion).status, 'accepted');
    assert.equal(f.calls.length, 1);
    assert.equal(f.calls[0][1].projectId, project);
    assert.equal(f.calls[0][1].isTemporaryChat, false);
    const binding = f.context.read(f.editor);
    f.values.set(f.conversation.K, other);
    assert.equal(f.context.owns(binding), false);
    f.page.__elonChatGptGroupRequestOwnershipEnabled = true;
    assert.equal(f.context.ownedRequestCurrent(binding), false);
  });
}
for (const kind of ['missing', 'other', 'malformed', 'temporary', 'work', 'account']) {
  test('rejects project ' + kind + ' before dispatch', async () => {
    const f = projectFixture();
    if (kind === 'missing') f.values.set(f.conversation.K, null);
    if (kind === 'other') f.values.set(f.conversation.K, other);
    if (kind === 'malformed') f.values.set(f.conversation.K, 'g-p-fixture');
    if (kind === 'temporary') f.page.location.search = '?temporary-chat=true';
    if (kind === 'work') f.values.set(f.conversation.w, 'tpp');
    if (kind === 'account') f.changeAccount();
    assert.equal((await f.submit.submit(f.command).completion).status, 'rejected');
    assert.equal(f.calls.length, 0);
    assert.equal((await f.submit.inspect(f.editor)).schema, 'elon.fresh_text_admission.v1');
  });
}
test('project replacement during preparation cannot redirect a send', async () => {
  const f = projectFixture();
  f.command.beforeSubmit = () => f.values.set(f.conversation.K, other);
  assert.equal((await f.submit.submit(f.command).completion).status, 'rejected');
  assert.equal(f.calls.length, 0);
});
test('new local project needs matching committed ancestry before its atom exists', async () => {
  const f = projectFixture();
  f.values.set(f.conversation.K, null);
  f.parent.memoizedProps.projectId = project;
  f.officialSubmit.a = async (scope, options) => {
    f.calls.push([scope, options]);
    assert.equal(options.isSubmissionCurrent(), true);
    f.values.set(f.conversation.K, project);
    assert.equal(options.isSubmissionCurrent(), true);
    return true;
  };
  assert.equal((await f.submit.submit(f.command).completion).status, 'accepted');
  assert.equal(f.calls[0][1].projectId, project);
});
test('new project rejects ambiguous or mismatched committed ancestry', async () => {
  for (const ambiguous of [false, true]) {
    const f = projectFixture();
    f.values.set(f.conversation.K, null);
    f.parent.memoizedProps.projectId = ambiguous ? project : other;
    if (ambiguous) f.root.memoizedProps.projectId = other;
    assert.equal((await f.submit.submit(f.command).completion).status, 'rejected');
    assert.equal(f.calls.length, 0);
  }
});
test('project conversation permits its exact server navigation after remount', async () => {
  const f = projectFixture();
  await f.page.__elonChatGptRspackRuntime.load();
  const binding = f.context.capture(f.editor);
  f.values.set(f.conversation.i, conversation);
  f.page.location = new URL('https://chatgpt.com/g/' + project + '/c/' + conversation);
  assert.equal(f.context.requestCurrent(binding, conversation), true);
  f.page.location = new URL('https://chatgpt.com/g/' + other + '/c/' + conversation);
  assert.equal(f.context.requestCurrent(binding, conversation), false);
});
