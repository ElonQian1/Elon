'use strict';

const assert = require('node:assert/strict');
const mutationModule = require('../android/app/src/main/assets/chatgpt_web_private_conversation_mutation.js');

function response(status, payload) {
  return {
    status,
    ok: status >= 200 && status < 300,
    json: async () => payload
  };
}

function deferred() {
  let resolve;
  const promise = new Promise((accept) => { resolve = accept; });
  return { promise, resolve };
}

function fixture(fetchImpl, enabled = true, rootOverrides = {}) {
  const calls = [];
  const accepted = [];
  const root = Object.assign({
    location: { origin: 'https://chatgpt.com' },
    AbortController,
    setTimeout,
    clearTimeout,
    fetch: async (url, init) => {
      calls.push({ url, init });
      return fetchImpl(url, init);
    }
  }, rootOverrides);
  const transport = {
    acquireSameOriginRequestHeaders: async () => ({
      Authorization: 'Bearer page-local-secret',
      Cookie: 'must-not-be-forwarded',
      'User-Agent': 'must-not-be-forwarded',
      'oai-device-id': 'page-local-device'
    })
  };
  const directory = {
    acceptPinnedState: (id, pinned) => {
      accepted.push({ id, pinned });
      return true;
    }
  };
  return {
    calls,
    accepted,
    transport: mutationModule.create(root, {
      enabled,
      privateTransport: transport,
      directory
    })
  };
}

async function successfulWriteIsSingleAndReconciled() {
  const test = fixture(async (url) => {
    if (url === '/backend-api/pins') {
      return response(200, { items: [{ id: 'conversation-123' }] });
    }
    return response(200, {});
  });
  const result = await test.transport.setPinned('/c/conversation-123', true);

  assert.deepStrictEqual(result, {
    ok: true,
    code: 'mutation_confirmed',
    attempted: true,
    reconciled: true
  });
  assert.strictEqual(test.calls.length, 2);
  const write = test.calls[0];
  assert.strictEqual(write.url, '/backend-api/conversation/conversation-123');
  assert.strictEqual(write.init.method, 'PATCH');
  assert.strictEqual(write.init.credentials, 'include');
  assert.strictEqual(write.init.cache, 'no-store');
  assert.deepStrictEqual(JSON.parse(write.init.body), { is_starred: true });
  assert.strictEqual(write.init.headers.Cookie, undefined);
  assert.strictEqual(write.init.headers['User-Agent'], undefined);
  assert.strictEqual(write.init.headers.Authorization, 'Bearer page-local-secret');
  assert.deepStrictEqual(test.accepted, [
    { id: 'conversation-123', pinned: true },
    { id: 'conversation-123', pinned: true }
  ]);
  assert.strictEqual(test.transport.state().failures, 0);
}

async function concurrentIntentNeverCreatesASecondWrite() {
  const pending = deferred();
  const test = fixture((url) => url === '/backend-api/pins'
    ? response(200, { items: [] })
    : pending.promise);
  const first = test.transport.setPinned('/c/conversation-123', true);
  await Promise.resolve();
  const second = await test.transport.setPinned('/c/conversation-456', true);
  assert.deepStrictEqual(second, {
    ok: false,
    code: 'mutation_busy',
    attempted: false,
    reconciled: false
  });
  assert.strictEqual(test.calls.length, 1);
  pending.resolve(response(200, {}));
  await first;
  assert.strictEqual(test.calls.length, 2);
}

async function successfulWriteIsNotRolledBackByLaggingPinIndex() {
  const test = fixture(async (url) => url === '/backend-api/pins'
    ? response(200, { items: [] })
    : response(200, {}));
  const result = await test.transport.setPinned('/c/conversation-123', true);

  assert.deepStrictEqual(result, {
    ok: true,
    code: 'mutation_server_acknowledged',
    attempted: true,
    reconciled: false
  });
  assert.deepStrictEqual(test.accepted, [
    { id: 'conversation-123', pinned: true }
  ]);
}

async function timedOutWriteIsReconciledWithoutWriteReplay() {
  let pinReads = 0;
  const test = fixture(async (url, init) => {
    if (url === '/backend-api/pins') {
      pinReads += 1;
      return response(200, pinReads === 1
        ? { items: [] }
        : { items: [{ id: 'conversation-123' }] });
    }
    return new Promise((resolve, reject) => {
      init.signal.addEventListener('abort', () => reject(new Error('aborted')), { once: true });
    });
  }, true, {
    setTimeout: (callback) => setImmediate(callback),
    clearTimeout: (id) => clearImmediate(id)
  });

  const result = await test.transport.setPinned('/c/conversation-123', true);
  assert.deepStrictEqual(result, {
    ok: true,
    code: 'mutation_confirmed_after_timeout',
    attempted: true,
    reconciled: true
  });
  assert.strictEqual(test.calls.filter((call) => call.init.method === 'PATCH').length, 1);
  assert.strictEqual(pinReads, 2);
  assert.deepStrictEqual(test.accepted, [
    { id: 'conversation-123', pinned: true }
  ]);
  assert.strictEqual(test.transport.state().failures, 0);
}

async function serverFailureIsNotRetriedOrOptimisticallyApplied() {
  const test = fixture(async () => response(500, {}));
  const result = await test.transport.setPinned('/g/g-p-demo/c/conversation-123', false);
  assert.deepStrictEqual(result, {
    ok: false,
    code: 'mutation_http_500',
    attempted: true,
    reconciled: false
  });
  assert.strictEqual(test.calls.length, 1);
  assert.deepStrictEqual(test.accepted, []);
  assert.strictEqual(test.transport.state().state, 'cooldown');
  const blocked = await test.transport.setPinned('/c/conversation-123', false);
  assert.strictEqual(blocked.code, 'mutation_circuit_open');
  assert.strictEqual(blocked.attempted, false);
  assert.strictEqual(test.calls.length, 1);
}

async function missingAuthorizationNeverAttemptsAWrite() {
  const test = fixture(async () => { throw new Error('fetch must not run'); });
  test.transport = mutationModule.create({
    location: { origin: 'https://chatgpt.com' },
    AbortController,
    setTimeout,
    clearTimeout,
    fetch: async () => { throw new Error('fetch must not run'); }
  }, {
    enabled: true,
    privateTransport: { acquireSameOriginRequestHeaders: async () => ({}) },
    directory: null
  });
  const result = await test.transport.setPinned('/c/conversation-123', true);
  assert.strictEqual(result.ok, false);
  assert.strictEqual(result.code, 'mutation_auth_unavailable');
  assert.strictEqual(result.attempted, false);
  assert.strictEqual(test.calls.length, 0);
}

async function disabledOrInvalidMutationIsSideEffectFree() {
  const disabled = fixture(async () => response(200, {}), false);
  const result = await disabled.transport.setPinned('/c/conversation-123', true);
  assert.strictEqual(result.code, 'mutation_unavailable');
  assert.strictEqual(disabled.calls.length, 0);
  const invalid = await fixture(async () => response(200, {})).transport
    .setPinned('/auth/login', true);
  assert.strictEqual(invalid.code, 'invalid_mutation');
}

async function adapterHandlerReportsTheCorrelatedResultAndRefreshesOnlyAfterSuccess() {
  const test = fixture(async (url) => url === '/backend-api/pins'
    ? response(200, { items: [{ id: 'conversation-123' }] })
    : response(200, {}));
  const directoryRequests = [];
  const snapshots = [];
  const result = new Promise((resolve) => {
    assert.strictEqual(test.transport.handle(
      'set_conversation_pinned',
      { value: '/c/conversation-123', selected: true },
      (action, ok, detail) => resolve({ action, ok, detail }),
      (immediate) => snapshots.push(immediate),
      { emitSnapshot: (scope) => directoryRequests.push(scope) }
    ), true);
  });

  assert.deepStrictEqual(await result, {
    action: 'set_conversation_pinned',
    ok: true,
    detail: 'mutation_confirmed'
  });
  assert.deepStrictEqual(directoryRequests, [null]);
  assert.deepStrictEqual(snapshots, [true]);
  assert.strictEqual(test.transport.handle('unrelated_action', {}, () => {}), false);
}

(async () => {
  await successfulWriteIsSingleAndReconciled();
  await concurrentIntentNeverCreatesASecondWrite();
  await successfulWriteIsNotRolledBackByLaggingPinIndex();
  await timedOutWriteIsReconciledWithoutWriteReplay();
  await serverFailureIsNotRetriedOrOptimisticallyApplied();
  await missingAuthorizationNeverAttemptsAWrite();
  await disabledOrInvalidMutationIsSideEffectFree();
  await adapterHandlerReportsTheCorrelatedResultAndRefreshesOnlyAfterSuccess();
  console.log('CHATGPT_WEB_PRIVATE_CONVERSATION_MUTATION_TESTS=passed');
})().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
