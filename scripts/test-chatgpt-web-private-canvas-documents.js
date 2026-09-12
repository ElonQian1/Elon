'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { fixture, CID, ID, PATH, LIST, SAVE, doc } = require('./fixtures/chatgpt-web-private-canvas-documents.cjs');

test('original list requires identity, not DOM, composer, or runtime imports; warm reads reuse the cache', async () => {
  const f = fixture(); f.page.__elonChatGptPrivateRuntimeBindings = null;
  const first = await f.list(); assert.equal(first.ok, true); assert.equal(first.documents[0].id, ID);
  assert.equal(first.documents[0].documentVersion, 4); assert.equal(f.imports(), 0);
  assert.equal((await f.list()).ticket, first.ticket); assert.equal(f.requests.length, 1);
  assert.notEqual((await f.list(true)).ticket, first.ticket); assert.equal(f.requests.length, 2);
  assert.equal(f.requests[0].url, LIST);
  assert.equal(f.requests[0].init.credentials, 'include'); assert.equal(f.requests[0].init.headers.Cookie, undefined);
  assert.equal(f.requests[0].init.redirect, 'error'); assert.equal(f.requests[0].init.body, undefined);
});

test('a selected original saves exactly once, confirms by readback, and invalidates only its data query', async () => {
  const f = fixture(), read = await f.list(), saved = await f.save(read.ticket);
  assert.equal(saved.ok, true); assert.equal(saved.code, 'canvas_saved'); assert.equal(saved.attempted, true);
  assert.equal(saved.documents[0].content, 'Edited content'); assert.equal(saved.documents[0].documentVersion, 5);
  assert.deepEqual(f.requests.map(value => [value.url, value.init.method]), [[LIST, 'GET'], [LIST, 'GET'], [SAVE, 'POST'], [LIST, 'GET']]);
  assert.deepEqual(JSON.parse(f.requests[2].init.body), { version: 4, content: 'Edited content', comments: [] });
  assert.deepEqual(f.invalidations, [{ queryKey: [CID, 'textdocs'], exact: true, refetchType: 'active' }]);
  assert.equal((await f.save(read.ticket)).code, 'canvas_selection_expired');
  assert.equal(f.requests.filter(value => value.init.method === 'POST').length, 1);
});

test('an unchanged native draft does not POST or invalidate the website', async () => {
  const f = fixture(), read = await f.list();
  const result = await f.save(read.ticket, { content: doc().content });
  assert.equal(result.code, 'canvas_unchanged'); assert.equal(result.attempted, false);
  assert.equal(f.requests.length, 2); assert.deepEqual(f.invalidations, []);
});

for (const state of ['pending', 'debounced', 'streaming', 'dictation', 'missing_runtime', 'unknown_store', 'delete_busy', 'disabled']) {
  test('no write when the source context is not safe: ' + state, async () => {
    const f = fixture(), read = await f.list();
    if (state === 'pending') f.edits.userEdits[ID] = [{ isPending: true }];
    if (state === 'debounced') f.edits.timestamps[ID] = { lastTriggeredAt: 10, lastFlushedAt: null };
    if (state === 'streaming') f.snapshot.streaming = true;
    if (state === 'dictation') f.snapshot.dictationCapturePending = true;
    if (state === 'missing_runtime') f.page.__elonChatGptPrivateRuntimeBindings.observed = () => false;
    if (state === 'unknown_store') f.edits.userEdits[ID] = [{}];
    if (state === 'delete_busy') f.page.__elonChatGptPrivateConversationDelete = { busy: () => true };
    if (state === 'disabled') f.page.__elonChatGptPrivateConversationMutationsEnabled = false;
    const result = await f.save(read.ticket);
    assert.equal(result.ok, false); assert.equal(result.attempted, false);
    assert.equal(f.requests.filter(value => value.init.method === 'POST').length, 0);
    assert.deepEqual(f.invalidations, []);
  });
}

test('official null reset timestamps and completed queued edits are not mistaken for pending edits', async () => {
  const f = fixture(), read = await f.list();
  f.edits.timestamps[ID] = { lastTriggeredAt: null, lastFlushedAt: null };
  f.edits.userEdits[ID] = [{ isPending: false, basedOnVersionInt: 3, nextVersionInt: 4 }];
  assert.equal((await f.save(read.ticket)).ok, true);
});

test('a mixed or changed website build during preflight cannot reuse old editor state', async () => {
  const f = fixture(), read = await f.list();
  f.setHook(() => { f.page.__elonChatGptPrivateRuntimeBindings.state = () => ({ profile_id: '' }); });
  assert.equal((await f.save(read.ticket)).code, 'canvas_runtime_unavailable');
  assert.equal(f.requests.filter(value => value.init.method === 'POST').length, 0);
});

for (const change of ['account', 'document', 'token', 'route', 'expired', 'clock_backwards', 'ticket', 'id', 'confirmation']) {
  test('selection cannot migrate across identity, document or owner: ' + change, async () => {
    const f = fixture(), read = await f.list(), patch = {};
    if (change === 'account') f.headers.Authorization = 'Bearer synthetic-other';
    if (change === 'document') f.page.document = {};
    if (change === 'token') f.page.__elonChatGptDocumentToken = 'doc_other';
    if (change === 'route') f.navigate('/c/22222222-2222-4222-8222-222222222222');
    if (change === 'expired') f.advance(1800001);
    if (change === 'clock_backwards') f.advance(-1001);
    if (change === 'ticket') read.ticket = 'wrong';
    if (change === 'id') patch.id = 'not_in_conversation';
    const result = await f.save(read.ticket, patch, change !== 'confirmation');
    assert.equal(result.ok, false); assert.equal(result.attempted, false); assert.equal(f.requests.length, 1);
  });
}

for (const patch of [{ version: 5 }, { content: 'Concurrent edit' }, { title: 'Renamed elsewhere' },
  { textdoc_type: 'code/python' }, { comments: [{ id: 'comment_new', content: 'New comment', start: 0, end: 1 }] }]) {
  test('fresh preflight detects version or document drift: ' + JSON.stringify(patch), async () => {
    const f = fixture(), read = await f.list(); Object.assign(f.rows[0], patch);
    assert.equal((await f.save(read.ticket)).code, 'canvas_version_conflict');
    assert.equal(f.requests.length, 2); assert.equal(f.requests[1].init.method, 'GET');
  });
}

for (const moment of ['preflight', 'dispatch']) {
  test('website edits starting while native preflight waits are preserved: ' + moment, async () => {
    const f = fixture(), read = await f.list();
    f.setHook(() => { f.edits.userEdits[ID] = [{ isPending: true }]; });
    if (moment === 'dispatch') {
      const copy = f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders;
      f.setHook(async () => {});
      f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => {
        if (f.requests.length >= 2) f.edits.userEdits[ID] = [{ isPending: true }];
        return copy();
      };
    }
    const result = await f.save(read.ticket);
    assert.equal(result.ok, false); assert.equal(result.attempted, false);
    assert.equal(f.requests.filter(value => value.init.method === 'POST').length, 0);
    assert.equal(f.edits.userEdits[ID][0].isPending, true);
  });
}

for (const failure of ['timeout', 'http_403', 'http_409', 'response_version', 'readback', 'readback_drift', 'account', 'document']) {
  test('uncertain save is neither replayed nor reported as saved: ' + failure, async () => {
    const f = fixture(), read = await f.list(); let writes = 0;
    f.setHook(({ init }) => {
      if (init.method === 'POST') {
        writes += 1;
        if (['timeout', 'http_403', 'http_409'].includes(failure)) throw Error(failure);
        if (failure === 'response_version') return { payload: { version: 4 } };
        if (failure === 'account') f.headers.Authorization = 'Bearer synthetic-different';
        if (failure === 'document') f.page.document = {};
      } else if (writes) {
        if (failure === 'readback') throw Error('timeout');
        if (failure === 'readback_drift') f.rows[0].content = 'Another writer';
      }
    });
    const result = await f.save(read.ticket);
    assert.equal(result.ok, false); assert.equal(result.attempted, true); assert.equal(result.code, 'canvas_write_unconfirmed');
    assert.deepEqual(f.invalidations, []); assert.equal((await f.save(read.ticket)).ok, false); assert.equal(writes, 1);
    if (failure === 'http_403') assert.deepEqual(f.rejectedAuth, ['canvas_rejected']);
  });
}

test('a server-success/network-failure is recoverable by GET verification without a second POST', async () => {
  const f = fixture(), read = await f.list(); let once = true;
  f.setHook(({ init }) => {
    if (init.method === 'POST' && once) {
      once = false; Object.assign(f.rows[0], JSON.parse(init.body), { version: 5 }); throw Error('timeout');
    }
  });
  assert.equal((await f.save(read.ticket)).code, 'canvas_write_unconfirmed');
  const reread = await f.list(true); assert.equal(reread.unconfirmedWrite, true);
  assert.equal((await f.save(reread.ticket)).code, 'canvas_write_unconfirmed');
  const verified = await f.run({ operation: 'verify', ticket: reread.ticket, id: ID });
  assert.equal(verified.code, 'canvas_saved'); assert.equal(verified.unconfirmedWrite, false);
  assert.equal(f.requests.filter(value => value.init.method === 'POST').length, 1);
});

test('a different readback requires explicit comparison before accepting its result', async () => {
  const f = fixture(), read = await f.list();
  f.setHook(({ init }) => { if (init.method === 'POST') throw Error('timeout'); });
  await f.save(read.ticket);
  const reread = await f.list(true);
  const mismatch = await f.run({ operation: 'verify', ticket: reread.ticket, id: ID });
  assert.equal(mismatch.code, 'canvas_verification_pending'); assert.equal(mismatch.unconfirmedWrite, true);
  const acknowledged = await f.run({ operation: 'verify', ticket: mismatch.ticket, id: ID }, true);
  assert.equal(acknowledged.code, 'canvas_result_acknowledged'); assert.equal(acknowledged.unconfirmedWrite, false);
  assert.equal(f.requests.filter(value => value.init.method === 'POST').length, 1);
});

test('comparison cannot acknowledge data that changed again after being displayed', async () => {
  const f = fixture(), read = await f.list();
  f.setHook(({ init }) => { if (init.method === 'POST') throw Error('timeout'); });
  await f.save(read.ticket); const reread = await f.list(true);
  f.rows[0].content = 'Newer conflict';
  assert.equal((await f.run({ operation: 'verify', ticket: reread.ticket, id: ID }, true)).code, 'canvas_version_conflict');
  assert.equal((await f.list(true)).unconfirmedWrite, true);
});

test('a temporary conversation switch does not discard the unknown write guard', async () => {
  const f = fixture(), read = await f.list();
  f.setHook(({ init }) => { if (init.method === 'POST') throw Error('timeout'); });
  await f.save(read.ticket); f.navigate('/c/22222222-2222-4222-8222-222222222222');
  const other = await f.list(); assert.equal((await f.save(other.ticket)).code, 'canvas_write_unconfirmed');
  f.navigate(PATH); assert.equal((await f.list()).unconfirmedWrite, true);
});

test('a website draft created during POST survives; native confirmation does not overwrite that queue', async () => {
  const f = fixture(), read = await f.list();
  f.setHook(({ init }) => { if (init.method === 'POST') f.edits.userEdits[ID] = [{ isPending: true, content: 'Website draft' }]; });
  const saved = await f.save(read.ticket); assert.equal(saved.code, 'canvas_saved');
  assert.deepEqual(f.invalidations, []); assert.equal(f.edits.userEdits[ID][0].content, 'Website draft');
});

test('native save confirmation is not blocked by website query refetch completion', async () => {
  const f = fixture(), read = await f.list(), original = f.page.__elonChatGptPrivateRuntimeBindings.load;
  f.page.__elonChatGptPrivateRuntimeBindings.load = async role => {
    const value = await original(role);
    return role === 'shared' ? { canvasQueryClient: () => ({ ...value.canvasQueryClient(),
      invalidateQueries: () => new Promise(() => {}) }) } : value;
  };
  assert.equal((await f.save(read.ticket)).code, 'canvas_saved');
});

test('the total deadline bounds preflight, write and readback together', async () => {
  const f = fixture(), read = await f.list();
  f.setHook(() => { f.advance(6500); });
  const saved = await f.save(read.ticket);
  assert.equal(saved.code, 'canvas_write_unconfirmed'); assert.ok(f.requests.at(-1).limits.timeoutMs <= 5000);
  assert.equal(f.requests.filter(value => value.init.method === 'POST').length, 1);
});

test('single flight prevents overlapping saves and list reads invalidating selection mid-save', async () => {
  const f = fixture(), read = await f.list(); let release;
  f.setHook(() => new Promise(resolve => { release = resolve; }));
  const first = f.save(read.ticket);
  while (!release) await new Promise(resolve => setImmediate(resolve));
  assert.equal((await f.save(read.ticket)).code, 'canvas_busy'); assert.equal((await f.list(true)).code, 'canvas_busy');
  f.setHook(async () => {}); release(); assert.equal((await first).code, 'canvas_saved');
});

test('API failure is an error, not an empty Canvas list', async () => {
  const f = fixture(); f.setHook(() => { throw Error('http_503'); });
  const result = await f.list(); assert.equal(result.ok, false); assert.equal(result.documents, undefined);
  assert.equal(result.code, 'canvas_http_503');
});
