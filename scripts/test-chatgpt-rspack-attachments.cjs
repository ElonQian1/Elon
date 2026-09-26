'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture } = require('./fixtures/chatgpt-rspack.cjs');
const api = require('../android/app/src/main/assets/chatgpt_web_rspack_attachments.js');

function setup(options = {}) {
  const f = fixture({ latest: options.latest || true });
  f.page.location = new URL('https://chatgpt.com/?temporary-chat=true');
  f.uploads = []; f.removed = [];
  f.files = [{ name: 'fixture.png', type: 'image/png', size: 64 }];
  f.descriptor = { href: f.page.location.href, documentToken: f.page.__elonChatGptDocumentToken };
  f.composer.N = async (scope, id, files, options) => {
    f.uploads.push({ scope, id, files, options });
    f.values.set(f.composer.f, files.map((file, i) => ({ uploadId: 'fixture-' + i, status: 'uploading', name: file.name })));
    options.onQueued(files.map((_, i) => 'fixture-' + i));
    f.values.set(f.composer.f, files.map((file, i) => ({ uploadId: 'fixture-' + i, status: 'ready',
      id: 'file-' + i, name: file.name, mimeType: file.type, size: file.size, width: 8, height: 8, source: 'local' })));
  };
  f.composer.E = (scope, id, uploadId) => {
    f.removed.push(uploadId);
    scope.set(f.composer.f, id, entries => entries.filter(e => e.uploadId !== uploadId));
  };
  f.attachments = api.create(f.page, options);
  f.page.__elonChatGptRspackAttachments = f.attachments;
  f.upload = () => f.attachments.upload(f.files, f.descriptor, new AbortController().signal);
  return f;
}

test('new reviewed profile uses exact new cache IDs without executing a module', async () => {
  const f = setup();
  const loaded = await f.page.__elonChatGptRspackRuntime.load();
  assert.equal(f.page.__elonChatGptRspackRuntime.profile, 'web_20260926_rspack');
  assert.equal(loaded.identity, f.identityAtoms);
  assert.equal(loaded.conversation, f.conversation);
  assert.equal(f.imports[0], 'https://chatgpt.com/cdn/assets/633146.6ed5d111e4.js');
});

test('official upload -> ready file specs -> one send, then consume only after ACK', async () => {
  const f = setup();
  await f.upload();
  assert.equal(f.uploads[0].options.isTemporaryChat, true);
  assert.equal(f.uploads[0].files, f.files);
  assert.equal(f.attachments.merge([])[0].state, 'ready');
  f.command.requireNativeAttachment = true;
  const result = await f.submit.submit(f.command).completion;
  assert.equal(result.status, 'accepted');
  assert.equal(f.calls.length, 1);
  const args = f.calls[0][1];
  assert.equal(args.preserveDraft, true);
  assert.equal(args.additionalAttachments[0].id, 'file-0');
  assert.equal(args.additionalAttachments[0].width, 8);
  assert.equal(f.values.get(f.composer.f).length, 0);
  assert.equal(args.isRequestCurrent(), true, 'consuming after ACK must not cancel the stream');
});

test('Win staged bytes pass through the shared sender into the current official upload', async () => {
  const f = setup({ latest: 'current' }), receipts = [];
  Object.assign(f.page, { File, Blob, atob, btoa,
    __elonChatGptAdapterVersion: 215,
    createImageBitmap: async () => ({ width: 8, height: 8, close() {} }),
    elonChatGptNative: { postMessage: raw => receipts.push(JSON.parse(raw)) },
  });
  const bridge = require('../desktop-shell/src-tauri/src/local_ai_browser/win_attachment_source.js')(f.page);
  f.page.__elonChatGptNativeAttachmentSource = require('../android/app/src/main/assets/chatgpt_web_native_attachment_source.js');
  f.page.__elonChatGptPrivateAttachmentSend = require('../android/app/src/main/assets/chatgpt_web_private_attachment_send.js').create(f.page);
  const batchId = '00000000-0000-4000-8000-000000000001';
  const leaseId = '00000000-0000-4000-8000-000000000002';
  const command = value => bridge.command(JSON.stringify({ requestId: 'mcp_image1', value: JSON.stringify({ batchId, ...value }) }));
  await command({ step: 'begin', files: [{ leaseId, name: 'fixture.png', type: 'image/png', size: 3 }] });
  await command({ step: 'chunk', leaseId, offset: 0, data: 'YWJj' });
  await command({ step: 'upload' });
  assert.equal(receipts.at(-1).detail, 'private_attachment_associated');
  assert.equal(f.uploads.length, 1);
  assert.equal(await f.uploads[0].files[0].text(), 'abc');
  assert.equal(f.uploads[0].options.isTemporaryChat, true);
  assert.equal(f.calls.length, 0, 'upload does not send the question');
  assert.equal(bridge.guardSend('{"action":"send_prompt"}'), false);
});

test('current live manifest resolves its reviewed conversation alias and image sender', async () => {
  const f = setup({ latest: 'current' });
  await f.upload();
  assert.equal(f.page.__elonChatGptRspackRuntime.profile, 'web_20260926b_rspack');
  assert.equal(f.page.__elonChatGptRspackRuntime.peek().conversation, f.cache.JqV.exports);
  f.command.requireNativeAttachment = true;
  assert.equal((await f.submit.submit(f.command).completion).status, 'accepted');
  assert.equal(f.calls[0][1].additionalAttachments.length, 1);
});

test('uploads from the committed composer when the legacy element ID is absent', async () => {
  const f = setup({ latest: 'current' });
  f.page.document.querySelector = () => null;
  f.page.document.querySelectorAll = selector => selector === '[data-testid="prompt-textarea"]' ? [f.editor] : [];
  await f.upload();
  assert.equal(f.attachments.prepare(f.editor).attachments[0].id, 'file-0');
});

test('composer lookup ignores hidden and unowned fields but rejects two live owners', async () => {
  const f = setup({ latest: 'current' });
  await f.page.__elonChatGptRspackRuntime.load();
  const hidden = { ...f.editor, getBoundingClientRect: () => ({ width: 0, height: 0 }) };
  const unrelated = { isConnected: true, getBoundingClientRect: f.editor.getBoundingClientRect };
  const candidates = [hidden, unrelated, f.editor];
  f.page.document.querySelectorAll = () => candidates;
  assert.equal(f.context.find().node, f.editor);
  candidates.push({ ...f.editor });
  assert.equal(f.context.find(), null);
  assert.equal(f.context.state().code, 'composer_ambiguous');
  await assert.rejects(f.upload());
  assert.equal(f.uploads.length, 0);
});

test('readiness accepts only this bridge confirmed uploads without dispatching', async () => {
  const f = setup({ latest: 'current' });
  await f.upload();
  assert.equal((await f.submit.inspect(f.editor)).code, 'ready');
  assert.equal(f.calls.length, 0);
  f.values.set(f.composer.f, [{ id: 'user-owned', status: 'ready' }]);
  assert.notEqual((await f.submit.inspect(f.editor)).code, 'ready');
  assert.equal(f.calls.length, 0);
});

test('a remounted composer can claim the exact upload in the same account and conversation', async () => {
  const f = setup({ latest: 'current' });
  await f.upload();
  const next = { ...f.editor };
  f.editor.isConnected = false;
  f.command.composer = next;
  f.command.requireNativeAttachment = true;
  assert.equal((await f.submit.submit(f.command).completion).status, 'accepted');
  assert.equal(f.calls[0][1].additionalAttachments[0].id, 'file-0');
});

test('a composer remount during upload preserves the exact account and ready entries', async () => {
  const f = setup({ latest: 'current' }), upload = f.composer.N;
  f.composer.N = async (...args) => {
    const next = { ...f.editor };
    f.editor.isConnected = false;
    f.page.document.querySelectorAll = selector => selector === '#prompt-textarea' ? [next] : [];
    f.command.composer = next;
    await upload(...args);
  };
  await f.upload();
  assert.equal(f.attachments.state().code, 'associated');
  f.command.requireNativeAttachment = true;
  assert.equal((await f.submit.submit(f.command).completion).status, 'accepted');
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0][1].additionalAttachments[0].id, 'file-0');
});

test('remount plus account drift cannot transfer an in-flight upload', async () => {
  const f = setup({ latest: 'current' }), upload = f.composer.N;
  f.composer.N = async (...args) => {
    const next = { ...f.editor };
    f.editor.isConnected = false;
    f.page.document.querySelectorAll = () => [next];
    f.changeAccount();
    await upload(...args);
  };
  await assert.rejects(f.upload());
  assert.equal(f.calls.length, 0);
  assert.equal(f.attachments.state().code, 'upload_association_unconfirmed');
});

test('official upload exception produces only a fixed transaction diagnostic', async () => {
  const f = setup();
  f.composer.N = async () => { throw Error('private upstream detail'); };
  await assert.rejects(f.upload());
  assert.equal(f.attachments.state().code, 'upload_transaction_failed');
  assert.ok(!JSON.stringify(f.attachments.state()).includes('private upstream detail'));
});

test('an editor replacement cannot transfer an upload to a different account', async () => {
  const f = setup({ latest: 'current' });
  await f.upload();
  f.command.composer = { ...f.editor };
  f.editor.isConnected = false;
  f.changeAccount();
  f.command.requireNativeAttachment = true;
  assert.equal((await f.submit.submit(f.command).completion).status, 'rejected');
  assert.equal(f.calls.length, 0);
});

test('a hydrated model may send only when the official routing check accepts the exact files', async () => {
  const f = setup({ latest: 'current' });
  await f.upload();
  const ready = f.values.get(f.composer.f);
  f.values.set(f.composer.s, { slug: 'hydrated-model', thinkingEffort: 'standard' });
  let checks = 0;
  f.composer.x = (scope, id, model, origin, uploads) => {
    checks++;
    assert.equal(scope, f.scope);
    assert.equal(id, f.parent.memoizedProps.conversationId);
    assert.equal(model.slug, 'hydrated-model');
    assert.equal(origin, null);
    assert.deepEqual(uploads, ready);
    assert.equal(uploads[0], ready[0]);
    return true;
  };
  assert.equal((await f.submit.inspect(f.editor)).code, 'ready');
  f.command.requireNativeAttachment = true;
  assert.equal((await f.submit.submit(f.command).completion).status, 'accepted');
  assert.ok(checks >= 2, 'recheck at dispatch, not just readiness');
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0][1].selectedModel.slug, 'hydrated-model');
  assert.equal(f.calls[0][1].additionalAttachments[0].id, 'file-0');
});

for (const outcome of ['missing', 'false', 'throws', 'unreviewed']) {
  test('changed model fails closed when compatibility is ' + outcome, async () => {
    const f = setup({ latest: outcome === 'unreviewed' ? true : 'current' });
    await f.upload();
    f.values.set(f.composer.s, { slug: 'other-model' });
    if (outcome !== 'missing') f.composer.x = () => {
      if (outcome === 'throws') throw Error('fixture unavailable');
      return outcome === 'unreviewed';
    };
    const result = await f.submit.submit(f.command).completion;
    assert.equal(result.status, 'rejected');
    assert.equal(result.code, 'attachment_model_changed');
    assert.equal(f.calls.length, 0);
    assert.equal(f.values.get(f.composer.f).length, 1);
  });
}

for (const kind of ['partial', 'error', 'wrong_ids', 'switched_account', 'changed_route', 'changed_model']) {
  test('does not confirm or send a ' + kind + ' upload', async () => {
    const f = setup();
    f.files.push({ ...f.files[0], name: 'second.png' });
    const base = f.composer.N;
    f.composer.N = async (...args) => {
      await base(...args);
      const ready = f.values.get(f.composer.f);
      if (kind === 'partial') f.values.set(f.composer.f, ready.slice(0, 1));
      if (kind === 'error') ready[0].status = 'error';
      if (kind === 'wrong_ids') ready[0].uploadId = 'unrelated';
      if (kind === 'switched_account') f.changeAccount();
      if (kind === 'changed_route') f.page.location = new URL('https://chatgpt.com/');
      if (kind === 'changed_model') f.values.set(f.composer.s, { slug: 'changed' });
    };
    if (kind === 'changed_model') {
      await f.upload();
      assert.equal(f.attachments.prepare(f.editor), null);
    } else await assert.rejects(f.upload());
    assert.equal(f.calls.length, 0);
  });
}

test('rejects user-owned existing uploads before starting a transaction', async () => {
  const f = setup();
  f.values.set(f.composer.f, [{ id: 'user-owned', status: 'ready' }]);
  await assert.rejects(f.upload());
  assert.equal(f.uploads.length, 0);
  assert.equal(f.values.get(f.composer.f)[0].id, 'user-owned');
});

test('timeout cancels exact queued IDs, retains single-flight and cleans late completion', async () => {
  const f = setup({ timeoutMs: 5 });
  let settle;
  f.composer.N = async (_scope, _id, _files, options) => {
    await new Promise(resolve => { settle = resolve; });
    f.values.set(f.composer.f, [{ uploadId: 'late', status: 'uploading' }, { uploadId: 'unrelated' }]);
    options.onQueued(['late']);
  };
  await assert.rejects(f.upload());
  await assert.rejects(f.upload(), /attachment_busy/);
  settle(); await new Promise(resolve => setImmediate(resolve));
  assert.deepEqual(f.values.get(f.composer.f), [{ uploadId: 'unrelated' }]);
  assert.equal(f.attachments.state().pending, false);
});

test('uncertain send retains attachments and never issues a second request', async () => {
  const f = setup(); await f.upload();
  f.officialSubmit.a = async () => { f.calls.push('attempt'); return false; };
  f.command.requireNativeAttachment = true;
  assert.equal((await f.submit.submit(f.command).completion).status, 'unknown');
  assert.equal((await f.submit.submit(f.command).completion).code, 'busy');
  assert.equal(f.values.get(f.composer.f).length, 1);
  assert.equal(f.calls.length, 1);
});

test('replaced ready metadata or entries cannot be submitted', async () => {
  const f = setup(); await f.upload();
  const lease = f.attachments.prepare(f.editor);
  f.values.get(f.composer.f)[0].id = 'different-file';
  assert.equal(lease.current(), false);
  f.values.set(f.composer.f, [{ ...f.values.get(f.composer.f)[0] }]);
  assert.equal(f.attachments.prepare(f.editor), null);
});

test('removal updates both official store and native chip without deleting other files', async () => {
  const f = setup(); await f.upload();
  assert.equal(f.attachments.remove('private_attachment_fixture-0'), true);
  assert.deepEqual(f.attachments.merge([]), []);
  assert.equal(f.attachments.prepare(f.editor), null);
});
