import assert from 'node:assert/strict'
import test from 'node:test'

import { MERCHANT_RUNTIME_MAX_BODY_BYTES } from '../src/index.js'
import {
  createTestRuntime,
  nowUnix,
  runtimeSecret,
  signedInvocation,
} from './merchant-runtime-fixtures.mjs'

test('runtime verifies a signed request and exposes platform provenance to the handler', async () => {
  let observed
  const runtime = createTestRuntime({
    handlers: {
      async 'catalog.search'(input, context) {
        observed = { input, context }
        return { items: [] }
      },
    },
  })

  const response = await runtime.handleInvoke(signedInvocation())

  assert.equal(response.status, 200)
  assert.deepEqual(observed.input, { query: 'latte' })
  assert.equal(observed.context.credentialEnvironment, 'sandbox')
  assert.equal(observed.context.credentialId, 'credential-demo-1')
  assert.equal(observed.context.actionConfirmationId, null)
})

test('runtime rejects invalid signature identity, encoding, timestamp, and body binding', async (t) => {
  const runtime = createTestRuntime()
  const cases = [
    ['key id', signedInvocation({}, { keyId: 'OPEN_COMMERCE_RUNTIME_SECRET_OTHER' })],
    ['secret', signedInvocation({}, { secret: `${runtimeSecret}-wrong` })],
    ['encoding', signedInvocation({}, { signature: 'sha256=wrong' })],
    ['replay window', signedInvocation({}, { nowUnix: nowUnix + 301 })],
    ['envelope timestamp', signedInvocation(
      { issued_at_unix: nowUnix - 1 },
      { timestamp: nowUnix, nowUnix },
    )],
  ]
  for (const [name, request] of cases) {
    await t.test(name, async () => {
      const response = await runtime.handleInvoke(request)
      assert.equal(response.status, 401)
      assert.equal(response.body.error_code, 'signature_rejected')
    })
  }
})

test('authorized capabilities require a grant and order commits require platform confirmation', async () => {
  const runtime = createTestRuntime({
    capabilities: [{
      key: 'order.commit',
      access: 'authorized',
      input_schema: { type: 'object' },
    }],
    handlers: {
      async 'order.commit'(input, context) {
        return { order_id: 'order-1', quote_id: input.quote_id, confirmation: context.actionConfirmationId }
      },
    },
  })
  const base = {
    capability_key: 'order.commit',
    input: { quote_id: 'quote-1' },
    idempotency_key: 'order-idempotency-1',
  }

  const noGrant = await runtime.handleInvoke(signedInvocation(base))
  assert.equal(noGrant.status, 401)

  const noConfirmation = await runtime.handleInvoke(signedInvocation({
    ...base,
    grant_id: 'grant-demo-1',
    input: { quote_id: 'quote-1', confirmed_by_user: true, confirmation_id: 'legacy-only' },
  }))
  assert.equal(noConfirmation.status, 400)
  assert.equal(noConfirmation.body.error_code, 'confirmation_required')

  const confirmed = await runtime.handleInvoke(signedInvocation({
    ...base,
    grant_id: 'grant-demo-1',
    action_confirmation_id: 'action-confirmation-1',
  }))
  assert.equal(confirmed.status, 200)
  assert.equal(confirmed.body.result.confirmation, 'action-confirmation-1')
})

test('runtime rejects oversized and non-raw request bodies before a handler runs', async () => {
  let calls = 0
  const runtime = createTestRuntime({
    handlers: { async 'catalog.search'() { calls += 1; return {} } },
  })
  const oversized = 'x'.repeat(MERCHANT_RUNTIME_MAX_BODY_BYTES + 1)

  const tooLarge = await runtime.handleInvoke(signedInvocation({}, { body: oversized }))
  const notRaw = await runtime.handleInvoke({ ...signedInvocation(), body: { invalid: true } })

  assert.equal(tooLarge.status, 400)
  assert.equal(tooLarge.body.error_code, 'request_too_large')
  assert.equal(notRaw.status, 400)
  assert.equal(notRaw.body.error_code, 'invalid_request')
  assert.equal(calls, 0)
})
