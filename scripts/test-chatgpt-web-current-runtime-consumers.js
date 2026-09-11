'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { attach } = require('./fixtures/chatgpt-runtime-bindings');
const assets = '../android/app/src/main/assets/';
const flush = async () => { for (let i = 0; i < 70; i++) await Promise.resolve(); };
const profiles = [undefined, require('./fixtures/chatgpt-runtime-bindings-sep10b'),
  require('./fixtures/chatgpt-runtime-bindings-sep11b')];

for (const profile of profiles) test.describe(profile?.id || 'web_20260907', () => {

test('real sharing owner uses the current export map and its selected branch', async () => {
  const { fixture, PATH, LINK, NODE } = require('./fixtures/chatgpt-web-private-conversation-share.cjs');
  const f = fixture('toast'), bridge = attach(f.page, f.modules, profile);
  const api = require(assets + 'chatgpt_web_private_conversation_share').create(f.page, {
    contract: require(assets + 'chatgpt_web_private_conversation_share_contract')
  });
  const result = await api.start(PATH, true, () => f.snapshot);
  assert.equal(result.url, LINK); assert.equal(result.ok, true);
  assert.equal(f.requests.length, 1); assert.equal(f.requests[0].url, '/backend-api/share/v2/create');
  assert.equal(JSON.parse(f.requests[0].init.body).current_node_id, NODE);
  await api.start(PATH, true, () => f.snapshot);
  assert.equal(f.requests.length, 1); assert.equal(bridge.loads.length, 2);
});

test('unknown sharing namespace never dispatches a write', async () => {
  const { fixture, PATH } = require('./fixtures/chatgpt-web-private-conversation-share.cjs');
  const f = fixture('toast'); delete f.modules.shared.H3; attach(f.page, f.modules, profile);
  const api = require(assets + 'chatgpt_web_private_conversation_share').create(f.page, {
    contract: require(assets + 'chatgpt_web_private_conversation_share_contract')
  });
  assert.equal((await api.start(PATH, true, () => f.snapshot)).attempted, false);
  assert.equal(f.requests.length, 0);
});

test('production model owner reads and changes the current stores without extra imports', async () => {
  const f = require('./fixtures/chatgpt-model-state').fixture();
  const bridge = attach(f.page, f.modules, profile), catalogs = [], results = [];
  const api = require(assets + 'chatgpt_web_private_model_state').create(f.page, {
    contract: require(assets + 'chatgpt_web_private_model_contract')
  });
  const result = (...args) => results.push(args);
  assert.equal(api.request(f.getTrigger, value => catalogs.push(value), result,
    () => { throw Error('unexpected fallback'); }), true);
  await flush();
  assert.equal(results.at(-1)[1], true);
  api.select(catalogs.at(-1).find(item => item.label === 'High').id, result, () => {}, () => {});
  assert.equal(results.at(-1)[1], true); assert.equal(f.model, 'fixture-thinking'); assert.equal(f.effort, 'high');
  assert.equal(f.writes.filter(value => value[0] === 'official-model-action').length, 1);
  api.request(f.getTrigger, () => {}, result, () => {}); await flush();
  assert.equal(bridge.loads.length, 3);
});

test('current stop function preserves the existing private request owner and completion', { timeout: 2000 }, async () => {
  const f = require('./fixtures/chatgpt-runtime-stop').fixture();
  const bridge = attach(f.page, f.modules, profile);
  const api = require(assets + 'chatgpt_web_private_stop_runtime').create(f.page);
  const transaction = api.stop(f.command); assert.equal(transaction.handled, true);
  await flush(); assert.equal(f.calls.length, 1);
  f.setState(false, null); f.settle(); await flush();
  assert.equal((await transaction.completion).status, 'accepted');
  assert.equal(bridge.loads.length, 2);
});
});
