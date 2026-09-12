'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const assets = path.join(__dirname, '../android/app/src/main/assets');
const source = fs.readFileSync(path.join(assets, 'chatgpt_web_text_transaction_orchestrator.js'), 'utf8');
const tick = () => new Promise(resolve => setImmediate(resolve));
function fixture(fresh) {
  const results = [], calls = [];
  const page = { __elonChatGptFreshTextTransaction: fresh,
    __elonChatGptPrivateTextRuntimeSubmit: { submit() {
      calls.push('official'); return { handled: true, completion: Promise.resolve({ status: 'accepted' }) };
    } } };
  vm.runInNewContext(source, { window: page });
  const api = page.__elonChatGptTextTransactionOrchestrator.create({
    findComposer: () => null, composerValue: () => '', setComposerValue: () => true,
    comparableText: value => value, scheduleSnapshot: () => calls.push('snapshot')
  });
  const respond = (...args) => results.push(args); respond.requestId = 'mcp_wiring';
  return { api, respond, results, calls };
}
test('eligible native send uses independent transaction without invoking official submit', async () => {
  const f = fixture({ state: () => ({ pending: false }), send(command) {
    assert.equal(command.prompt, 'fixture'); command.onDispatch();
    return { handled: true, completion: Promise.resolve({ status: 'accepted' }) };
  } });
  f.api.sendPrompt('fixture', '', f.respond, true); await tick();
  assert.equal(f.calls.includes('official'), false);
  assert.deepEqual(f.results[0], ['send_prompt', true, 'private_text_v1:accepted']);
});
test('disabled candidate preserves the accepted production text sender', async () => {
  const f = fixture({ state: () => ({ pending: false }), send: () => ({ handled: false }) });
  f.api.sendPrompt('fixture', '', f.respond, true); await tick();
  assert.equal(f.calls.filter(c => c === 'official').length, 1);
  assert.equal(f.results[0][1], true);
});
test('unknown independent write cannot fall through to official send', async () => {
  const f = fixture({ state: () => ({ pending: false }), send: () => ({ handled: true,
    completion: Promise.resolve({ status: 'unknown', code: 'timeout' }) }) });
  f.api.sendPrompt('fixture', '', f.respond, true); await tick();
  assert.equal(f.calls.includes('official'), false); assert.equal(f.results[0][1], false);
});

test('pre-dispatch rejection does not leave the native ledger in indeterminate delivery', async () => {
  const f = fixture({ state: () => ({ pending: false }), send: () => ({ handled: true,
    completion: Promise.resolve({ status: 'rejected', code: 'context_changed' }) }) });
  f.api.sendPrompt('fixture', '', f.respond, true); await tick();
  assert.equal(f.calls.includes('official'), false);
  assert.deepEqual(f.results[0], ['send_prompt', false, 'official_runtime_v1:rejected:not_ready']);
});
test('unavailable before dispatch needs a single-use fallback claim', async () => {
  for (const allowed of [false, true]) {
    let count = 0;
    const f = fixture({ state: () => ({ pending: false }), send: () => ({ handled: true,
      completion: Promise.resolve({ status: 'unavailable' }), claimFallback: () => { count++; return allowed; } }) });
    f.api.sendPrompt('fixture', '', f.respond, true); await tick();
    assert.equal(count, 1); assert.equal(f.calls.includes('official'), allowed);
    if (!allowed) assert.equal(f.results[0][2], 'official_runtime_v1:rejected:not_ready');
  }
});
test('an unreconciled private write blocks attachment and regeneration callbacks', () => {
  const f = fixture({ state: () => ({ pending: true }) });
  f.api.sendPrompt('fixture', '', f.respond, false);
  f.api.regenerateResponse(f.respond, () => { throw Error('must not fall back'); });
  assert.equal(f.calls.includes('official'), false); assert.equal(f.results.length, 2);
});
test('cancelling a private HTTP reader does not falsely report confirmed server stop', () => {
  let cancelled = 0;
  const f = fixture({ state: () => ({ pending: true }), cancel: () => { cancelled++; } });
  f.api.stopGeneration(f.respond, () => { throw Error('must not fall back'); });
  assert.equal(cancelled, 1);
  assert.deepEqual(f.results[0], ['stop_generation', false, 'private_text_v1:unknown:server_stop_unconfirmed']);
});
test('production asset assembly loads dependencies before the one existing send router', () => {
  const source = require('./chatgpt-web-adapter-assembly').readAdapterSource();
  const names = [...source.split('private val ADAPTER_ASSETS = listOf(')[1].split(')')[0]
    .matchAll(/"([a-z0-9_]+\.js)"/g)].map(m => m[1]);
  const chain = ['chatgpt_web_private_runtime_bindings.js', 'chatgpt_web_private_text_runtime_submit.js',
    'chatgpt_web_fresh_text_request.js', 'chatgpt_web_fresh_text_context.js',
    'chatgpt_web_fresh_text_reconcile.js',
    'chatgpt_web_fresh_text_transaction.js', 'chatgpt_web_text_transaction_orchestrator.js'];
  for (let i = 0; i < chain.length; i++) {
    assert.equal(names.filter(n => n === chain[i]).length, 1);
    if (i) assert.ok(names.indexOf(chain[i]) > names.indexOf(chain[i - 1]));
  }
});
