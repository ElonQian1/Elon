'use strict';

const assert = require('node:assert/strict');
const { test } = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const evidence = require(path.join(assets, 'chatgpt_web_private_protocol_evidence.js'));
const jsonRequest = require(path.join(assets, 'chatgpt_web_private_json_request.js'));
const source = fs.readFileSync(path.join(assets, 'chatgpt_web_private_research_probe.js'), 'utf8');

function fixture(options = {}) {
  const events = [], requests = [];
  class Xhr {
    constructor() { this.listeners = {}; this.status = 201; this.responseText = '{"id":"secret"}'; }
    open() {}
    send() { this.sent = true; }
    getResponseHeader() { return 'application/json'; }
    addEventListener(name, handler) { this.listeners[name] = handler; }
  }
  const location = { origin: 'https://chatgpt.com', href: 'https://chatgpt.com/' };
  const window = {
    location, AbortController, Response, FormData, setTimeout, clearTimeout,
    __elonChatGptDocumentToken: 'doc_probe_test',
    __elonChatGptPrivateResearchEnabled: false,
    __elonChatGptPrivateProtocolEvidence: evidence,
    __elonChatGptPrivateJsonRequest: jsonRequest,
    elonChatGptNative: { postMessage: (value) => events.push(JSON.parse(value)) },
    XMLHttpRequest: Xhr,
    fetch: (input, init) => {
      requests.push({ input, init });
      return options.fetch ? options.fetch(input, init) : Promise.resolve(new Response('{"id":"secret"}', {
        headers: { 'content-type': 'application/json' },
      }));
    }, ...options.window,
  };
  const context = { window, location, URL, WeakMap, Promise, Date, Number, Math, JSON };
  vm.runInNewContext(source, context);
  const probe = window.__elonChatGptPrivateResearchProbe;
  const command = (mode) => {
    let result;
    assert.equal(probe.handle('private_protocol_probe', { value: mode }, (action, ok, detail) => {
      assert.equal(action, 'private_protocol_probe');
      result = { ok, detail };
    }), true);
    return result;
  };
  const read = () => JSON.parse(command('read').detail);
  return { window, context, probe, command, read, events, requests };
}

const tick = async () => { for (let i = 0; i < 8; i++) await new Promise(setImmediate); };

test('production observer stays dormant, preserves promise identity and emits no legacy telemetry', async () => {
  const original = Promise.resolve(new Response('untouched'));
  const f = fixture({ fetch: () => original });
  assert.equal(f.window.fetch('/backend-api/conversations'), original);
  assert.equal(await (await original).text(), 'untouched');
  for (const method of ['recordPrivateOutcome', 'recordPrivateStreamOutcome',
    'recordPrivateStreamShape', 'recordPrivatePayloadShape']) f.probe[method]('success', 1, 12);
  assert.deepEqual(f.events, []);
  assert.deepEqual(f.read().records, []);
  assert.equal(f.probe.enabled, false);
  assert.equal(f.command('bogus').ok, false);
  assert.equal(f.probe.handle('send_prompt', {}, () => assert.fail('must not intercept send')), false);
});

test('explicit command observes same-origin calls without posting or duplicating traffic', async () => {
  const f = fixture();
  f.command('start');
  for (const url of ['/backend-api/files?credential=secret', '/ces/example',
    'https://example.com/backend-api/files', '/_next/chunk.js']) {
    const response = await f.window.fetch(url, { method: 'POST', body: '{"name":"secret"}' });
    assert.equal(await response.text(), '{"id":"secret"}');
  }
  await tick();
  assert.equal(f.requests.length, 4);
  assert.equal(f.read().records.length, 2);
  assert.equal(f.read().records[0].responseState, 'ready');
  assert.doesNotMatch(JSON.stringify(f.read()), /credential|secret|example\.com/);
  assert.deepEqual(f.events, []);
  f.command('stop');
});

test('network rejection is preserved and never automatically retried', async () => {
  const failure = new Error('original-failure');
  const f = fixture({ fetch: () => Promise.reject(failure) });
  f.command('start');
  await assert.rejects(f.window.fetch('/backend-api/files'), (error) => error === failure);
  assert.equal(f.requests.length, 1);
  assert.equal(f.read().records[0].status, 0);
  assert.equal(f.read().records[0].responseState, 'skipped');
  f.command('stop');
});

test('diagnostic exceptions cannot block a website request', async () => {
  const f = fixture({ window: { __elonChatGptPrivateProtocolEvidence: {
    create: () => ({ active: () => true, begin: () => { throw new Error('observer-only'); },
      response: () => { throw new Error('observer-only'); } }),
  } } });
  const response = await f.window.fetch('/backend-api/files');
  assert.equal(await response.text(), '{"id":"secret"}');
  assert.equal(f.requests.length, 1);
});

test('XHR is passive and stop rejects a late loadend callback', () => {
  const f = fixture();
  const dormant = new f.window.XMLHttpRequest();
  dormant.open('POST', '/backend-api/files'); dormant.send();
  assert.equal(dormant.listeners.loadend, undefined);
  f.command('start');
  const xhr = new f.window.XMLHttpRequest();
  xhr.open('POST', '/backend-api/files'); xhr.send('{"size":12}');
  assert.equal(xhr.sent, true);
  xhr.listeners.loadend();
  assert.equal(f.read().records[0].status, 201);
  assert.deepEqual(f.read().records[0].responseFields, ['$:object', '$.id:string']);
  const late = new f.window.XMLHttpRequest();
  late.open('POST', '/backend-api/files'); late.send();
  f.command('stop'); late.listeners.loadend();
  assert.equal(f.read().records[1].responseState, 'cancelled');
});

test('reinjection is idempotent and document/native bridge gates remain enforced', () => {
  const f = fixture();
  const wrapped = f.window.fetch;
  vm.runInNewContext(source, f.context);
  assert.equal(f.window.fetch, wrapped);
  assert.equal(fixture({ window: { __elonChatGptDocumentToken: 'invalid' } }).probe, undefined);
  assert.equal(fixture({ window: { elonChatGptNative: null } }).probe, undefined);
});

test('native diagnostic receipt is retained without triggering UI feedback or a DOM snapshot', () => {
  const background = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt'), 'utf8');
  assert.match(background, /observedMcpState\.accept\(event\)\s+if \(event is ChatGptWebEvent\.CommandResult && event\.action == "private_protocol_probe"\) return\s+when \(event\)/);
});
