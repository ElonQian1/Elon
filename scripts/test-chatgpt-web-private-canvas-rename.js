'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const { fixture, ID, LIST, SAVE } = require('./fixtures/chatgpt-web-private-canvas-documents.cjs');
const rename = (f, ticket, title = 'Renamed original', confirmed = true) =>
  f.run({ operation: 'rename', ticket, id: ID, title }, confirmed);
const writes = f => f.requests.filter(value => value.init.method === 'POST');

test('rename uses its exact endpoint once, accepts empty success bodies, and verifies unchanged content', async () => {
  const f = fixture(), before = structuredClone(f.rows[0]), list = await f.list();
  const result = await rename(f, list.ticket);
  assert.equal(result.code, 'canvas_renamed'); assert.equal(result.attempted, true);
  assert.deepEqual(f.requests.map(value => [value.url, value.init.method]),
    [[LIST, 'GET'], [LIST, 'GET'], [SAVE + '/rename', 'POST'], [LIST, 'GET']]);
  assert.deepEqual(JSON.parse(writes(f)[0].init.body), { title: 'Renamed original' });
  assert.equal(writes(f)[0].limits.mode, 'none');
  assert.equal(result.documents[0].content, before.content);
  assert.equal(result.documents[0].documentVersion, before.version);
  assert.equal((await rename(f, list.ticket)).code, 'canvas_selection_expired');
  assert.equal(writes(f).length, 1);
});
test('unchanged names consume no write', async () => {
  const f = fixture(), list = await f.list();
  assert.equal((await rename(f, list.ticket, f.rows[0].title)).code, 'canvas_unchanged');
  assert.equal(writes(f).length, 0);
});
test('Unicode names and a server version bump preserve source and comments', async () => {
  const f = fixture(), list = await f.list(), title = '\u753b\u5e03\ud83d\ude00';
  f.setHook(({ init }) => {
    if (init.method !== 'POST') return;
    f.rows[0].title = title; f.rows[0].version += 1;
    return { ok: true, status: 204 };
  });
  const result = await rename(f, list.ticket, title);
  assert.equal(result.code, 'canvas_renamed'); assert.equal(result.documents[0].title, title);
  assert.equal(result.documents[0].documentVersion, 5); assert.equal(writes(f).length, 1);
});
for (const title of ['', ' ', ' padded', 'trailing ', 'x\nline', 'x'.repeat(513), '\ud800', 1, null]) {
  test('invalid title never reaches a write: ' + JSON.stringify(title).slice(0, 40), async () => {
    const f = fixture(), list = await f.list();
    assert.equal((await rename(f, list.ticket, title)).ok, false); assert.equal(writes(f).length, 0);
  });
}
for (const state of ['confirmation', 'disabled', 'title_changed', 'body_changed', 'pending_edit', 'stale_ticket', 'stale_scope']) {
  test('rename preserves conflicting or unconfirmed state: ' + state, async () => {
    const f = fixture(), list = await f.list();
    if (state === 'disabled') f.page.__elonChatGptPrivateConversationMutationsEnabled = false;
    if (state === 'title_changed') f.rows[0].title = 'Other title';
    if (state === 'body_changed') f.rows[0].content = 'Other content';
    if (state === 'pending_edit') f.edits.userEdits[ID] = [{ isPending: true }];
    if (state === 'stale_ticket') list.ticket = 'stale';
    if (state === 'stale_scope') f.page.__elonChatGptDocumentToken = 'doc_changed';
    assert.equal((await rename(f, list.ticket, 'New', state !== 'confirmation')).ok, false);
    assert.equal(writes(f).length, 0);
  });
}
for (const drift of ['title', 'body', 'comments', 'type', 'version', 'missing']) {
  test('2xx without matching readback cannot report renamed: ' + drift, async () => {
    const f = fixture(), list = await f.list();
    f.setHook(({ init }) => {
      if (init.method !== 'POST') return;
      f.rows[0].title = drift === 'title' ? 'Wrong title' : 'Renamed original';
      if (drift === 'body') f.rows[0].content = 'Unrelated body';
      if (drift === 'comments') f.rows[0].comments = [{ id: 'new_comment', start: 0, end: 1, content: 'Other' }];
      if (drift === 'type') f.rows[0].textdoc_type = 'code/python';
      if (drift === 'version') f.rows[0].version = 3;
      if (drift === 'missing') f.rows.length = 0;
      return { status: 204, ok: true };
    });
    assert.equal((await rename(f, list.ticket)).code, 'canvas_write_unconfirmed');
    const refreshed = await f.list(true);
    assert.equal(refreshed.unconfirmedWrite, true);
    assert.equal((await rename(f, refreshed.ticket)).code, 'canvas_write_unconfirmed');
    assert.equal(writes(f).length, 1); assert.deepEqual(f.invalidations, []);
  });
}
test('successful rename with lost response is recovered by GET, without another POST', async () => {
  const f = fixture(), list = await f.list();
  f.setHook(({ init }) => { if (init.method === 'POST') { f.rows[0].title = 'Renamed original'; throw Error('timeout'); } });
  assert.equal((await rename(f, list.ticket)).code, 'canvas_write_unconfirmed');
  f.setHook(async () => {});
  const refreshed = await f.list(true);
  const result = await f.run({ operation: 'verify', ticket: refreshed.ticket, id: ID }, false);
  assert.equal(result.code, 'canvas_renamed'); assert.equal(result.unconfirmedWrite, false);
  assert.equal(writes(f).length, 1);
});
test('an unchanged GET cannot silently acknowledge an uncertain rename', async () => {
  const f = fixture(), list = await f.list();
  f.setHook(({ init }) => { if (init.method === 'POST') throw Error('timeout'); });
  await rename(f, list.ticket); f.setHook(async () => {});
  const read = await f.list(true);
  const pending = await f.run({ operation: 'verify', ticket: read.ticket, id: ID }, false);
  assert.equal(pending.code, 'canvas_verification_pending'); assert.equal(pending.unconfirmedWrite, true);
  const acknowledged = await f.run({ operation: 'verify', ticket: pending.ticket, id: ID }, true);
  assert.equal(acknowledged.code, 'canvas_result_acknowledged'); assert.equal(acknowledged.unconfirmedWrite, false);
  assert.equal(writes(f).length, 1);
});
test('rename is single flight with all Canvas writes', async () => {
  const f = fixture(), list = await f.list(); let release;
  f.setHook(({ init }) => init.method === 'POST' ? new Promise(resolve => { release = resolve; }) : undefined);
  const pending = rename(f, list.ticket);
  for (let i = 0; !release && i < 50; i++) await new Promise(resolve => setTimeout(resolve, 1));
  assert.equal((await f.save(list.ticket)).code, 'canvas_busy');
  release(); assert.equal((await pending).code, 'canvas_renamed'); assert.equal(writes(f).length, 1);
});

test('public editor proves original rename ownership and body, not a shared-link endpoint', {
  skip: !process.env.CHATGPT_PUBLIC_RUNTIME_DIR && 'Reviewed public assets are optional offline inputs.'
}, () => {
  const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
  const { parseSource } = require('./analyze-chatgpt-runtime-contracts.cjs');
  const source = fs.readFileSync(path.join(process.env.CHATGPT_PUBLIC_RUNTIME_DIR, 'ac476d6c-foq8estmw3bbr7op.js'));
  assert.equal(crypto.createHash('sha256').update(source).digest('hex'), 'f341cbd1ebc80c14838f6bada429d1826a5a78577f094f5be452ca7663c3c895');
  const parsed = parseSource(source.toString('utf8')), owner = parsed.definitions.get('zi');
  assert.equal(owner.length, 1);
  const code = parsed.text.slice(owner[0].start, owner[0].end);
  assert.ok(code.includes('safePost(`/textdoc/{textdoc_id}/rename`'));
  assert.ok(code.includes('path:{textdoc_id:e}},requestBody:{title:t}'));
  assert.ok(parsed.text.includes('mutationKey:[e,`textdocs`],mutationFn:zi'));
});

test('rename is wired through the production editor and preserves native drafts', () => {
  const fs = require('node:fs'), path = require('node:path');
  const read = name => fs.readFileSync(path.join(__dirname, '../android/app/src/main/kotlin/com/elon/app', name), 'utf8');
  const view = read('WebChatCanvasEditorView.kt'), manager = read('WebChatCanvasManagementCoordinator.kt');
  const owner = read('WebChatCanvasDocumentsCoordinator.kt');
  for (const token of ['ic_project_action_rename', 'web-chat-canvas-editor-rename', '!busy && writable']) assert.ok(view.includes(token));
  for (const token of ['web-chat-canvas-rename-title', 'web-chat-canvas-rename-confirm', 'web-chat-canvas-rename-cancel',
    'draft.renameRequest(value, title)', 'if (!active(run)) return@setOnClickListener', 'isEnabled = false']) assert.ok(manager.includes(token));
  assert.ok(owner.includes('rename = { management?.showRename() }'));
  assert.ok(owner.includes('editing.acceptRename(renamed)'));
  assert.ok(owner.includes('!value.unconfirmedWrite && editing.acceptRename(server)'));
  assert.ok(!owner.includes('editing.adopt(renamed)'));
  assert.ok(!manager.includes('evaluateJavascript') && !manager.includes('loadUrl('));
});
