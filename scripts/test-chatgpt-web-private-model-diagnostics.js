'use strict';
const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture } = require('./fixtures/chatgpt-model-state');
const runtime = require('../android/app/src/main/assets/chatgpt_web_private_model_state.js');
const fs = require('node:fs'), vm = require('node:vm');
const flush = async () => { for (let i = 0; i < 30; i++) await Promise.resolve(); };

test('reinjecting the model module preserves the existing composer diagnostic owner', async () => {
  const f = fixture();
  const source = fs.readFileSync(require.resolve('../android/app/src/main/assets/chatgpt_web_private_model_state.js'), 'utf8');
  f.page.__elonChatGptPrivateModelContract = require('../android/app/src/main/assets/chatgpt_web_private_model_contract');
  const context = { window: f.page };
  vm.runInNewContext(source, context);
  const api = f.page.__elonChatGptPrivateModelState;
  const instance = api.create(f.page, { loadRuntime: async url =>
    f.modules[Object.keys(f.contract.urls).find(key => f.contract.urls[key] === url)] });
  instance.request(f.getTrigger, () => {}, () => {}, () => {}); await flush();
  assert.equal(api.state(f.page), 'ready');
  vm.runInNewContext(source, context);
  assert.equal(f.page.__elonChatGptPrivateModelState, api);
  assert.equal(f.page.__elonChatGptPrivateModelState.state(f.page), 'ready');
  assert.deepEqual(f.writes, []);
});

test('model diagnostics are passive, document-bound and never expose context values', async () => {
  const f = fixture();
  assert.equal(runtime.state(f.page), 'not_observed');
  f.request(); await flush();
  assert.equal(runtime.state(f.page), 'ready');
  const imports = f.imports;
  assert.equal(runtime.state(f.page), 'ready');
  assert.equal(f.imports, imports);
  assert.deepEqual(f.writes, []);
  f.page.__elonChatGptDocumentToken = 'doc_replaced';
  assert.equal(runtime.state(f.page), 'not_observed');
});

for (const [code, change] of Object.entries({
  identity_unavailable: f => { f.account = ''; },
  trigger_detached: f => { f.node.isConnected = false; },
  owner_unavailable: f => { f.top.stateNode.current = {}; },
  picker_missing: f => { delete f.props.dropdownContent; },
  picker_disabled: f => { f.props.ariaDisabled = true; },
  conversation_mismatch: f => { f.page.location.href = 'https://chatgpt.com/'; },
  route_unsupported: f => { f.page.location.href += '?secret=not-exported'; },
  menu_open: f => { f.props.dropdownOpen = true; }
})) test('model admission distinguishes ' + code + ' without changing fallback', () => {
  const f = fixture(); change(f);
  assert.equal(f.request(), false);
  assert.equal(runtime.state(f.page), code);
  assert.equal(f.imports, 0);
  assert.deepEqual(f.writes, []);
});

test('unknown module exports report a contract failure, not missing provider capability', async () => {
  const f = fixture(); delete f.modules.shared.RW;
  f.request(); await flush();
  assert.equal(runtime.state(f.page), 'runtime_unknown');
  assert.equal(f.fallback, 1);
  assert.deepEqual(f.writes, []);
});
