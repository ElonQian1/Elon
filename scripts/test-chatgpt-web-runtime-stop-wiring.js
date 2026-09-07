'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const source = name => fs.readFileSync(path.join(assets, name + '.js'), 'utf8');
const flush = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };

function fixture(receipt = { status: 'accepted', code: 'stop_observed' }) {
  const events = [], calls = [];
  let fallback = 0, snapshots = 0, relayStopped = false, stopped = false;
  const page = {
    __elonChatGptPrivateTextTransactionRelay: { stop: () => relayStopped },
    __elonChatGptPrivateStreamTransport: { finishPrivateSend: () => { stopped = true; } },
    __elonChatGptPrivateStopRuntime: {
      stop: command => { calls.push(command); return {
        handled: true, completion: Promise.resolve(receipt), claimFallback: () => true
      }; },
      state: () => ({ pending: false })
    }
  };
  const composer = {};
  vm.runInNewContext(source('chatgpt_web_text_transaction_orchestrator'), { window: page });
  const api = page.__elonChatGptTextTransactionOrchestrator.create({
    findComposer: () => composer, composerValue: () => '', setComposerValue: () => true,
    comparableText: x => x, scheduleSnapshot: () => { snapshots++; }
  });
  const respond = (action, ok, detail) => events.push({ action, ok, detail });
  respond.requestId = 'mcp_stop';
  return { page, api, events, calls, composer, respond,
    run: () => api.stopGeneration(respond, () => { fallback++; }),
    counts: () => ({ fallback, snapshots, stopped }), useRelay: () => { relayStopped = true; } };
}

test('native stop uses the runtime transaction without a DOM stop button', async () => {
  const f = fixture(); f.run(); await flush();
  assert.equal(f.calls.length, 1);
  assert.equal(f.calls[0].composer, f.composer);
  assert.equal(f.calls[0].requestId, 'mcp_stop');
  assert.deepEqual(f.events, [{ action: 'stop_generation', ok: true, detail: 'official_runtime_v1:stop_observed' }]);
  assert.equal(f.counts().fallback, 0);
  assert.equal(f.counts().stopped, false, 'official runtime, not native state, owns completion');
});

test('the captured private sender keeps its own cancellation owner', async () => {
  const f = fixture(); f.useRelay(); f.run(); await flush();
  assert.equal(f.calls.length, 0);
  assert.equal(f.events[0].detail, 'private_text_v1:stopped');
  assert.equal(f.counts().stopped, true);
});

for (const status of ['unknown', 'rejected']) {
  test(status + ' stop cannot retry by clicking another button', async () => {
    const f = fixture({ status, code: 'context_changed' }); f.run(); await flush();
    assert.equal(f.events[0].ok, false);
    assert.equal(f.counts().fallback, 0);
    assert.equal(f.counts().stopped, false);
  });
}

test('only an unavailable pre-invocation runtime keeps the original stop fallback', async () => {
  const f = fixture({ status: 'unavailable', code: 'runtime_unavailable' }); f.run(); await flush();
  assert.equal(f.counts().fallback, 1);
  assert.equal(f.events.length, 0);
});

test('missing runtime keeps the existing stop route', () => {
  const f = fixture(); delete f.page.__elonChatGptPrivateStopRuntime; f.run();
  assert.equal(f.counts().fallback, 1);
});

test('a stale fallback ticket never clicks a stop button in the next conversation', async () => {
  const f = fixture();
  f.page.__elonChatGptPrivateStopRuntime.stop = () => ({ handled: true,
    completion: Promise.resolve({ status: 'unavailable' }), claimFallback: () => false });
  f.run(); await flush();
  assert.equal(f.counts().fallback, 0);
  assert.equal(f.events[0].ok, false);
});

test('pending stop blocks both send and regeneration before their fallback paths', () => {
  const f = fixture();
  f.page.__elonChatGptPrivateStopRuntime.state = () => ({ pending: true });
  f.api.sendPrompt('synthetic', '', f.respond, true);
  f.api.regenerateResponse(f.respond, () => { throw Error('must not regenerate while stopping'); });
  assert.deepEqual(f.events.map(e => e.ok), [false, false]);
  assert.ok(f.events.every(e => e.detail.endsWith('stop_pending')));
});

test('production bundle wires stopping after its shared conversation context', () => {
  const adapter = fs.readFileSync(path.join(assets, '../kotlin/com/elon/app/chatgptweb/ChatGptWebPageAdapter.kt'), 'utf8');
  const names = [...adapter.split('private val ADAPTER_ASSETS = listOf(')[1].split(')')[0].matchAll(/"([a-z0-9_]+)\.js"/g)].map(m => m[1]);
  const stop = names.indexOf('chatgpt_web_private_stop_runtime');
  assert.ok(stop > names.indexOf('chatgpt_web_private_text_runtime_submit'));
  assert.ok(stop < names.indexOf('chatgpt_web_text_transaction_orchestrator'));
  assert.match(source('chatgpt_web_adapter'), /textTransactionOrchestrator\.stopGeneration/);
  new vm.Script(names.map(source).join('\n'));
});
