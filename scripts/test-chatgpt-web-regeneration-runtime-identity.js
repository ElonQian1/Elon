'use strict';

const assert = require('node:assert/strict');
const test = require('node:test');
const { fixture, flush, id } = require('./fixtures/chatgpt-runtime-regeneration.js');
const models = require('../android/app/src/main/assets/chatgpt_web_private_model_contract.js');

test('non-runtime model owners retain the existing device-header guard', () => {
  const f = fixture(), headers = { authorization: 'Bearer synthetic-only-identity', 'oai-device-id': 'device-a' };
  f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => headers;
  const contract = models.create(f.page), binding = contract.capture(f.command.getModelTrigger);
  headers['oai-device-id'] = 'device-b';
  assert.equal(contract.ownerState(binding), 'owner_device');
});

for (const [name, headers] of Object.entries({
  warmed_headers_gain_account_and_device: { authorization: 'Bearer synthetic-only-identity',
    'chatgpt-account-id': id(7), 'oai-device-id': 'synthetic-device' },
  incidental_request_uses_another_account: { authorization: 'Bearer synthetic-only-identity',
    'chatgpt-account-id': id(99), 'oai-device-id': 'another-synthetic-device' },
  token_refresh: { authorization: 'Bearer refreshed-synthetic-credential' },
  incidental_request_headers_unavailable: null
})) {
  test('official reply identity is independent of transport header cache: ' + name, async () => {
    const f = fixture(), result = f.api.regenerate(f.command);
    await flush();
    f.page.__elonChatGptPrivateTransport.copySameOriginRequestHeaders = () => headers;
    f.publish({ state: 'completed' });
    f.runTimer(15000);
    assert.deepEqual(await result.completion, { status: 'accepted', code: 'regenerate_observed' });
    assert.equal(f.calls.length, 1);
    assert.equal(f.api.state().pending, false);
    assert.equal(f.listeners.size, 0);
  });
}

for (const [reason, change] of Object.entries({
  owner_auth: f => { f.identity.userId = 'another-synthetic-user'; },
  owner_account: f => { f.identity.accountId = id(99); },
  owner_identity: f => { f.identity.loggedIn = false; }
})) {
  test('unchanged copied headers cannot hide a real runtime identity change: ' + reason, async () => {
    const f = fixture(), result = f.api.regenerate(f.command);
    await flush(); change(f); f.publish({ state: 'completed' }); f.runTimer(15000);
    assert.deepEqual(await result.completion, { status: 'unknown', code: 'timeout_' + reason });
    assert.equal(f.api.state().pending, true);
    assert.equal(f.calls.length, 1);
    assert.equal((await f.api.regenerate(f.command).completion).code, 'busy');
  });
}

for (const [name, change] of Object.entries({
  getter_missing: f => { delete f.modules.shared.F5; },
  logged_out: f => { f.identity.loggedIn = false; },
  missing_user: f => { f.identity.userId = ''; },
  missing_account: f => { f.identity.accountId = ''; },
  inconsistent_account: f => { f.modules.shared.mq = () => ({ id: id(99), authUserId: f.identity.userId }); },
  inconsistent_user: f => { f.modules.shared.mq = () => ({ id: f.identity.accountId, authUserId: 'other' }); },
  getter_throws: f => { f.modules.shared.F5 = () => { throw Error('synthetic-private-data'); }; },
  account_changes_during_preparation: f => { f.command.beforeSubmit = () => { f.identity.accountId = id(99); }; },
  account_changes_during_stream_reset: f => {
    f.page.__elonChatGptPrivateStreamTransport.prepareSend = () => { f.identity.accountId = id(99); };
  }
})) {
  test('unconfirmed runtime identity refuses invocation: ' + name, async () => {
    const f = fixture(); change(f);
    const result = f.api.regenerate(f.command); await flush(); f.runTimer(15000);
    const receipt = await result.completion;
    assert.notEqual(receipt.status, 'accepted');
    assert.notEqual(receipt.status, 'unknown');
    assert.equal(f.calls.length, 0);
    assert.equal(f.api.state().pending, false);
    assert.doesNotMatch(JSON.stringify(receipt), /synthetic-private-data/);
  });
}
