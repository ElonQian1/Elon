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

test('document diagnostics upgrade only the command surface without network capture', () => {
  const f = fixture({ window: { document: { readyState: 'loading', visibilityState: 'hidden',
    hasFocus: () => false, querySelectorAll: () => [] } } });
  const fetch = f.window.fetch;
  for (const previous of [null, 18]) {
    if (previous) {
      f.window.__elonChatGptPrivateResearchProbe = { ...f.probe, version: previous };
      vm.runInNewContext(source, f.context);
    }
    let value;
    f.window.__elonChatGptPrivateResearchProbe.handle('private_protocol_probe', { value: 'document_state' },
      (_, ok, detail) => { assert.equal(ok, true); value = JSON.parse(detail); });
    assert.equal(value.schema, 'elon.document_state.v1');
    assert.equal(value.ready, 'loading');
    assert.equal(value.prompt_count, 0);
    assert.equal(f.window.fetch, fetch);
    assert.equal(f.requests.length, 0);
    assert.equal(f.read().active, false);
  }
});

test('directory diagnostics use the existing read-only probe without network capture', () => {
  const f = fixture();
  assert.deepEqual(JSON.parse(f.command('directory_refresh').detail), {
    schema: 'elon.directory_refresh.v1', observed: false, durationMs: 0, identityMs: 0, reads: [],
  });
  const value = { schema: 'elon.directory_refresh.v1', observed: true, durationMs: 100, identityMs: 10, reads: [] };
  f.window.__elonChatGptPrivateConversationDirectory = { refreshDiagnostics: () => value };
  assert.deepEqual(JSON.parse(f.command('directory_refresh').detail), value);
  assert.equal(f.requests.length, 0);
  assert.equal(f.read().active, false);
  f.window.__elonChatGptPrivateResearchProbe = { ...f.probe, version: 16 };
  vm.runInNewContext(source, f.context);
  let upgraded;
  f.window.__elonChatGptPrivateResearchProbe.handle('private_protocol_probe', { value: 'directory_refresh' },
    (_, ok, detail) => { assert.equal(ok, true); upgraded = JSON.parse(detail); });
  assert.deepEqual(upgraded, value);
});

test('download source diagnostics are on demand and survive observer upgrade', () => {
  const f = fixture();
  assert.equal(JSON.parse(f.command('file_download_source').detail).observed, false);
  const value = { schema: 'elon.download_source.v1', observed: true, origin: 'same_origin',
    path: '/backend-api/estuary/content', relative: true, whitespace: false, credentials: false, port: false, fragment: false };
  f.window.__elonChatGptPrivateFileDownload = { sourceDiagnostics: () => value };
  for (const previous of [null, 20]) {
    if (previous) {
      f.window.__elonChatGptPrivateResearchProbe = { ...f.probe, version: previous };
      vm.runInNewContext(source, f.context);
    }
    let detail;
    f.window.__elonChatGptPrivateResearchProbe.handle('private_protocol_probe', { value: 'file_download_source' },
      (_, ok, result) => { assert.equal(ok, true); detail = JSON.parse(result); });
    assert.deepEqual(detail, value);
  }
  assert.equal(f.requests.length, 0); assert.equal(f.read().active, false);
});

test('native gate admits the stop owner mode used by the page probe', () => {
  const native = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPrivateProtocolEvidence.kt'), 'utf8');
  const modes = native.match(/val MODES = setOf\(([\s\S]*?)\)/)?.[1] || '';
  assert.match(modes, /"stop_runtime_owner"/);
  assert.match(native, /"elon\.stop_runtime_owner\.v1"/);
});

test('runtime asset inventory is on demand and does not enable capture or extra requests', () => {
  const f = fixture({ window: { performance: { getEntriesByType: () => [
    { name: 'https://chatgpt.com/cdn/assets/shared-abc123.js' },
  ] } } });
  const result = f.command('runtime_assets');
  assert.equal(result.ok, true);
  assert.deepEqual(JSON.parse(result.detail).assets, ['shared-abc123.js']);
  assert.equal(f.read().active, false);
  assert.deepEqual(f.events, []);
  assert.deepEqual(f.requests, []);
});

test('model diagnostics use the same closed vocabulary in page and native receiver', () => {
  const native = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPrivateProtocolEvidence.kt'), 'utf8');
  const codes = [...native.match(/modelContextCodes = setOf\(([\s\S]*?)\)/)[1].matchAll(/"([a-z_]+)"/g)].map(m => m[1]);
  for (const previous of [null, 17]) {
    const f = fixture({ window: { __elonChatGptPrivateModelState: { state: () => 'ready' } } });
    const fetch = f.window.fetch;
    if (previous) {
      f.window.__elonChatGptPrivateResearchProbe = { ...f.probe, version: previous };
      vm.runInNewContext(source, f.context);
    }
    const read = () => {
      let result;
      f.window.__elonChatGptPrivateResearchProbe.handle('private_protocol_probe', { value: 'model_runtime_context' },
        (_, ok, detail) => { assert.equal(ok, true); result = detail; });
      return result;
    };
    for (const code of codes) {
      f.window.__elonChatGptPrivateModelState.state = () => code;
      assert.equal(read(), 'model_runtime_context:' + code);
    }
    f.window.__elonChatGptPrivateModelState.state = () => 'secret raw context';
    assert.equal(read(), 'model_runtime_context:not_observed');
    f.window.__elonChatGptPrivateModelState.state = () => { throw Error('secret'); };
    assert.equal(read(), 'model_runtime_context:not_observed');
    assert.equal(f.window.fetch, fetch);
    assert.deepEqual(f.events, []); assert.deepEqual(f.requests, []);
    assert.equal(f.read().active, false);
  }
  assert.match(native.match(/val MODES = setOf\(([\s\S]*?)\)/)[1], /"model_runtime_context"/);
});

test('tool context diagnostics expose only known codes without running a capture', () => {
  const f = fixture({ window: { __elonChatGptPrivateComposerToolContext: {
    state: () => 'model_unavailable', capture: () => assert.fail('read must not capture'),
  } } });
  assert.equal(f.command('composer_tool_context').detail, 'composer_tool_context:model_unavailable');
  f.window.__elonChatGptPrivateComposerToolContext.state = () => 'private raw value';
  assert.equal(f.command('composer_tool_context').detail, 'composer_tool_context:not_observed');
  assert.equal(f.read().active, false);
  assert.deepEqual(f.events, []);
  assert.deepEqual(f.requests, []);
});

test('library policy diagnostics survive reinjection without validation, network or raw values', () => {
  const native = fs.readFileSync(path.join(__dirname,
    '../android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebPrivateProtocolEvidence.kt'), 'utf8');
  const codes = [...native.match(/libraryPolicyCodes = setOf\(([\s\S]*?)\)/)[1].matchAll(/"([a-z_]+)"/g)].map(m => m[1]);
  assert.match(native.match(/val MODES = setOf\(([\s\S]*?)\)/)[1], /"library_attachment_policy"/);
  for (const previous of [null, 19]) {
    const f = fixture(), fetch = f.window.fetch;
    const policy = { state: () => 'ready', prepare: () => assert.fail('read cannot validate or mutate') };
    f.window.__elonChatGptPrivateLibraryAttachmentPolicy = policy;
    if (previous) {
      f.window.__elonChatGptPrivateResearchProbe = { ...f.probe, version: previous };
      vm.runInNewContext(source, f.context);
    }
    const read = () => {
      let value;
      f.window.__elonChatGptPrivateResearchProbe.handle('private_protocol_probe', { value: 'library_attachment_policy' },
        (_, ok, detail) => { assert.equal(ok, true); value = detail; });
      return value;
    };
    for (const code of codes) { policy.state = () => code; assert.equal(read(), 'library_attachment_policy:' + code); }
    policy.state = () => 'secret raw context';
    assert.equal(read(), 'library_attachment_policy:not_observed');
    policy.state = () => { throw Error('secret'); };
    assert.equal(read(), 'library_attachment_policy:not_observed');
    assert.equal(f.window.fetch, fetch);
    assert.deepEqual(f.requests, []); assert.deepEqual(f.events, []);
    assert.equal(f.read().active, false);
  }
});

test('version 13 command upgrade preserves existing observers and other commands', () => {
  const f = fixture();
  const fetch = f.window.fetch, send = f.window.XMLHttpRequest.prototype.send;
  const existing = f.probe;
  f.window.__elonChatGptPrivateResearchProbe = { ...existing, version: 13 };
  vm.runInNewContext(source, f.context);
  assert.equal(f.window.__elonChatGptPrivateResearchProbe.version, 21);
  assert.equal(f.window.fetch, fetch);
  assert.equal(f.window.XMLHttpRequest.prototype.send, send);
  const answers = [];
  f.window.__elonChatGptPrivateResearchProbe.handle('private_protocol_probe', { value: 'composer_tool_context' }, (...args) => answers.push(args));
  assert.equal(answers[0][2], 'composer_tool_context:not_observed');
  f.window.__elonChatGptPrivateResearchProbe.handle('private_protocol_probe', { value: 'read' }, (...args) => answers.push(args));
  assert.equal(JSON.parse(answers[1][2]).active, false);
});

for (const previousVersion of [null, 15]) test('stop owner shape is read-only; previous=' + previousVersion, () => {
  const f = fixture({ window: { __elonChatGptPrivateStopRuntime: {
    diagnostics: () => ({ cached: true, request: 'missing', tree: true,
      generation: true, mode: 'streaming', token: 'secret', text: 'private text' }),
    stop: () => assert.fail('diagnostics must not stop a request'),
  } } });
  const fetch = f.window.fetch, send = f.window.XMLHttpRequest.prototype.send;
  if (previousVersion) {
    f.window.__elonChatGptPrivateResearchProbe = { ...f.probe, version: previousVersion };
    vm.runInNewContext(source, f.context);
  }
  let result;
  f.window.__elonChatGptPrivateResearchProbe.handle('private_protocol_probe', { value: 'stop_runtime_owner' },
    (_, ok, detail) => { assert.equal(ok, true); result = JSON.parse(detail); });
  assert.deepEqual(result, { schema: 'elon.stop_runtime_owner.v1', cached: true,
    request: 'missing', tree: true, generation: true, mode: 'streaming' });
  assert.equal(f.window.fetch, fetch); assert.equal(f.window.XMLHttpRequest.prototype.send, send);
  assert.equal(f.read().active, false); assert.deepEqual(f.events, []); assert.deepEqual(f.requests, []);
});

test('stop owner diagnostics reject raw values and tolerate unavailable runtime', () => {
  const f = fixture({ window: { __elonChatGptPrivateStopRuntime: {
    diagnostics: () => ({ cached: 'secret', request: 'secret', tree: 'secret',
      generation: 'secret', mode: 'secret', requestId: 'secret' })
  } } });
  const expected = { schema: 'elon.stop_runtime_owner.v1', cached: false,
    request: 'missing', tree: false, generation: false, mode: 'unknown' };
  assert.deepEqual(JSON.parse(f.command('stop_runtime_owner').detail), expected);
  f.window.__elonChatGptPrivateStopRuntime.diagnostics = () => { throw Error('secret'); };
  assert.deepEqual(JSON.parse(f.command('stop_runtime_owner').detail), expected);
});

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
  const start = background.indexOf('private fun handleEvent(event: ChatGptWebEvent)');
  const dispatch = background.indexOf('when (event)', start);
  const receipt = background.indexOf('observedMcpState.accept(event)', start);
  const earlyReturn = background.indexOf('if (event is ChatGptWebEvent.CommandResult && event.action == "private_protocol_probe") return', start);
  assert.ok(start >= 0 && receipt > start && earlyReturn > receipt && dispatch > earlyReturn);
  assert.doesNotMatch(background.slice(start, earlyReturn), /requestSnapshot|onCommandResult/);
});
