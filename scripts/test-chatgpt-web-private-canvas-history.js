'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { management, CID, ID, LIST, RESTORE, HISTORY, doc } = require('./fixtures/chatgpt-web-private-canvas-management.cjs');

test('history pages are scoped, ordered, full source and stop at version one without runtime imports', async () => {
  const f = management();
  f.versions[0].content = '<script>inert</script>\n' + 'x'.repeat(40000);
  f.page.__elonChatGptPrivateRuntimeBindings = null;
  const value = await f.list(), first = await f.history(value);
  assert.equal(first.ok, true); assert.equal(first.history.beforeVersion, 4);
  assert.deepEqual(first.history.versions.map(x => x.documentVersion), [3, 2]);
  assert.equal(first.history.versions[0].content, f.versions[0].content);
  assert.equal(first.history.nextBeforeVersion, 2);
  const second = await f.history(first, 2);
  assert.deepEqual(second.history.versions.map(x => x.documentVersion), [1]);
  assert.equal(second.history.nextBeforeVersion, null);
  const count = f.requests.length;
  assert.deepEqual((await f.history(second, 1)).history.versions, []);
  assert.equal(f.requests.length, count); assert.equal(f.imports(), 0); assert.equal(f.writes().length, 0);
});

for (const kind of ['missing', 'wrong_id', 'ascending', 'duplicate', 'not_before', 'string_version']) {
  test('malformed history is not rendered as no versions: ' + kind, async () => {
    const f = management(), value = await f.list();
    let rows = [doc({ version: 3 })];
    if (kind === 'wrong_id') rows[0].id = 'other';
    if (kind === 'ascending') rows = [doc({ version: 2 }), doc({ version: 3 })];
    if (kind === 'duplicate') rows.push(rows[0]);
    if (kind === 'not_before') rows[0].version = 4;
    if (kind === 'string_version') rows[0].version = '3';
    f.setHook(() => ({ payload: kind === 'missing' ? {} : { previous_doc_states: rows } }));
    const result = await f.history(value);
    assert.equal(result.ok, false); assert.equal(result.history, undefined); assert.equal(f.writes().length, 0);
  });
}

test('restore uses the selected historical version and confirms original title, type, body and comments', async () => {
  const f = management();
  f.versions[0] = doc({ version: 3, title: 'Old title', textdoc_type: 'code/python', content: 'a\u{1f600}bc',
    comments: [{ id: 'comment_1', start: 1, end: 2, content: 'Original comment' }] });
  const history = await f.history(await f.list());
  assert.equal(history.history.versions[0].comments[0].end, 3);
  const saved = await f.restore(history);
  assert.equal(saved.ok, true); assert.equal(saved.code, 'canvas_saved');
  assert.equal(saved.documents[0].content, f.versions[0].content);
  assert.equal(saved.documents[0].title, 'Old title'); assert.equal(saved.documents[0].documentType, 'code/python');
  assert.equal(saved.documents[0].documentVersion, 5); assert.equal(saved.documents[0].comments[0].end, 3);
  assert.equal(f.writes().length, 1); assert.equal(f.writes()[0].url, RESTORE);
  assert.deepEqual(JSON.parse(f.writes()[0].init.body), { version: 4, restore_from_version: 3 });
  assert.deepEqual(f.requests.slice(-4).map(x => x.url), [LIST, HISTORY + 4, RESTORE, LIST]);
  assert.deepEqual(f.invalidations, [{ queryKey: [CID, 'textdocs'], exact: true, refetchType: 'active' }]);
  assert.equal((await f.restore(history)).ok, false); assert.equal(f.writes().length, 1);
});

for (const kind of ['confirmation', 'ticket', 'history_ticket', 'target', 'scope', 'expired', 'page_changed', 'source_changed', 'history_changed', 'website_edit', 'streaming']) {
  test('restore cannot submit a stale or unconfirmed selection: ' + kind, async () => {
    const f = management(), history = await f.history(await f.list());
    if (kind === 'ticket') history.ticket = 'expired';
    if (kind === 'history_ticket') history.history.ticket = 'expired';
    if (kind === 'scope') history.scope = 'expired';
    if (kind === 'expired') f.advance(1800001);
    if (kind === 'page_changed') await f.history(history, 2);
    if (kind === 'source_changed') f.rows[0].content = 'Other writer';
    if (kind === 'history_changed') f.versions[0].content = 'Different historical source';
    if (kind === 'website_edit') f.edits.userEdits[ID] = [{ isPending: true }];
    if (kind === 'streaming') f.snapshot.streaming = true;
    const result = await f.restore(history, kind === 'target' ? 1 : 3, kind !== 'confirmation');
    assert.equal(result.ok, false); assert.equal(result.attempted, false); assert.equal(f.writes().length, 0);
  });
}

for (const kind of ['success_timeout', 'unapplied_timeout', 'wrong_readback']) {
  test('unknown restore is read-verified and never replayed: ' + kind, async () => {
    const f = management(), history = await f.history(await f.list());
    f.setHook(req => {
      if (req.url === RESTORE) {
        if (kind !== 'unapplied_timeout') f.respond(req);
        if (kind === 'wrong_readback') { f.rows[0].title = 'Different'; return { payload: { version: 5 } }; }
        throw Error('timeout');
      }
      return f.respond(req);
    });
    assert.equal((await f.restore(history)).code, 'canvas_write_unconfirmed');
    const refreshed = await f.list(true);
    assert.equal(refreshed.unconfirmedWrite, true);
    assert.equal((await f.restore(history)).code, 'canvas_write_unconfirmed');
    const verified = await f.run(f.selection(refreshed, 'verify'));
    assert.equal(verified.unconfirmedWrite, kind !== 'success_timeout');
    if (verified.unconfirmedWrite) {
      const acknowledged = await f.run(f.selection(verified, 'verify'), true);
      assert.equal(acknowledged.unconfirmedWrite, false);
    }
    assert.equal(f.writes().length, 1);
  });
}
