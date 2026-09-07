'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const vm = require('node:vm');
const path = require('node:path');
const { fixture, flush, id } = require('./fixtures/chatgpt-runtime-regeneration.js');
const assetRoot = path.join(__dirname, '../android/app/src/main/assets');
const source = name => fs.readFileSync(path.join(assetRoot, name + '.js'), 'utf8');

function production(options) {
  const f = fixture(options), events = [];
  let fallback = 0, relay = 0, begin = 0;
  f.page.__elonChatGptComposer = { modelTrigger: () => f.modelButton };
  f.page.__elonChatGptPrivateTextTransactionRelay = {
    dispatchRegenerate() { relay++; return { dispatched: false }; }, state: () => ({ active: false })
  };
  f.page.__elonChatGptPrivateStreamTransport.preparePrivateRegeneration = () => true;
  vm.runInNewContext(source('chatgpt_web_text_transaction_orchestrator'), { window: f.page });
  const api = f.page.__elonChatGptTextTransactionOrchestrator.create({
    findComposer: () => ({}), composerValue: () => 'untouched synthetic draft',
    setComposerValue() { throw Error('regeneration must never replace a draft'); }, comparableText: x => x,
    scheduleSnapshot() {}, messageAdapter: { lastAssistantTurn: () => f.turn },
    streamingPolicy: {}, streamingPolicyModule: { begin: () => { begin++; } }
  });
  const respond = (action, ok, detail) => events.push({ action, ok, detail }); respond.requestId = 'mcp_retry';
  return { ...f, events, run: () => api.regenerateResponse(respond, () => { fallback++; }),
    counts: () => ({ fallback, relay, begin }) };
}

test('the actual orchestrator routes regeneration to runtime and preserves the existing draft', async () => {
  const f = production(); f.run(); await flush(); f.publish(); await flush();
  assert.equal(f.calls.length, 1);
  assert.deepEqual(f.counts(), { fallback: 0, relay: 0, begin: 1 });
  assert.deepEqual(f.events, [{ action: 'regenerate_response', ok: true, detail: 'official_runtime_v1:regenerate_observed' }]);
});

test('uncertain regeneration cannot replay through a private template or an official menu', async () => {
  const f = production(); f.run(); await flush(); f.runTimer(15000); await flush();
  assert.equal(f.events[0].ok, false); assert.match(f.events[0].detail, /regenerate_unknown:timeout$/);
  f.run(); await flush();
  assert.match(f.events[1].detail, /regenerate_unknown:busy$/);
  assert.equal(f.calls.length, 1);
  assert.deepEqual(f.counts(), { fallback: 0, relay: 0, begin: 1 });
  assert.equal(f.prepared(), 1);
  f.publish();
});

test('a missing module before invocation keeps the original fallback with the same target', async () => {
  const f = production({ loadRuntime: () => Promise.reject(Error('synthetic unavailable')) });
  f.run(); await flush();
  assert.equal(f.calls.length, 0);
  assert.deepEqual(f.counts(), { fallback: 1, relay: 1, begin: 1 });
});

test('changed conversation during preparation does not invoke fallback in the new conversation', async () => {
  const f = production(); f.run();
  f.page.location.href = 'https://chatgpt.com/c/' + id(9); await flush();
  assert.equal(f.calls.length, 0); assert.equal(f.counts().fallback, 0);
  assert.match(f.events[0].detail, /regenerate_rejected:context_changed$/);
});

test('the production adapter delegates the same native regeneration command without preemptive stream clearing', () => {
  const adapter = source('chatgpt_web_adapter');
  const branch = adapter.split("if (action === 'regenerate_response' && messageAdapter) {")[1].split("if (action === 'stop_generation')")[0];
  assert.match(branch, /textTransactionOrchestrator\.regenerateResponse/);
  assert.doesNotMatch(branch, /prepareSend|streamingPolicyModule\.begin/);
  assert.match(source('chatgpt_web_adapter_messages'), /lastAssistantObservation,\s+lastAssistantTurn,/);
  assert.match(source('chatgpt_web_adapter_composer'), /modelTrigger: findModelButton/);
});

test('production assets include the versioned contracts before the command orchestrator', () => {
  const adapter = fs.readFileSync(path.join(assetRoot, '../kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  const names = [...adapter.split('private val ADAPTER_ASSETS = listOf(')[1].split(')')[0].matchAll(/"([a-z0-9_]+)\.js"/g)].map(m => m[1]);
  for (const name of ['chatgpt_web_private_regenerate_contract', 'chatgpt_web_private_regenerate_runtime']) {
    assert.ok(names.indexOf(name) >= 0 && names.indexOf(name) < names.indexOf('chatgpt_web_text_transaction_orchestrator'));
    assert.doesNotMatch(source(name), /\bfetch\s*\(|\.click\s*\(|document\.cookie|localStorage|setInterval/);
  }
  new vm.Script(names.map(source).join('\n'));
});
