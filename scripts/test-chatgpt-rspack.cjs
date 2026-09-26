'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, files, CDN } = require('./fixtures/chatgpt-rspack.cjs');

test('reviewed loader imports one observed runtime and reads only executed cache', async () => {
  const f = fixture(), runtime = f.page.__elonChatGptRspackRuntime;
  assert.equal(runtime.observed(), true);
  assert.equal(runtime.peek(), null);
  const [first, second] = await Promise.all([runtime.load(), runtime.load()]);
  assert.equal(first, second);
  assert.equal(first.scope.a, f.scope.scope);
  assert.equal(f.imports.length, 1);
  assert.equal(runtime.peek().submit, f.officialSubmit);
});

for (const kind of ['unobserved', 'unreviewed', 'unloaded', 'module_error', 'wrong_scope', 'document_changed']) {
  test('rejects runtime ' + kind + ' without executing require or sending', async () => {
    const f = fixture(), runtime = f.page.__elonChatGptRspackRuntime;
    if (kind === 'unobserved') f.page.performance.getEntriesByType = () => [];
    if (kind === 'unreviewed') f.page.performance.getEntriesByType = () => [{ name: CDN + files[0] }];
    if (kind === 'unloaded') delete f.cache.CUv;
    if (kind === 'module_error') f.cache.CUv.error = Error('fixture');
    if (kind === 'wrong_scope') f.composer.r.scope = {};
    const promise = runtime.load();
    if (kind === 'document_changed') f.page.__elonChatGptDocumentToken = 'doc_replaced';
    assert.equal(await promise, null);
    assert.equal(f.calls.length, 0);
  });
}

test('captures only the committed AppScope and excludes credentials from receipts', async () => {
  const f = fixture();
  await f.page.__elonChatGptRspackRuntime.load();
  const binding = f.context.capture(f.editor);
  assert.equal(binding.scope, f.scope);
  assert.equal(f.context.current(binding), true);
  assert.equal(f.context.owns(binding), true);
  assert.equal(binding.accessToken, undefined);
  const probe = await f.submit.inspect(f.editor);
  assert.deepEqual(probe, { profile: 'web_20260925_rspack', stage: 'ready', code: 'ready' });
});

for (const kind of ['uncommitted', 'detached', 'identity', 'account_switch', 'busy', 'upload', 'tools', 'project', 'route', 'server', 'ambiguous']) {
  test('rejects unsafe context ' + kind, async () => {
    const f = fixture();
    await f.page.__elonChatGptRspackRuntime.load();
    if (kind === 'uncommitted') f.root.stateNode.current = {};
    if (kind === 'detached') f.editor.isConnected = false;
    if (kind === 'identity') f.changeAccount();
    if (kind === 'account_switch') f.switchAccount();
    if (kind === 'busy') f.values.set(f.conversation.T, 'streaming');
    if (kind === 'upload') f.values.set(f.composer.f, [{ status: 'ready' }]);
    if (kind === 'tools') f.values.set(f.composer.u, ['search']);
    if (kind === 'project') f.values.set(f.conversation.K, 'g-p-fixture');
    if (kind === 'route') f.page.location = new URL('https://chatgpt.com/share/fixture');
    if (kind === 'server') f.values.set(f.conversation.i, 'another-thread');
    if (kind === 'ambiguous') f.root.memoizedProps = { conversationId: '22222222-2222-4222-8222-222222222222' };
    assert.equal(f.context.capture(f.editor), null);
    assert.equal(f.calls.length, 0);
  });
}

test('official composer transaction owns model, temporary mode and stream dispatch', async () => {
  const f = fixture();
  f.page.location = new URL('https://chatgpt.com/?temporary-chat=true');
  f.officialSubmit.a = async (scope, options) => {
    f.calls.push([scope, options]);
    f.values.set(f.composer.h, true);
    // Own busy state must not invalidate the request after invocation.
    assert.equal(options.isSubmissionCurrent(), true);
    return true;
  };
  const result = f.submit.submit(f.command);
  assert.equal(result.handled, true);
  assert.equal((await result.completion).status, 'accepted');
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0][0], f.scope);
  assert.equal(f.calls[0][1].isTemporaryChat, true);
  assert.equal(f.calls[0][1].requireDispatchAcceptance, true);
  assert.equal(f.calls[0][1].selectedModel.slug, 'auto');
  assert.equal(f.submit.state().pending, false);
});

for (const kind of ['draft', 'native_attachment', 'preparation_change']) {
  test('pre-dispatch ' + kind + ' cannot write', async () => {
    const f = fixture();
    if (kind === 'draft') f.values.set(f.composer.r, 'user unsent draft');
    if (kind === 'native_attachment') f.command.requireNativeAttachment = true;
    if (kind === 'preparation_change') f.command.beforeSubmit = f.changeAccount;
    assert.equal((await f.submit.submit(f.command).completion).status, 'rejected');
    assert.equal(f.calls.length, 0);
    assert.equal(f.submit.state().pending, false);
  });
}

test('an empty rich-text editor line break is not a changed draft', async () => {
  const f = fixture();
  f.command.readDraft = () => '\n';
  assert.equal((await f.submit.submit(f.command).completion).status, 'accepted');
  assert.equal(f.calls.length, 1);
});

test('empty-editor normalization never permits a nonempty official or visible draft', async () => {
  for (const stored of [false, true]) {
    const f = fixture();
    f.command.readDraft = () => stored ? '\n' : '\nnot empty\n';
    if (stored) f.values.set(f.composer.r, 'not empty');
    assert.equal((await f.submit.submit(f.command).completion).code, 'draft_mismatch');
    assert.equal(f.calls.length, 0);
  }
});

for (const kind of ['false', 'throw', 'timeout']) {
  test('ambiguous ' + kind + ' retains single-flight and never replays', async () => {
    const f = fixture({ timeoutMs: 5 });
    f.officialSubmit.a = (_scope, options) => {
      f.calls.push(options);
      if (kind === 'throw') throw Error('untrusted detail');
      return kind === 'false' ? Promise.resolve(false) : new Promise(() => {});
    };
    const result = f.submit.submit(f.command);
    assert.equal((await result.completion).status, 'unknown');
    assert.equal(f.submit.state().pending, true);
    assert.equal((await f.submit.submit(f.command).completion).code, 'busy');
    assert.equal(f.calls.length, 1);
    if (kind === 'timeout') assert.equal(f.calls[0].signal.aborted, true);
  });
}

test('account switch after invocation invalidates request guard', async () => {
  const f = fixture();
  f.officialSubmit.a = async (_scope, options) => {
    f.changeAccount();
    assert.equal(options.isRequestCurrent(), false);
    return true;
  };
  assert.equal((await f.submit.submit(f.command).completion).current, false);
});

test('official server alias keeps the stream current after the editor remounts', async () => {
  const f = fixture(), serverId = '22222222-2222-4222-8222-222222222222';
  let request;
  f.officialSubmit.a = async (_scope, options) => {
    request = options;
    f.values.set(f.conversation.i, serverId);
    f.parent.memoizedProps.conversationId = serverId;
    const editor = { ...f.editor };
    f.editor.isConnected = false;
    f.page.document.querySelectorAll = selector => selector === '#prompt-textarea' ? [editor] : [];
    options.onServerThreadIdChange(serverId);
    assert.equal(options.isRequestCurrent(), true);
    return true;
  };
  assert.equal((await f.submit.submit(f.command).completion).status, 'accepted');
  assert.equal(request.isRequestCurrent(), true);
  f.page.location = new URL('https://chatgpt.com/c/' + serverId);
  assert.equal(request.isRequestCurrent(), true);
  f.changeAccount();
  assert.equal(request.isRequestCurrent(), false);
});

for (const drift of ['alias', 'owner', 'route', 'document', 'account', 'ambiguous']) {
  test('server alias does not authorize ' + drift + ' drift', async () => {
    const f = fixture(), serverId = '22222222-2222-4222-8222-222222222222';
    await f.page.__elonChatGptRspackRuntime.load();
    const binding = f.context.capture(f.editor);
    f.values.set(f.conversation.i, serverId);
    f.parent.memoizedProps.conversationId = serverId;
    assert.equal(f.context.requestCurrent(binding, serverId), true);
    if (drift === 'alias') f.values.set(f.conversation.i, '33333333-3333-4333-8333-333333333333');
    if (drift === 'owner') f.parent.memoizedProps.conversationId = '33333333-3333-4333-8333-333333333333';
    if (drift === 'route') f.page.location = new URL('https://chatgpt.com/c/33333333-3333-4333-8333-333333333333');
    if (drift === 'document') f.page.__elonChatGptDocumentToken = 'doc_changed';
    if (drift === 'account') f.changeAccount();
    if (drift === 'ambiguous') f.page.document.querySelectorAll = () => [f.editor, { ...f.editor }];
    assert.equal(f.context.requestCurrent(binding, serverId), false);
  });
}

test('exclusive group request stays bound to its account when the editor disappears', async () => {
  const f = fixture();
  f.page.__elonChatGptGroupRequestOwnershipEnabled = true;
  let request;
  f.officialSubmit.a = async (_scope, options) => { request = options; return true; };
  assert.equal((await f.submit.submit(f.command).completion).status, 'accepted');
  f.editor.isConnected = false;
  assert.equal(request.isSubmissionCurrent(), false);
  assert.equal(request.isRequestCurrent(), true);
  assert.equal(f.submit.state().requestCurrent, true);
  assert.ok(f.submit.readAccepted());
  f.refreshAuth();
  assert.equal(request.isRequestCurrent(), true, 'same-account credential refresh is not a different owner');
  f.changeAccount();
  assert.equal(request.isRequestCurrent(), false);
  assert.equal(f.submit.readAccepted(), null);
});

test('exclusive request cannot survive document or route changes or be enabled on a personal host', async () => {
  const f = fixture();
  await f.page.__elonChatGptRspackRuntime.load();
  const binding = f.context.capture(f.editor);
  assert.equal(f.context.ownedRequestCurrent(binding), false);
  f.page.__elonChatGptGroupRequestOwnershipEnabled = true;
  assert.equal(f.context.ownedRequestCurrent(binding), true);
  f.page.location = new URL('https://chatgpt.com/?temporary-chat=true');
  assert.equal(f.context.ownedRequestCurrent(binding), false);
  f.page.location = new URL(binding.href);
  f.page.__elonChatGptDocumentToken = 'doc_reloaded';
  assert.equal(f.context.ownedRequestCurrent(binding), false);
});
