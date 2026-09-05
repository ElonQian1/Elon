'use strict';

const assert = require('node:assert/strict');
const { test } = require('node:test');
const evidence = require('../android/app/src/main/assets/chatgpt_web_private_protocol_evidence.js');
const jsonRequest = require('../android/app/src/main/assets/chatgpt_web_private_json_request.js');

function fixture(overrides = {}) {
  const timers = new Map();
  let sequence = 0;
  const root = {
    AbortController, Response, FormData,
    __elonChatGptPrivateJsonRequest: jsonRequest,
    setTimeout: (callback, ms) => { const id = ++sequence; timers.set(id, { callback, ms }); return id; },
    clearTimeout: (id) => timers.delete(id), ...overrides,
  };
  const probe = evidence.create(root, (url) => url.pathname.replace(/fixture-secret/g, '{id}'));
  const command = (mode) => JSON.parse(probe.command(mode));
  const begin = (init = {}, input = null) => probe.begin(input, init,
    new URL('https://chatgpt.com/backend-api/files/fixture-secret?token=hidden'),
    init.method || 'POST', 'fetch');
  const expire = (ms) => {
    for (const [id, timer] of [...timers]) if (timer.ms === ms) {
      timers.delete(id); timer.callback();
    }
  };
  return { probe, command, begin, timers, expire };
}

const tick = async () => { for (let i = 0; i < 8; i++) await new Promise(setImmediate); };
const json = (value, status = 200) => new Response(JSON.stringify(value), {
  status, headers: { 'content-type': 'application/json' },
});

test('default is inactive with no timer, read, network or automatic start', () => {
  const f = fixture();
  assert.equal(f.probe.active(), false);
  assert.equal(f.begin({ body: '{"secret":"untouched"}' }), null);
  assert.equal(f.timers.size, 0);
  assert.deepEqual(f.command('read'), {
    schema: 'elon.private_protocol_probe.v1', active: false, dropped: 0, records: [],
  });
  assert.equal(f.probe.command('post'), null);
});

test('JSON evidence includes types, never field values, headers or queries', async () => {
  const f = fixture();
  f.command('start');
  const record = f.begin({ body: JSON.stringify({ file_id: 'private-id',
    metadata: { name: 'private-name', done: true }, messages: ['private-text'] }),
    headers: { Authorization: 'private-key' } });
  const original = json({ upload_url: 'https://private.example/?sig=private-token', ok: true });
  f.probe.response(record, original);
  assert.equal((await original.json()).ok, true);
  await tick();
  const result = f.command('stop');
  assert.equal(result.records[0].path, '/backend-api/files/{id}');
  assert.deepEqual(result.records[0].requestFields,
    ['$:object', '$.file_id:string', '$.metadata:object', '$.metadata.name:string',
      '$.metadata.done:boolean', '$.messages:array', '$.messages[]:string']);
  assert.deepEqual(result.records[0].responseFields,
    ['$:object', '$.upload_url:string', '$.ok:boolean']);
  assert.doesNotMatch(JSON.stringify(result), /private-|hidden|Authorization|https:/);
  assert.equal(f.timers.size, 0);
});

test('error JSON is observable without consuming or changing original status', async () => {
  const f = fixture(); f.command('start');
  const record = f.begin();
  const original = json({ error: { code: 'secret-value' } }, 403);
  f.probe.response(record, original);
  assert.equal(original.status, 403);
  assert.equal((await original.json()).error.code, 'secret-value');
  await tick();
  assert.equal(record.status, 403);
  assert.equal(record.responseState, 'ready');
  assert.deepEqual(record.responseFields, ['$:object', '$.error:object', '$.error.code:string']);
  f.command('stop');
});

test('multipart records field types, not bytes, filenames or strings', () => {
  const f = fixture(); f.command('start');
  const body = new FormData();
  body.append('file', new Blob(['private-file-bytes']), 'private-filename.txt');
  body.append('purpose', 'private-purpose');
  const record = f.begin({ body });
  assert.equal(record.requestKind, 'multipart');
  assert.deepEqual(record.requestFields, ['$.file:file', '$.purpose:string']);
  assert.doesNotMatch(JSON.stringify(f.command('stop')), /private-/);
});

test('Request-object JSON clone leaves the original body usable', async () => {
  const f = fixture(); f.command('start');
  const request = new Request('https://chatgpt.com/backend-api/files', {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: '{"size":12}',
  });
  const record = f.begin({}, request);
  assert.deepEqual(await request.json(), { size: 12 });
  await tick();
  assert.deepEqual(record.requestFields, ['$:object', '$.size:number']);
  f.command('stop');
});

test('binary and SSE responses are not cloned or read', () => {
  const f = fixture(); f.command('start');
  for (const contentType of ['image/png', 'text/event-stream']) {
    const record = f.begin();
    f.probe.response(record, { status: 200, headers: new Headers({ 'content-type': contentType }),
      clone: () => assert.fail('must not clone binary or SSE') });
    assert.equal(record.responseState, 'skipped');
  }
  f.command('stop');
});

test('body size bound rejects synchronous and streamed oversized input', async () => {
  const f = fixture(); f.command('start');
  const record = f.begin({ body: JSON.stringify({ text: 'x'.repeat(65536) }) });
  assert.equal(record.requestState, 'oversize');
  f.probe.response(record, json({ text: 'x'.repeat(65536) }));
  await tick();
  assert.equal(record.responseState, 'oversize');
  assert.deepEqual(record.responseFields, []);
  f.command('stop');
});

test('clone read deadline settles even if a body ignores cancellation', async () => {
  const f = fixture(); f.command('start');
  const record = f.begin();
  let resolveText;
  const stalled = { status: 200, headers: new Headers({ 'content-type': 'application/json' }),
    text: () => new Promise((resolve) => { resolveText = resolve; }) };
  f.probe.response(record, { ...stalled, clone: () => stalled });
  await tick();
  f.expire(2000);
  await tick();
  assert.equal(record.responseState, 'timeout');
  resolveText('{"late":"secret"}');
  await tick();
  assert.deepEqual(record.responseFields, []);
  f.command('stop');
});

test('stop, restart and expiration reject stale readers and release all timers', async () => {
  let deliver;
  const f = fixture({ __elonChatGptPrivateJsonRequest: {
    request: () => new Promise((resolve) => { deliver = resolve; }),
  } });
  f.command('start');
  const record = f.begin();
  f.probe.response(record, json({ ignored: true }));
  assert.equal(f.command('stop').records[0].responseState, 'cancelled');
  f.command('start');
  deliver({ text: '{"late":"secret"}' });
  await tick();
  assert.deepEqual(f.command('read').records, []);
  f.expire(60000);
  assert.equal(f.probe.active(), false);
  assert.equal(f.timers.size, 0);
});

test('caps records, field depth and output; clear removes evidence', () => {
  const f = fixture(); f.command('start');
  const body = JSON.stringify(Object.fromEntries(Array.from({ length: 40 }, (_, i) => [
    'field' + i, { nested: { nested: { secret: 'not-recorded' } } },
  ])));
  for (let i = 0; i < 50; i++) f.begin({ body });
  const value = f.command('read');
  assert.equal(value.records.length, 12);
  assert.equal(value.dropped, 38);
  assert.ok(value.records.every((record) => record.requestFields.length <= 12));
  assert.ok(JSON.stringify(value).length <= 12000);
  assert.doesNotMatch(JSON.stringify(value), /not-recorded|nested\.nested\.secret/);
  assert.deepEqual(f.command('clear').records, []);
  assert.equal(f.timers.size, 0);
});

test('background timer throttling cannot extend the capture lease', () => {
  let time = 1000;
  const f = fixture({ Date: { now: () => time } });
  f.command('start');
  const record = f.begin();
  time += 60001;
  assert.equal(f.begin(), null);
  f.probe.xhrResponse(record, 200, 'application/json', '{"late":true}');
  assert.equal(f.command('read').active, false);
  assert.equal(record.responseState, 'cancelled');
  assert.equal(f.timers.size, 0);
});

test('nested array field paths remain within the native receipt contract', () => {
  const f = fixture(); f.command('start');
  const record = f.begin({ body: JSON.stringify({ ['a'.repeat(39)]: { ['b'.repeat(30)]: [true] } }) });
  assert.ok(record.requestFields.every((field) => field.length <= 80));
  assert.ok(record.requestFields.some((field) => field.endsWith(':array')));
  f.command('stop');
});

test('missing helper cancels only the diagnostic clone and reports unavailable', () => {
  const f = fixture({ __elonChatGptPrivateJsonRequest: null }); f.command('start');
  const record = f.begin();
  let cancelled = 0;
  f.probe.response(record, { status: 200, headers: new Headers({ 'content-type': 'application/json' }),
    clone: () => ({ body: { cancel: () => { cancelled++; } } }) });
  assert.equal(record.responseState, 'unavailable');
  assert.equal(cancelled, 1);
  f.command('stop');
});
