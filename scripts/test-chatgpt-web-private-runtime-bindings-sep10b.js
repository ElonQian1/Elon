'use strict';
const test = require('node:test');
const assert = require('node:assert/strict');
const { attach, CDN } = require('./fixtures/chatgpt-runtime-bindings');
const profile = require('./fixtures/chatgpt-runtime-bindings-sep10b');

test('current attachment validators and quota helpers preserve exact singleton identities', async () => {
  const expectedExports = Object.fromEntries(Object.keys(profile.files).map(role => [role,
    { ...profile.expectedExports[role], ...profile.extraExports[role] }]));
  const modules = Object.fromEntries(Object.entries(profile.extraExports).map(([role, keys]) => [role,
    Object.fromEntries(Object.keys(keys).map(key => [key, { role, key }]))]));
  modules.composer.fh = { validateChatAttachment: () => true };
  const page = { location: { origin: 'https://chatgpt.com' }, document: {}, setTimeout, clearTimeout };
  const binding = attach(page, modules, { ...profile, expectedExports });
  for (const role of Object.keys(modules)) {
    const loaded = await binding.api.load(role);
    for (const [key, value] of Object.entries(modules[role])) assert.equal(loaded[key], value);
    assert.equal(await binding.api.load(role), loaded);
  }
  assert.equal(binding.loads.length, 3);
});

test('new profile admits the actual regeneration consumer and observes its one owned variant', async () => {
  const { fixture, flush } = require('./fixtures/chatgpt-runtime-regeneration');
  const f = fixture();
  const binding = attach(f.page, f.modules, profile);
  const runtime = require('../android/app/src/main/assets/chatgpt_web_private_regenerate_runtime');
  const api = runtime.create(f.page, {
    contract: require('../android/app/src/main/assets/chatgpt_web_private_regenerate_contract')
  });
  const transaction = api.regenerate(f.command);
  assert.equal(transaction.handled, true);
  for (let i = 0; i < 3; i++) await flush();
  assert.equal(f.calls.length, 1);
  f.publish();
  const result = await transaction.completion;
  assert.equal(result.code, 'regenerate_observed');
  assert.equal(f.calls.length, 1);
  assert.equal(binding.api.state().profile_id, profile.id);
  assert.deepEqual(new Set(binding.loads), new Set([CDN + profile.files.shared, CDN + profile.files.conversation]));
  assert.equal(f.listeners.size, 0); assert.equal(f.timers.size, 0);
});

test('an adjacent uninspected build cannot be admitted using a filename prefix', async () => {
  const page = { location: { origin: 'https://chatgpt.com' }, document: {}, setTimeout, clearTimeout };
  const binding = attach(page, { shared: { H3: () => true } }, profile);
  binding.observed.clear();
  binding.observed.add(CDN + '4813494d-unknown-new-build.js');
  assert.equal(binding.api.observed('shared'), false);
  await assert.rejects(binding.api.load('shared'), /runtime_not_observed/);
  assert.equal(binding.loads.length, 0);
});
