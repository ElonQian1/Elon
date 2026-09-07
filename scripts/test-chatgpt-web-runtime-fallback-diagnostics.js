'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname,
  '../android/app/src/main/assets/chatgpt_web_text_transaction_orchestrator.js'), 'utf8');

function fixture(code, includeRuntime = true) {
  let time = 1, draft = '', clicks = 0;
  const timers = [], events = [];
  const composer = { closest: () => null };
  const window = {
    __elonChatGptPrivateTextRuntimeSubmit: { version: 7, submit: () => ({ handled: false, code }) },
    __elonChatGptPrivateTextTransactionRelay: { dispatch: () => ({ dispatched: false, code: 'template_unavailable' }) },
    __elonChatGptPrivateStreamTransport: { preparePrivateSend() {}, prepareSend() {} },
    setTimeout(callback, ms) { timers.push(() => { time += ms; callback(); }); },
  };
  if (!includeRuntime) delete window.__elonChatGptPrivateTextRuntimeSubmit;
  vm.runInNewContext(source, { window, Date: { now: () => time } });
  const api = window.__elonChatGptTextTransactionOrchestrator.create({
    findComposer: () => composer, composerValue: () => draft,
    setComposerValue: (_, value) => { draft = value; return true; },
    comparableText: value => value, scheduleSnapshot() {}, isVisible: node => !!node,
    findButton: () => button, readStreamingState: () => ({ active: false }),
  });
  const button = { disabled: false, getAttribute: () => '', click() { clicks++; draft = ''; } };
  const respond = (action, ok, detail) => events.push({ action, ok, detail }); respond.requestId = 'mcp_test';
  return { api, events, respond, window, clicks: () => clicks, run() {
    api.sendPrompt('synthetic', '', respond, true);
    for (let n = 0; timers.length && n < 100; n++) timers.shift()();
    assert.equal(timers.length, 0);
  } };
}

test('one DOM fallback receipt retains both pre-dispatch gate failures', () => {
  const f = fixture('react_owner_unavailable'); f.run();
  assert.equal(f.clicks(), 1); assert.equal(f.events.length, 1);
  assert.equal(f.events[0].ok, true);
  assert.match(f.events[0].detail, /\[private_fallback:template_unavailable\]/);
  assert.match(f.events[0].detail, /\[runtime_fallback:react_owner_unavailable\]/);
  assert.doesNotMatch(f.events[0].detail, /synthetic|mcp_test/);
});

test('legacy runtime absence does not invent an admission result', () => {
  const f = fixture('', false); f.run();
  assert.equal(f.events[0].ok, true);
  assert.doesNotMatch(f.events[0].detail, /runtime_fallback/);
  assert.match(f.events[0].detail, /\[private_fallback:template_unavailable\]/);
});

test('diagnostic codes are bounded, not arbitrary exception or request content', () => {
  const f = fixture('CONTEXT: ' + 'x'.repeat(200) + ' https://example.invalid'); f.run();
  const code = /\[runtime_fallback:([^\]]+)\]/.exec(f.events[0].detail)?.[1];
  assert.equal(code, 'x'.repeat(32));
});
