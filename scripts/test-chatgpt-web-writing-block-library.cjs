'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const { harness, parser } = require('./fixtures/chatgpt-writing-library-harness.cjs');
const turn = () => new Promise(resolve => setImmediate(resolve));

test('linked wrapper and widget preserve exact library identity without exposing it in display data', () => {
  for (const widget of [false, true]) {
    const h = harness(), m = h.state.message;
    if (widget) { m.content.parts = ['']; m.metadata.content_references = [{ type: 'client_defined_widget', category: 'writing_block',
      data: { id: h.bid, variant: 'document', content: 'Original\n', library_file_id: h.lid } }]; }
    const before = JSON.stringify(m), projected = parser.project(m, true, true);
    assert.equal(projected.writeSources[0].libraryFileId, h.lid);
    assert.equal(projected.parts[0].textBlock.sourceMessageId, h.mid);
    assert.equal(JSON.stringify(parser.project(m)).includes(h.lid), false);
    assert.equal(JSON.stringify(m), before);
    if (widget) m.metadata.content_references[0].data.library_file_id = 'libfile_other';
    else m.content.parts[0] = m.content.parts[0].replace('variant="document"', 'variant="document" library_file_id="libfile_other"');
    assert.deepEqual(parser.project(m, true, true).writeSources, []);
  }
});

test('clean hydrated library save uses one message POST, shared queue, readback and one-node reconciliation', async () => {
  const h = harness(), p = await h.prepare();
  assert.equal(p.code, 'writing_ready');
  assert.equal((await h.save(p.ticket)).code, 'writing_saved');
  assert.equal(h.state.posts.length, 1);
  assert.equal(h.state.posts[0].writing_block.content, 'Native edit');
  assert.equal(h.state.posts[0].writing_block.library_file_id, undefined);
  assert.equal(h.state.posts[0].expected_current_version, undefined);
  assert.equal(h.state.message.metadata.writing_blocks[h.bid].library_file_id, h.lid);
  assert.deepEqual(h.state.message.metadata.writing_blocks.unrelated, { content: 'Keep other block' });
  assert.equal(h.state.updates, 1);
  assert.equal(h.state.session.draftContent, 'Native edit');
  assert.equal(h.state.session.lastSavedContent, 'Native edit');
  assert.equal(h.state.session.dirty, false);
  assert.equal(h.state.session.inFlightSaveSequence, null);
  assert.equal(h.state.retains, 0);
  assert.equal(h.state.invalidated.length, 4);
  assert.equal((await h.save(p.ticket, 'Second edit')).code, 'writing_saved');
  assert.equal(h.state.posts.length, 2);
});

for (const [label, patch] of Object.entries({ seed: { hydratedFromLibrary: false }, dirty: { dirty: true },
  editor: { hasPendingEditorChanges: true }, saving: { inFlightSaveSequence: 3 }, otherDraft: { draftContent: 'Other draft' },
  stale: { lastSavedContent: 'Newer library content' }, owner: { libraryFileId: 'libfile_other' } })) {
  test('linked preparation rejects ' + label + ' without changing drafts or sending', async () => {
    const h = harness(); h.change(patch); const before = JSON.stringify(h.state.session);
    assert.equal((await h.prepare()).code, 'writing_web_edit_pending');
    assert.equal(JSON.stringify(h.state.session), before);
    assert.equal(h.state.posts.length, 0);
  });
}

test('preparation binds version and save sequence, even if content is unchanged', async () => {
  for (const patch of [{ baseVersionNumber: 5 }, { latestIssuedSaveSequence: 2 }, { fileId: 'file_new' }]) {
    const h = harness(), p = await h.prepare(); h.change(patch);
    assert.equal((await h.save(p.ticket)).code, 'writing_version_conflict');
    assert.equal(h.state.posts.length, 0);
    assert.equal(h.state.session.draftContent, 'Original\n');
  }
});

test('waiting in official queue rechecks ownership and never writes after a timeout', async () => {
  for (const expire of [false, true]) {
    const h = harness(), p = await h.prepare(); h.state.holdQueue = true;
    const save = h.save(p.ticket); await turn();
    assert.equal(h.state.posts.length, 0);
    if (expire) {
      const timer = h.state.timerCallbacks.findLast(row => row.delay > 3000);
      clearTimeout(timer.timer); timer.fn();
      assert.equal((await save).code, 'writing_timeout');
    } else h.change({ draftContent: 'Another editor', dirty: true });
    await h.state.queued();
    if (!expire) assert.equal((await save).code, 'writing_version_conflict');
    assert.equal(h.state.posts.length, 0); assert.equal(h.state.retains, 0);
  }
});

test('known HTTP rejection rolls back only our draft, preserves another editor and releases our sequence', async () => {
  for (const otherEdit of [false, true]) {
    const h = harness(), p = await h.prepare(); h.state.effects = false; h.state.postError = 'http_403';
    if (otherEdit) h.state.onPost = () => h.change({ draftContent: 'Other editor', dirty: true });
    assert.equal((await h.save(p.ticket)).code, 'writing_http_403');
    assert.equal(h.state.session.draftContent, otherEdit ? 'Other editor' : 'Original\n');
    assert.equal(h.state.session.inFlightSaveSequence, null);
    assert.equal(h.state.session.dirty, otherEdit);
    assert.equal(h.state.invalidated.length, 0);
  }
});

test('uncertain POST reserves the sequence, cannot replay, and readback completes it once', async () => {
  const h = harness(), p = await h.prepare(); h.state.postError = 'timeout';
  assert.equal((await h.save(p.ticket)).code, 'writing_write_unconfirmed');
  assert.equal(h.state.session.inFlightSaveSequence, 1);
  assert.equal((await h.save(p.ticket, 'Do not send')).pending, true);
  assert.equal((await h.prepare()).pending, true);
  assert.equal((await h.verify(p.ticket)).code, 'writing_saved');
  assert.equal(h.state.posts.length, 1);
  assert.equal(h.state.session.inFlightSaveSequence, null);
  assert.equal((await h.verify(p.ticket)).code, 'writing_ready');
  assert.equal(h.state.updates, 1);
});

test('failed readback and externally changed drafts do not falsely acknowledge or overwrite', async () => {
  for (const change of ['readback', 'draft', 'identity', 'linkage']) {
    const h = harness(), p = await h.prepare();
    if (change === 'readback') h.state.effects = false;
    if (change === 'draft') h.state.onPost = () => h.change({ draftContent: 'External draft', dirty: true });
    if (change === 'identity') h.state.onPost = () => { h.state.current = false; };
    if (change === 'linkage') h.state.onPost = () => { delete h.state.server.metadata.writing_blocks[h.bid].library_file_id; };
    const result = await h.save(p.ticket);
    assert.equal(result.pending, true, change);
    assert.notEqual(result.code, 'writing_saved');
    assert.equal(h.state.updates, 0);
    assert.equal(h.state.posts.length, 1);
    if (change === 'draft') assert.equal(h.state.session.draftContent, 'External draft');
  }
});

test('missing library runtime never falls through to uncoordinated message writes', async () => {
  const h = harness(); delete h.page.__elonChatGptWritingLibrarySession;
  assert.equal((await h.prepare()).code, 'writing_runtime_unavailable');
  assert.equal(h.state.posts.length, 0);
});

test('a legacy resident writer cannot use a newly injected parser to bypass the library queue', () => {
  const h = harness();
  assert.deepEqual(parser.project(h.state.message, true).writeSources, []);
  assert.equal(parser.project(h.state.message, true, true).writeSources.length, 1);
  const policy = require('./fixtures/chatgpt-writing-library-harness.cjs').policy;
  const cid = h.path.slice(3), payload = { conversation_id: cid, is_do_not_remember: false, current_node: h.mid,
    mapping: { [h.mid]: { id: h.mid, parent: null, message: h.state.message } } };
  assert.throws(() => policy.source(payload, cid, h.mid, h.bid, parser), /writing_selection_unavailable/);
  assert.equal(policy.source(payload, cid, h.mid, h.bid, parser, null, true).libraryFileId, h.lid);
});

test('refused save reservation restores only our unsent draft', async () => {
  for (const change of [false, true]) {
    const h = harness(), p = await h.prepare();
    h.store.beginSave = () => { if (change) h.change({ draftContent: 'Other editor', dirty: true }); return null; };
    assert.equal((await h.save(p.ticket)).code, 'writing_web_edit_pending');
    assert.equal(h.state.session.draftContent, change ? 'Other editor' : 'Original\n');
    assert.equal(h.state.posts.length, 0);
    assert.equal(h.state.retains, 0);
  }
});
