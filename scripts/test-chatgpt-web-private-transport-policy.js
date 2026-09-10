'use strict';

const assert = require('node:assert/strict');
const policyModule = require('../android/app/src/main/assets/chatgpt_web_private_transport_policy.js');

class MemoryStorage {
  constructor() { this.values = new Map(); }
  getItem(key) { return this.values.has(key) ? this.values.get(key) : null; }
  setItem(key, value) { this.values.set(key, String(value)); }
}

let now = 1_000_000;
const storage = new MemoryStorage();
const policy = policyModule.create({ enabled: true, now: () => now, storage });

assert.equal(policy.canAttempt(true), false);
policy.recordOfficial(200, 400);
assert.equal(policy.canAttempt(false), false);
assert.equal(policy.canAttempt(true), true);
assert.equal(policy.attemptBudgetMs(), 3000);

policy.recordOfficial(0, 50);
assert.equal(policy.canAttempt(true), true, 'cancelled navigation must not cool down healthy transport');
assert.equal(policy.snapshot().lastOutcome, 'none');

policy.recordSuccess(500);
let health = policy.snapshot();
assert.equal(health.successes, 1);
assert.equal(health.privateLatencyMs, 500);
assert.equal(health.lastOutcome, 'success');
assert.equal(health.attemptBudgetMs, 3000);

policy.recordFailure('timeout');
health = policy.snapshot();
assert.equal(health.failures, 1);
assert.equal(health.lastOutcome, 'timeout');
assert.equal(policy.canAttempt(true), false);
assert.ok(health.cooldownRemainingMs >= 60_000);

now += 60_001;
assert.equal(policy.canAttempt(true), true);
const restored = policyModule.create({ enabled: true, now: () => now, storage });
assert.equal(restored.snapshot().successes, 1);
assert.equal(restored.snapshot().failures, 1);
assert.equal(restored.canAttempt(true), true);

const persisted = storage.getItem(policyModule.storageKey);
assert.doesNotMatch(persisted, /authorization|cookie|token|message|title|content/i);

now += (2 * 60 * 1000) + 1;
assert.equal(restored.canAttempt(true), false);
const disabled = policyModule.create({ enabled: false, now: () => now, storage });
assert.equal(disabled.canAttempt(true), false);

const slowStorage = new MemoryStorage();
const slow = policyModule.create({ enabled: true, now: () => now, storage: slowStorage });
slow.recordOfficial(200, 1000);
assert.equal(slow.attemptBudgetMs(), 3550);
slow.recordSuccess(1000);
assert.equal(slow.attemptBudgetMs(), 3550);

const account = policyModule.create({ enabled: true, now: () => now, storage, scope: 'account_read' });
assert.equal(account.snapshot().failures, 0);
account.recordFailure('network');
assert.equal(policyModule.create({ enabled: true, now: () => now, storage,
  scope: 'account_read' }).snapshot().lastOutcome, 'network');
assert.equal(policyModule.create({ enabled: true, now: () => now, storage }).snapshot().lastOutcome, 'timeout');
for (const status of [401, 403, 429]) {
  const shared = new MemoryStorage();
  const background = policyModule.create({ enabled: true, now: () => now, storage: shared });
  background.recordOfficial(status, 20);
  const explicit = policyModule.create({ enabled: true, now: () => now, storage: shared, scope: 'account_read' });
  assert.equal(explicit.snapshot().cooldownRemainingMs, background.snapshot().cooldownRemainingMs);
  assert.equal(explicit.snapshot().lastOutcome, status === 429 ? 'rate_limit' : 'auth');
  now += 500;
  const restoredExplicit = policyModule.create({ enabled: true, now: () => now, storage: shared, scope: 'account_read' });
  assert.equal(restoredExplicit.snapshot().cooldownRemainingMs, explicit.snapshot().cooldownRemainingMs,
    'scope migration must not extend protected cooldown on reload');
}
console.log('CHATGPT_WEB_PRIVATE_TRANSPORT_POLICY_TESTS=passed');
