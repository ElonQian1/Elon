'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, response, emptyPage } = require('./fixtures/chatgpt-directory-refresh');
const row = offset => ({ id: 'row-' + offset, title: 'Synthetic fixture' });
const offsetOf = url => Number(new URL(url, 'https://chatgpt.com').searchParams.get('offset'));
const history = offset => response({ items: [row(offset)], offset, limit: 28, total: 84 });

test('only a timed-out global page gets a longer retry, reset after a successful page', async () => {
  let fail = true;
  const f = fixture(async url => {
    if (url.includes('/snorlax/')) return response(emptyPage(url));
    if (fail && offsetOf(url) === 28) return new Promise(() => {});
    return history(offsetOf(url));
  });
  const deadlines = [];
  f.root.setTimeout = (fn, ms) => { deadlines.push(ms); return setTimeout(fn, ms === 4000 ? 10 : ms); };
  assert.equal((await f.controller.refresh()).code, 'directory_timeout');
  assert.equal((await f.controller.refresh()).code, 'directory_timeout');
  assert.equal(deadlines.at(-1), 8000, 'the second attempt remains bounded');
  fail = false;
  deadlines.length = 0;
  assert.equal((await f.controller.refresh()).code, 'directory_ready');
  assert.deepEqual(deadlines, [7000, 8000, 4000]);
  assert.deepEqual(f.calls.filter(c => !c.url.includes('/snorlax/')).map(c => offsetOf(c.url)), [0, 28, 28, 28, 56]);
});

test('retry resumes the failed ordinary page and reuses the completed project catalog', async () => {
  let fail = true;
  const f = fixture(async url => {
    if (url.includes('/snorlax/')) return response(emptyPage(url));
    const offset = offsetOf(url);
    return fail && offset === 28 ? response({}, 503) : history(offset);
  });
  assert.equal((await f.controller.refresh()).ok, false);
  fail = false;
  assert.equal((await f.controller.refresh()).code, 'directory_ready');
  assert.deepEqual(f.calls.filter(c => !c.url.includes('/snorlax/')).map(c => offsetOf(c.url)), [0, 28, 28, 56]);
  assert.equal(f.calls.filter(c => c.url.includes('/snorlax/')).length, 1);
  const diagnostic = f.controller.diagnostics();
  assert.equal(diagnostic.reads.find(r => r.family === 'conversations').resumedPages, 1);
  assert.equal(diagnostic.reads.find(r => r.family === 'projects').requests, 0);
  assert.deepEqual(JSON.parse(f.accepted.at(-1)[1]).items.map(r => r.id), ['row-0', 'row-28', 'row-56']);
  await f.controller.refresh();
  assert.equal(f.calls.filter(c => c.url.includes('/snorlax/')).length, 2, 'a finished cycle is not a TTL cache');
});

test('page admission budget never shortens an admitted page deadline', async () => {
  let time = 0;
  const f = fixture(async url => {
    if (url.includes('/snorlax/')) return response(emptyPage(url));
    time += 3900;
    const offset = offsetOf(url);
    return response({ items: [row(offset)], offset, limit: 28, total: 140 });
  });
  f.root.performance = { now: () => time };
  const deadlines = [];
  f.root.setTimeout = (callback, ms) => { deadlines.push(ms); return setTimeout(callback, ms); };
  const first = await f.controller.refresh();
  assert.equal(first.code, 'directory_partial');
  assert.equal(first.conversationsComplete, false);
  assert(deadlines.every(ms => ms === 4000 || ms === 7000));
  const count = f.calls.length;
  assert.equal((await f.controller.refresh()).code, 'directory_ready');
  assert.equal(f.calls.length, count + 1);
  assert.equal(offsetOf(f.calls.at(-1).url), 112);
});

for (const change of ['account', 'token', 'document', 'transport', 'cancel', 'expiry']) {
  test(`global continuation is discarded after ${change}`, async () => {
    let time = 0;
    const f = fixture(async url => url.includes('/snorlax/') ? response(emptyPage(url)) :
      offsetOf(url) === 28 ? response({}, 503) : history(0));
    f.root.performance = { now: () => time };
    await f.controller.refresh();
    const previous = f.calls.length;
    if (change === 'account') f.headers['chatgpt-account-id'] = 'replacement-account';
    if (change === 'token') f.headers.Authorization = 'Bearer replacement-fixture-auth';
    if (change === 'document') f.root.__elonChatGptDocumentToken = 'doc_new_fixture';
    if (change === 'transport') f.root.__elonChatGptPrivateTransport = { ...f.root.__elonChatGptPrivateTransport };
    if (change === 'cancel') f.controller.cancel();
    if (change === 'expiry') time = 60001;
    await f.controller.refresh();
    assert.equal(offsetOf(f.calls[previous].url), 0);
    assert.equal(f.calls.filter(c => c.url.includes('/snorlax/')).length, 2);
  });
}

test('a failed page does not extend the age of a continuation', async () => {
  let time = 0;
  const f = fixture(async url => url.includes('/snorlax/') ? response(emptyPage(url)) :
    offsetOf(url) === 28 ? response({}, 503) : history(0));
  f.root.performance = { now: () => time };
  await f.controller.refresh();
  time = 59000;
  await f.controller.refresh();
  time = 60001;
  const previous = f.calls.length;
  await f.controller.refresh();
  assert.equal(offsetOf(f.calls[previous].url), 0);
});

test('malformed pagination is not reused as a retry checkpoint', async () => {
  const f = fixture(async url => url.includes('/snorlax/') ? response(emptyPage(url)) :
    offsetOf(url) === 28 ? response({ unrelated: true }) : history(0));
  assert.equal((await f.controller.refresh()).code, 'directory_response_invalid');
  const previous = f.calls.length;
  await f.controller.refresh();
  assert.equal(offsetOf(f.calls[previous].url), 0);
});

test('diagnostics contain only bounded structural data and expire with the document', async () => {
  const f = fixture();
  assert.equal(f.controller.diagnostics().observed, false);
  await f.controller.refresh();
  const value = f.controller.diagnostics();
  assert.equal(value.observed, true);
  assert.equal(value.reads.length, 2);
  const text = JSON.stringify(value);
  for (const secret of ['fixture', 'Bearer', 'account', 'cursor', 'title', '/backend-api/']) assert(!text.includes(secret));
  value.reads.length = 0;
  assert.equal(f.controller.diagnostics().reads.length, 2, 'callers cannot mutate stored evidence');
  f.root.__elonChatGptDocumentToken = 'doc_new_fixture';
  assert.equal(f.controller.diagnostics().observed, false);
});
