'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { management, ID, SHARE } = require('./fixtures/chatgpt-web-private-canvas-management.cjs');

test('first share requires confirmation and posts once without guessed audience/body', async () => {
  const f = management(), value = await f.list();
  assert.equal((await f.share(value)).share.state, 'missing');
  assert.equal((await f.share(value, 'share_create')).code, 'canvas_confirmation_required');
  assert.equal(f.writes().length, 0);
  const result = await f.share(value, 'share_create', true);
  assert.equal(result.code, 'canvas_share_created'); assert.equal(result.attempted, true);
  assert.deepEqual(result.share, { documentId: ID, state: 'public', id: 'synthetic_shared_canvas', documentVersion: 4 });
  assert.equal(f.writes().length, 1); assert.equal(f.writes()[0].url, SHARE); assert.equal(f.writes()[0].init.body, undefined);
  assert.deepEqual(f.invalidations, [{ queryKey: ['canvas', 'textdoc', 'share', ID], exact: true, refetchType: 'active' }]);
  assert.equal((await f.share(result, 'share_create', true)).code, 'canvas_share_ready');
  assert.equal(f.writes().length, 1);
});

test('existing share is returned without publishing newer source or requiring composer/runtime', async () => {
  const f = management(), value = await f.list();
  f.setShared({ ...f.published(), version: 2, content: 'Older published snapshot' });
  f.page.__elonChatGptPrivateRuntimeBindings = null;
  const result = await f.share(value, 'share_create', true);
  assert.equal(result.share.documentVersion, 2); assert.equal(f.writes().length, 0); assert.equal(f.imports(), 0);
});

for (const patch of [{ access: 'workspace' }, { access: 'private' }, { is_moderation_blocked: true }, { is_anonify_api_key_detected: true }]) {
  test('restricted share cannot expose a public link or silently change access: ' + JSON.stringify(patch), async () => {
    const f = management(), value = await f.list(); f.setShared({ ...f.published(), ...patch });
    const result = await f.share(value, 'share_create', true);
    assert.deepEqual(result.share, { documentId: ID, state: 'restricted', id: '', documentVersion: null });
    assert.equal(f.writes().length, 0);
  });
}

for (const payload of [{}, { shared_textdoc: {} }, { shared_textdoc: false }]) {
  test('missing/malformed metadata is unknown, not an absent share', async () => {
    const f = management(), value = await f.list(); f.setHook(() => ({ payload }));
    const result = await f.share(value, 'share_create', true);
    assert.equal(result.code, 'canvas_share_unconfirmed'); assert.equal(result.share, undefined); assert.equal(f.writes().length, 0);
  });
}

for (const kind of ['version', 'id', 'content', 'private', 'timeout']) {
  test('share POST response and fresh GET must agree before success: ' + kind, async () => {
    const f = management(), value = await f.list(); let wrote = false;
    f.setHook(req => {
      const response = f.respond(req);
      if (req.url === SHARE && req.init.method === 'POST') { wrote = true; return response; }
      if (req.url === SHARE && wrote) {
        if (kind === 'timeout') throw Error('timeout');
        const patch = kind === 'version' ? { version: 2 } : kind === 'id' ? { shared_textdoc_id: 'other' } :
          kind === 'private' ? { access: 'private' } : { content: 'Different' };
        return { payload: { shared_textdoc: { ...f.published(), ...patch } } };
      }
      return response;
    });
    assert.equal((await f.share(value, 'share_create', true)).code, 'canvas_share_write_unconfirmed');
    assert.equal(f.writes().length, 1); assert.deepEqual(f.invalidations, []);
    const refreshed = await f.list(true); assert.equal(refreshed.unconfirmedWrite, true);
    assert.equal((await f.run(f.selection(refreshed, 'verify'))).code, 'canvas_share_verification_required');
  });
}

test('successful publication with lost response is reconciled by lookup without re-POST', async () => {
  const f = management(), value = await f.list();
  f.setHook(req => { const response = f.respond(req); if (req.init.method === 'POST') throw Error('timeout'); return response; });
  assert.equal((await f.share(value, 'share_create', true)).code, 'canvas_share_write_unconfirmed');
  const refreshed = await f.list(true);
  const checked = await f.share(refreshed);
  assert.equal(checked.unconfirmedWrite, false); assert.equal(checked.share.state, 'public');
  assert.equal(f.writes().length, 1); assert.equal(f.invalidations.length, 1);
});

test('unapplied publication remains guarded until explicit read-only acknowledgement', async () => {
  const f = management(), value = await f.list();
  f.setHook(req => { if (req.init.method === 'POST') throw Error('timeout'); return f.respond(req); });
  await f.share(value, 'share_create', true);
  const refreshed = await f.list(true);
  assert.equal((await f.share(refreshed, 'share_create', true)).code, 'canvas_write_unconfirmed');
  const checked = await f.share(refreshed);
  assert.equal(checked.share.state, 'missing'); assert.equal(checked.unconfirmedWrite, true);
  assert.equal((await f.share(checked, 'share_ack', false)).unconfirmedWrite, true);
  assert.equal((await f.share(checked, 'share_ack', true)).unconfirmedWrite, false);
  assert.equal(f.writes().length, 1);
});

for (const kind of ['source_changed', 'pending_web_edit', 'disabled', 'account_changed']) {
  test('share creation respects original identity, version and pending edit guards: ' + kind, async () => {
    const f = management(), value = await f.list();
    if (kind === 'source_changed') f.rows[0].content = 'Other writer';
    if (kind === 'pending_web_edit') f.edits.userEdits[ID] = [{ isPending: true }];
    if (kind === 'disabled') f.page.__elonChatGptPrivateConversationMutationsEnabled = false;
    if (kind === 'account_changed') f.headers.Authorization = 'Bearer synthetic-other';
    const result = await f.share(value, 'share_create', true);
    assert.equal(result.ok, false); assert.equal(result.attempted, false); assert.equal(f.writes().length, 0);
  });
}

test('share acknowledgement cannot discard an unknown original-content write', async () => {
  const f = management(), value = await f.list();
  f.setHook(req => { if (req.init.method === 'POST') throw Error('timeout'); return f.respond(req); });
  await f.save(value.ticket);
  const refreshed = await f.list(true);
  assert.equal((await f.share(refreshed, 'share_ack', true)).code, 'canvas_write_unconfirmed');
  assert.equal((await f.list(true)).unconfirmedWrite, true);
  assert.equal(f.writes().length, 1);
});
