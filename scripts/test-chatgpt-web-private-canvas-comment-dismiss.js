'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const { fixture, ID, LIST, SAVE } = require('./fixtures/chatgpt-web-private-canvas-documents.cjs');
const policy = require('../android/app/src/main/assets/chatgpt_web_private_canvas_document_policy.js');
function setup() {
  const f = fixture();
  f.rows[0].content = 'A\ud83d\ude00B text';
  f.rows[0].comments = [{ id: 'first', start: 1, end: 2, content: 'First suggestion' },
    { id: 'keep', start: 3, end: 7, content: 'Kept suggestion' }];
  return f;
}
const dismiss = (f, ticket, commentId = 'first', confirmed = true) =>
  f.run({ operation: 'dismiss_comment', ticket, id: ID, commentId }, confirmed);
const writes = f => f.requests.filter(value => value.init.method !== 'GET');

test('dismiss uses the exact original/version/comment DELETE and preserves the body and other comments', async () => {
  const f = setup(), initial = structuredClone(f.rows[0]), list = await f.list();
  const result = await dismiss(f, list.ticket);
  assert.equal(result.code, 'canvas_comment_dismissed'); assert.equal(result.attempted, true);
  assert.deepEqual(f.requests.map(value => [value.url, value.init.method]),
    [[LIST, 'GET'], [LIST, 'GET'], [SAVE + '/4/comment/first?reason=dismiss', 'DELETE'], [LIST, 'GET']]);
  assert.equal(writes(f)[0].init.body, undefined);
  assert.equal(writes(f)[0].limits.mode, 'json');
  assert.equal(result.documents[0].content, initial.content);
  assert.equal(result.documents[0].documentVersion, 5);
  assert.deepEqual(result.documents[0].comments, [{ ...initial.comments[1], start: 4, end: 8 }]);
  assert.equal((await dismiss(f, list.ticket)).code, 'canvas_selection_expired');
  assert.equal(writes(f).length, 1);
});

test('ordinary body save still cannot silently remove or edit comments', async () => {
  const f = setup(), list = await f.list(), before = list.documents[0];
  assert.throws(() => policy.draft(before, { content: before.content, comments: before.comments.slice(1) }), /comment_removal_unconfirmed/);
  assert.throws(() => policy.draft(before, { content: before.content, comments: before.comments.map(value => ({ ...value, content: 'Changed' })) }), /comments_changed/);
  assert.equal(writes(f).length, 0);
});

for (const change of ['confirmation', 'disabled', 'foreign_comment', 'unsafe_comment', 'version', 'body', 'title',
  'comment', 'pending_edit', 'identity', 'ticket', 'streaming']) {
  test('dismiss fails before writing on ' + change, async () => {
    const f = setup(), list = await f.list(); let comment = 'first';
    if (change === 'disabled') f.page.__elonChatGptPrivateConversationMutationsEnabled = false;
    if (change === 'foreign_comment') comment = 'foreign';
    if (change === 'unsafe_comment') comment = 'first?reason=accept';
    if (change === 'version') f.rows[0].version += 1;
    if (change === 'body') f.rows[0].content += 'changed';
    if (change === 'title') f.rows[0].title = 'Changed';
    if (change === 'comment') f.rows[0].comments[0].content = 'Changed';
    if (change === 'pending_edit') f.edits.userEdits[ID] = [{ isPending: true }];
    if (change === 'identity') f.headers.Authorization = 'Bearer synthetic_other';
    if (change === 'ticket') list.ticket = 'expired';
    if (change === 'streaming') f.snapshot.streaming = true;
    assert.equal((await dismiss(f, list.ticket, comment, change !== 'confirmation')).ok, false);
    assert.equal(writes(f).length, 0);
  });
}

for (const mismatch of ['missing_version', 'old_version', 'wrong_version', 'body', 'title', 'kept_comment', 'retained_target', 'all_comments']) {
  test('successful HTTP alone does not prove dismissal: ' + mismatch, async () => {
    const f = setup(), list = await f.list();
    f.setHook(({ init }) => {
      if (init.method !== 'DELETE') return;
      f.rows[0].version = 5;
      if (mismatch !== 'retained_target') f.rows[0].comments.shift();
      if (mismatch === 'body') f.rows[0].content += 'changed';
      if (mismatch === 'title') f.rows[0].title = 'Changed';
      if (mismatch === 'kept_comment') f.rows[0].comments[0].content = 'Changed';
      if (mismatch === 'all_comments') f.rows[0].comments = [];
      return { payload: mismatch === 'missing_version' ? {} : { version: mismatch === 'old_version' ? 4 : mismatch === 'wrong_version' ? 6 : 5 } };
    });
    assert.equal((await dismiss(f, list.ticket)).code, 'canvas_write_unconfirmed');
    const fresh = await f.list(true);
    assert.equal(fresh.unconfirmedWrite, true);
    assert.equal((await f.save(fresh.ticket)).code, 'canvas_write_unconfirmed');
    assert.equal((await dismiss(f, fresh.ticket)).code, 'canvas_write_unconfirmed');
    assert.equal(writes(f).length, 1); assert.equal(f.invalidations.length, 0);
  });
}

test('lost DELETE response is verified by GET without a second DELETE or body save', async () => {
  const f = setup(), list = await f.list();
  f.setHook(({ init }) => { if (init.method === 'DELETE') {
    f.rows[0].comments.shift(); f.rows[0].version += 1; throw Error('timeout');
  } });
  assert.equal((await dismiss(f, list.ticket)).code, 'canvas_write_unconfirmed');
  const fresh = await f.list(true);
  const result = await f.run({ operation: 'verify', ticket: fresh.ticket, id: ID }, false);
  assert.equal(result.code, 'canvas_comment_dismissed'); assert.equal(result.unconfirmedWrite, false);
  assert.equal(writes(f).length, 1);
});

test('unchanged server state requires explicit acknowledgement of a failed dismissal', async () => {
  const f = setup(), list = await f.list();
  f.setHook(({ init }) => { if (init.method === 'DELETE') throw Error('timeout'); });
  await dismiss(f, list.ticket);
  const fresh = await f.list(true);
  const pending = await f.run({ operation: 'verify', ticket: fresh.ticket, id: ID }, false);
  assert.equal(pending.code, 'canvas_verification_pending'); assert.equal(pending.unconfirmedWrite, true);
  const result = await f.run({ operation: 'verify', ticket: pending.ticket, id: ID }, true);
  assert.equal(result.code, 'canvas_result_acknowledged'); assert.equal(result.unconfirmedWrite, false);
  assert.equal(writes(f).length, 1);
});

test('comment dismissal shares the existing single-flight write owner', async () => {
  const f = setup(), list = await f.list(); let release;
  f.setHook(({ init }) => init.method === 'DELETE' ? new Promise(resolve => { release = resolve; }) : undefined);
  const pending = dismiss(f, list.ticket);
  for (let i = 0; !release && i < 100; i++) await new Promise(resolve => setTimeout(resolve, 1));
  assert.equal(typeof release, 'function');
  assert.equal((await f.save(list.ticket)).code, 'canvas_busy');
  assert.equal((await dismiss(f, list.ticket)).code, 'canvas_busy');
  release(); assert.equal((await pending).code, 'canvas_comment_dismissed'); assert.equal(writes(f).length, 1);
});

test('reviewed public owner distinguishes comment DISMISS from ACCEPT and generated editing', {
  skip: !process.env.CHATGPT_PUBLIC_RUNTIME_DIR && 'Reviewed public assets are optional offline inputs.'
}, () => {
  const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto');
  const read = name => fs.readFileSync(path.join(process.env.CHATGPT_PUBLIC_RUNTIME_DIR, name), 'utf8');
  const source = read('conversation-small-h1dtzoris1y9588z.js');
  assert.equal(crypto.createHash('sha256').update(source).digest('hex'), 'da08c64c132306779e09ba89cac64fa560b120e7560ffdc29b3ce5a0b8ccd67e');
  const at = source.indexOf('kir=async('), end = source.indexOf('})),jir,', at);
  assert.ok(at > 0 && end > at);
  const owner = source.slice(at, end);
  assert.ok(owner.includes('safeDelete(`/textdoc/{textdoc_id}/{version}/comment/{comment_id}`'));
  assert.ok(owner.includes('query:{reason:r},path:{textdoc_id:t,version:String(e),comment_id:n}'));
  assert.ok(owner.endsWith(')).version'));
  assert.ok(source.includes('e.ACCEPT=`accept`,e.DISMISS=`dismiss`'));
  const editor = read('ac476d6c-foq8estmw3bbr7op.js');
  assert.ok(editor.includes('at=({id:e})=>{Xe(e,xt.DISMISS),mr.reset()}'));
  assert.ok(editor.includes('if(await Xe(n,xt.ACCEPT)===!1)return Je(!1);k('));
});
