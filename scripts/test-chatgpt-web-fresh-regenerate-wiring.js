'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const source = fs.readFileSync(path.join(__dirname, '../android/app/src/main/assets/chatgpt_web_text_transaction_orchestrator.js'), 'utf8');
const tick = () => new Promise(resolve => setImmediate(resolve));

function fixture(result) {
  const calls = [], receipts = [], composer = {}, turn = {}, model = {};
  const page = { __elonChatGptComposer: { modelTrigger: node => { assert.equal(node, composer); return model; } },
    __elonChatGptFreshTextTransaction: { state: () => ({ pending: false }), regenerate(command) {
      assert.equal(command.composer, composer); assert.equal(command.turn, turn);
      assert.equal(command.getModelTrigger(), model); assert.equal(command.expectedDraft, 'Synthetic unsent draft');
      assert.equal(command.readDraft(), 'Synthetic unsent draft');
      assert.equal(command.clearDraft, undefined); calls.push('fresh');
      if (result?.dispatched) command.onDispatch();
      return result;
    } },
    __elonChatGptPrivateRegenerateRuntime: { regenerate() {
      calls.push('runtime'); return { handled: true, completion: Promise.resolve({ status: 'accepted' }) };
    } } };
  vm.runInNewContext(source, { window: page });
  const api = page.__elonChatGptTextTransactionOrchestrator.create({
    findComposer: () => composer, composerValue: () => 'Synthetic unsent draft',
    setComposerValue: () => { throw Error('do not edit the draft'); }, comparableText: value => value,
    messageAdapter: { lastAssistantTurn: () => turn }, scheduleSnapshot() {},
    streamingPolicy: {}, streamingPolicyModule: { begin() { calls.push('begin'); } }
  });
  const respond = (...value) => receipts.push(value); respond.requestId = 'mcp_route1';
  return { calls, receipts, run: () => api.regenerateResponse(respond, () => calls.push('dom')) };
}

test('native regenerate button routes to the single fresh ledger and retains its existing receipt', async () => {
  const f = fixture({ handled: true, dispatched: true, completion: Promise.resolve({ status: 'accepted' }) });
  f.run(); await tick();
  assert.deepEqual(f.calls, ['fresh', 'begin']);
  assert.deepEqual(f.receipts, [['regenerate_response', true, 'private_text_v1:regenerate_accepted']]);
});

test('disabled candidate retains the already accepted runtime regeneration', async () => {
  const f = fixture({ handled: false }); f.run(); await tick();
  assert.deepEqual(f.calls, ['fresh', 'runtime']);
  assert.equal(f.receipts[0][2], 'official_runtime_v1:regenerate_observed');
});

test('only an owned pre-dispatch compatibility claim can fall back', async () => {
  for (const allowed of [true, false]) {
    let claims = 0;
    const f = fixture({ handled: true, completion: Promise.resolve({ status: 'unavailable', code: 'context_unavailable' }),
      claimFallback() { claims++; return allowed; } });
    f.run(); await tick();
    assert.equal(claims, 1); assert.equal(f.calls.includes('runtime'), allowed);
    assert.equal(f.calls.includes('dom'), false); assert.equal(f.receipts[0][1], allowed);
  }
});

test('post-dispatch uncertainty or completion failure cannot call another writer', async () => {
  for (const completion of [() => Promise.resolve({ status: 'unknown', code: 'stream_timeout' }),
    () => Promise.reject(Error('synthetic completion error'))]) {
    const f = fixture({ handled: true, dispatched: true, completion: completion(),
      claimFallback() { throw Error('cannot claim after a write'); } });
    f.run(); await tick();
    assert.deepEqual(f.calls, ['fresh', 'begin']); assert.equal(f.receipts[0][1], false);
    assert.match(f.receipts[0][2], /^private_text_v1:regenerate_unknown:/);
  }
});

test('production assembly loads the regeneration scope before the shared ledger', () => {
  const adapter = require('./chatgpt-web-adapter-assembly').readAdapterSource();
  const names = [...adapter.split('private val ADAPTER_ASSETS = listOf(')[1].split(')')[0]
    .matchAll(/"([a-z0-9_]+\.js)"/g)].map(match => match[1]);
  const scope = names.indexOf('chatgpt_web_fresh_regenerate_context.js');
  assert.equal(names.filter(name => name === 'chatgpt_web_fresh_regenerate_context.js').length, 1);
  assert.ok(scope > names.indexOf('chatgpt_web_fresh_text_context.js'));
  assert.ok(scope > names.indexOf('chatgpt_web_private_regenerate_contract.js'));
  assert.ok(scope < names.indexOf('chatgpt_web_fresh_text_transaction.js'));
  const page = { __elonChatGptTextTransactionOrchestrator: { version: 10 } };
  vm.runInNewContext(source, { window: page });
  assert.equal(page.__elonChatGptTextTransactionOrchestrator.version, 11);
});
