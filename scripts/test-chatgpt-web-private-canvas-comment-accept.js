'use strict';
const test = require('node:test'), assert = require('node:assert/strict');
const { fixture, ID, SAVE } = require('./fixtures/chatgpt-web-private-canvas-documents.cjs');
function setup() {
  const f = fixture(); let invoked = 0, prepared = [], settled = false, fault = '';
  f.rows[0].content = 'A\u{1f600}BC';
  f.rows[0].comments = [{ id: 'comment', content: 'Synthetic suggestion', start: 1, end: 2 }];
  f.page.__elonChatGptPrivateCanvasGeneration = { create: () => ({ prepare: async (binding, original, input, check) => {
    prepared.push({ binding, original, input });
    return {
      validate() { check(); if (fault === 'prepare') throw Error('canvas_generation_blocked'); },
      invoke(before) {
        check(); if (fault === 'before') throw Error('canvas_generation_blocked');
        before(); invoked++;
        if (fault === 'after') throw Error('canvas_generation_unconfirmed');
      }, settled: () => settled
    };
  } }) };
  const selection = (index, operation = 'accept_comment') => ({ operation, ticket: index.ticket, scope: index.scope,
    id: ID, ...(operation === 'accept_comment' ? { commentId: 'comment' } : {}) });
  return { ...f, selection, invoked: () => invoked, prepared, finish: () => { settled = true; },
    fault: value => { fault = value; }, writes: () => f.requests.filter(row => row.init.method !== 'GET'),
    accept: async () => f.run(selection(await f.list()), true),
    verify: async (confirmed = false) => f.run(selection(await f.list(true), 'verify'), confirmed) };
}

test('accept is one versioned reason=accept DELETE followed by one original-context AI turn', async () => {
  const f = setup(), first = await f.list();
  assert.equal((await f.run(f.selection(first))).code, 'canvas_confirmation_required');
  assert.equal(f.writes().length, 0);
  const sent = await f.run(f.selection(first), true);
  assert.equal(sent.code, 'canvas_generation_dispatched'); assert.equal(sent.unconfirmedWrite, true);
  assert.equal(f.writes().length, 1); assert.equal(f.invoked(), 1);
  assert.equal(f.writes()[0].url, SAVE + '/4/comment/comment?reason=accept');
  assert.equal(f.writes()[0].init.body, undefined);
  assert.equal(f.prepared[0].original.documentVersion, 4);
  assert.equal(f.prepared[0].original.comments[0].end, 3);
  assert.deepEqual(f.prepared[0].input, { operation: 'accept_comment', commentId: 'comment' });
  assert.equal(sent.documents[0].documentVersion, 5); assert.equal(sent.documents[0].content, first.documents[0].content);
  assert.equal((await f.verify(true)).code, 'canvas_generation_pending');
  assert.equal(f.api.generationPending(), true);
  f.finish(); f.rows[0].version = 6; f.rows[0].content = 'AI revised source';
  const done = await f.verify(); assert.equal(done.code, 'canvas_generated'); assert.equal(done.unconfirmedWrite, false);
  assert.equal(f.api.generationPending(), false);
});

test('comment removal version alone is never reported as AI content generation', async () => {
  const f = setup(); await f.accept(); f.finish();
  const done = await f.verify(); assert.equal(done.code, 'canvas_generation_no_change');
  assert.equal(done.documents[0].documentVersion, 5); assert.equal(f.invoked(), 1);
});

for (const fault of ['timeout', 'malformed', 'before']) {
  test('confirmed removal after ' + fault + ' can resume the unsent AI turn without deleting twice', async () => {
    const f = setup();
    if (fault === 'before') f.fault(fault);
    else f.setHook(async ({ init }) => {
      if (init.method !== 'DELETE') return;
      f.rows[0].comments = []; f.rows[0].version = 5;
      if (fault === 'timeout') throw Error('timeout');
      return { payload: { version: '5' } };
    });
    assert.equal((await f.accept()).code, 'canvas_comment_accept_unconfirmed');
    assert.equal(f.invoked(), 0); assert.equal(f.api.generationPending(), true);
    const verified = await f.verify(); assert.equal(verified.code, 'canvas_comment_accept_ready');
    assert.equal(verified.unconfirmedWrite, true);
    assert.equal((await f.run(f.selection(verified), true)).code, 'canvas_write_unconfirmed');
    assert.equal((await f.run(f.selection(verified, 'resume_comment'))).code, 'canvas_confirmation_required');
    f.fault('');
    const resumed = await f.run(f.selection(verified, 'resume_comment'), true);
    assert.equal(resumed.code, 'canvas_generation_dispatched'); assert.equal(f.invoked(), 1); assert.equal(f.writes().length, 1);
    assert.equal((await f.run(f.selection(resumed, 'resume_comment'), true)).code, 'canvas_write_unconfirmed');
  });
}

test('explicit stop after partial success retains original source and never sends AI', async () => {
  const f = setup(); f.fault('before'); await f.accept();
  const pending = await f.verify(); assert.equal(pending.code, 'canvas_comment_accept_ready');
  const stopped = await f.run(f.selection(pending, 'verify'), true);
  assert.equal(stopped.code, 'canvas_comment_accept_stopped'); assert.equal(stopped.unconfirmedWrite, false);
  assert.equal(stopped.documents[0].content, 'A\u{1f600}BC'); assert.equal(f.invoked(), 0); assert.equal(f.api.generationPending(), false);
  assert.equal((await f.run(f.selection(stopped, 'resume_comment'), true)).code, 'canvas_write_unconfirmed');
});

test('uncertain AI dispatch cannot be stopped or resumed as an unsent request', async () => {
  const f = setup(); f.fault('after');
  assert.equal((await f.accept()).code, 'canvas_generation_unconfirmed');
  const pending = await f.verify(true);
  assert.equal(pending.code, 'canvas_generation_pending'); assert.equal(pending.unconfirmedWrite, true);
  assert.equal((await f.run(f.selection(pending, 'resume_comment'), true)).code, 'canvas_write_unconfirmed');
  assert.equal(f.invoked(), 1); assert.equal(f.writes().length, 1);
});

test('a rejected deletion is not followed by AI and only explicit comparison clears unknown result', async () => {
  const f = setup(); f.setHook(async ({ init }) => { if (init.method === 'DELETE') throw Error('http_403'); });
  assert.equal((await f.accept()).code, 'canvas_comment_accept_unconfirmed');
  let pending = await f.verify(); assert.equal(pending.code, 'canvas_verification_pending');
  assert.equal((await f.run(f.selection(pending, 'resume_comment'), true)).code, 'canvas_comment_accept_unconfirmed');
  pending = await f.verify(true); assert.equal(pending.code, 'canvas_comment_accept_stopped');
  assert.equal(f.invoked(), 0); assert.equal(f.writes().length, 1); assert.deepEqual(f.rejectedAuth, ['canvas_rejected']);
});

test('pre-delete permission and original-version conflicts perform no mutation', async () => {
  const f = setup(); f.fault('prepare');
  assert.equal((await f.accept()).code, 'canvas_generation_blocked'); assert.equal(f.writes().length, 0);
  f.fault(''); const first = await f.list(); f.rows[0].version++;
  assert.equal((await f.run(f.selection(first), true)).code, 'canvas_version_conflict'); assert.equal(f.writes().length, 0);
});

test('changed original after deletion prevents a stale AI edit and cannot resume', async () => {
  const f = setup();
  f.setHook(async ({ init }) => {
    if (init.method !== 'DELETE') return;
    f.rows[0].version = 5; f.rows[0].comments = []; f.rows[0].content = 'Different edit';
    return { payload: { version: 5 } };
  });
  assert.equal((await f.accept()).code, 'canvas_comment_accept_unconfirmed');
  const pending = await f.verify(); assert.equal(pending.code, 'canvas_verification_pending');
  assert.equal((await f.run(f.selection(pending, 'resume_comment'), true)).code, 'canvas_comment_accept_unconfirmed');
  assert.equal(f.invoked(), 0); assert.equal(f.writes().length, 1);
});

test('partial acceptance cannot move to another document or account', async () => {
  const f = setup(); f.fault('before'); await f.accept();
  const pending = await f.verify(); f.page.__elonChatGptDocumentToken = 'doc_other';
  assert.equal((await f.run(f.selection(pending, 'resume_comment'), true)).code, 'canvas_write_unconfirmed');
  assert.equal(f.invoked(), 0);
});

test('duplicate taps during the DELETE do not create a second mutation', async () => {
  const f = setup(); let entered, release;
  const waiting = new Promise(resolve => { entered = resolve; });
  f.setHook(async ({ init }) => {
    if (init.method === 'DELETE') { entered(); await new Promise(resolve => { release = resolve; }); }
  });
  const first = await f.list(), request = f.selection(first), sending = f.run(request, true);
  await waiting;
  assert.equal((await f.run(request, true)).code, 'canvas_busy');
  release(); assert.equal((await sending).code, 'canvas_generation_dispatched');
  assert.equal(f.writes().length, 1); assert.equal(f.invoked(), 1);
});
