'use strict';

const assert = require('node:assert/strict');
const authModule = require('../android/app/src/main/assets/chatgpt_web_private_auth_context.js');

function response(status, payload) {
  return {
    status,
    ok: status >= 200 && status < 300,
    json: async () => payload
  };
}

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((accept, deny) => {
    resolve = accept;
    reject = deny;
  });
  return { promise, resolve, reject };
}

function createRoot(fetchImpl, enabled = true) {
  const requests = [];
  const root = {
    __elonChatGptPrivateAuthContextEnabled: enabled,
    location: { origin: 'https://chatgpt.com' },
    AbortController,
    setTimeout,
    clearTimeout,
    fetch: async (url, init) => {
      requests.push({ url, init });
      return fetchImpl(url, init);
    }
  };
  return { root, requests };
}

async function singleFlightPrewarmKeepsCredentialsPageLocal() {
  const pending = deferred();
  const fixture = createRoot(() => pending.promise);
  const context = authModule.create(fixture.root);
  const phases = [];
  context.subscribe((state) => phases.push(state.state));

  const first = context.prewarm();
  const second = context.prewarm();
  assert.strictEqual(first, second);
  assert.strictEqual(context.state().state, 'loading');
  assert.strictEqual(fixture.requests.length, 1);
  assert.strictEqual(fixture.requests[0].url, '/api/auth/session');
  assert.strictEqual(fixture.requests[0].init.credentials, 'include');
  assert.strictEqual(fixture.requests[0].init.cache, 'no-store');

  pending.resolve(response(200, {
    accessToken: 'page-local-token-12345',
    expires: new Date(Date.now() + 60 * 60 * 1000).toISOString()
  }));
  const headers = await first;
  assert.deepStrictEqual(headers, { Authorization: 'Bearer page-local-token-12345' });
  assert.strictEqual(context.state().ready, true);
  assert.strictEqual(context.canAcquire(), true);
  assert.strictEqual(phases.includes('loading'), true);
  assert.strictEqual(phases.includes('ready'), true);
  assert.strictEqual(JSON.stringify(context.state()).includes('page-local-token'), false);
}

async function observedOfficialAuthorizationWinsWithoutNetwork() {
  const fixture = createRoot(() => {
    throw new Error('network should not run');
  });
  const context = authModule.create(fixture.root);
  assert.strictEqual(context.acceptObservedHeaders(new Map([
    ['Accept', 'application/json'],
    ['Authorization', 'Bearer official-observed-token']
  ])), true);
  assert.deepStrictEqual(await context.acquireRequestHeaders(), {
    Authorization: 'Bearer official-observed-token'
  });
  assert.strictEqual(context.state().lastOutcome, 'official_observed');
  assert.strictEqual(fixture.requests.length, 0);
}

async function authFailureOpensBoundedCircuit() {
  const fixture = createRoot(async () => response(401, {}));
  const context = authModule.create(fixture.root);
  await assert.rejects(context.prewarm(), /auth_http_401/);
  assert.strictEqual(context.state().state, 'cooldown');
  assert.strictEqual(context.state().lastOutcome, 'auth');
  assert.strictEqual(context.state().failures, 1);
  assert.strictEqual(context.canAcquire(), false);
  await assert.rejects(context.prewarm(), /auth_cooldown/);
  assert.strictEqual(fixture.requests.length, 1);

  context.invalidate('page_changed');
  assert.strictEqual(context.state().state, 'idle');
  assert.strictEqual(context.copyRequestHeaders(), null);
}

async function disabledContextNeverRequestsSession() {
  const fixture = createRoot(async () => response(200, { accessToken: 'unexpected-token' }), false);
  const context = authModule.create(fixture.root);
  assert.strictEqual(context.enabled, false);
  assert.strictEqual(context.state().state, 'unavailable');
  await assert.rejects(context.prewarm(), /auth_unavailable/);
  assert.strictEqual(fixture.requests.length, 0);
}

(async () => {
  await singleFlightPrewarmKeepsCredentialsPageLocal();
  await observedOfficialAuthorizationWinsWithoutNetwork();
  await authFailureOpensBoundedCircuit();
  await disabledContextNeverRequestsSession();
  console.log('CHATGPT_WEB_PRIVATE_AUTH_CONTEXT_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
