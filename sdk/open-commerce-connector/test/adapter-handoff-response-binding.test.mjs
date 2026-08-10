import assert from 'node:assert/strict'
import test from 'node:test'

import {
  AdapterHandoffClientError,
  createAdapterHandoffClient,
} from '../src/index.js'
import {
  adapterToken,
  claim,
  issue,
  jsonResponse,
  receipt,
} from './adapter-handoff-fixtures.mjs'

test('client accepts bound transitions and a machine-authority completion receipt', async () => {
  const claimed = issue()
  const client = createAdapterHandoffClient({
    baseUrl: 'https://commerce.example.test',
    token: adapterToken,
    fetch: async (url) => {
      const path = new URL(url).pathname
      if (path.endsWith('/renew')) {
        return jsonResponse({
          schema: 'open_commerce.adapter_business_handoff_claim_renew.v1',
          claim: claim(),
          renewed: true,
          boundary: [],
        })
      }
      if (path.endsWith('/release')) {
        return jsonResponse({
          schema: 'open_commerce.adapter_business_handoff_claim_release.v1',
          claim: claim({ status: 'released' }),
          retryable: true,
          boundary: [],
        })
      }
      return jsonResponse(receipt(claim()))
    },
  })

  const renewed = await client.renew(claimed, { extendSeconds: 600 })
  const released = await client.release(claimed, 'manual_release')
  const completed = await client.complete(claimed, appliedCompletion())
  assert.equal(renewed.claim.id, claimed.claim.id)
  assert.equal(released.claim.status, 'released')
  assert.equal(completed.adapter_claim_id, claimed.claim.id)
  assert.equal(completed.assertion_authority, 'adapter_token_authenticated')
  assert.equal(completed.funds_moved, false)
})

test('client rejects one-time lease and evidence binding violations', async () => {
  for (const invalidIssue of [
    issue({ lease_token_visible_once: false }),
    issue({ task: { evidence: {
      invocation_id: 'invocation-other',
      merchant_id: 'merchant-demo',
      status: 'succeeded',
      receipt_state: 'valid',
      result_available: true,
    }, result: {} } }),
  ]) {
    const client = createAdapterHandoffClient({
      baseUrl: 'https://commerce.example.test',
      token: adapterToken,
      fetch: async () => jsonResponse({
        schema: 'open_commerce.adapter_business_handoff_claim_poll.v1',
        claimed: true,
        issue: invalidIssue,
        retry_after_seconds: 0,
        boundary: [],
      }),
    })
    await assert.rejects(() => client.claimNext(), isContractError())
  }
})

test('client rejects swapped renew, release, and completion responses', async (t) => {
  const claimed = issue()
  await t.test('renew response', async () => {
    const client = clientReturning({
      schema: 'open_commerce.adapter_business_handoff_claim_renew.v1',
      claim: claim({ invocation_id: 'invocation-other' }),
      renewed: true,
      boundary: [],
    })
    await assert.rejects(() => client.renew(claimed), isContractError('claim_binding_mismatch'))
  })
  await t.test('release response', async () => {
    const client = clientReturning({
      schema: 'open_commerce.adapter_business_handoff_claim_release.v1',
      claim: claim({ id: 'handoffclaim-other', status: 'released' }),
      retryable: true,
      boundary: [],
    })
    await assert.rejects(
      () => client.release(claimed, 'manual_release'),
      isContractError('claim_binding_mismatch'),
    )
  })
  await t.test('completion receipt', async () => {
    const client = clientReturning(receipt(claim(), { merchant_id: 'merchant-other' }))
    await assert.rejects(
      () => client.complete(claimed, appliedCompletion()),
      isContractError('completion_binding_mismatch'),
    )
  })
})

test('invalid completion shapes fail before network access', async () => {
  let requests = 0
  const client = createAdapterHandoffClient({
    baseUrl: 'https://commerce.example.test',
    token: adapterToken,
    fetch: async () => {
      requests += 1
      return jsonResponse({})
    },
  })
  await assert.rejects(
    () => client.complete(issue(), { ...appliedCompletion(), targetReference: undefined }),
    isContractError('invalid_completion'),
  )
  await assert.rejects(
    () => client.complete(issue(), {
      ...appliedCompletion(),
      status: 'rejected',
      targetReference: undefined,
      errorCode: undefined,
    }),
    isContractError('invalid_completion'),
  )
  assert.equal(requests, 0)
})

test('completion identifiers use the same lowercase normalization as the server', async () => {
  let requestBody
  const client = createAdapterHandoffClient({
    baseUrl: 'https://commerce.example.test',
    token: adapterToken,
    fetch: async (_url, init) => {
      requestBody = JSON.parse(init.body)
      return jsonResponse(receipt(claim(), {
        status: 'rejected',
        receipt_key: 'erp-order-upper',
        target_reference_sha256: null,
        error_code: 'source_rejected',
      }))
    },
  })
  await client.complete(issue(), {
    receiptKey: 'ERP-ORDER-UPPER',
    status: 'rejected',
    targetDomain: 'erp',
    errorCode: 'SOURCE_REJECTED',
    completedAt: '2026-08-10T00:00:01.000Z',
  })
  assert.equal(requestBody.receipt_key, 'erp-order-upper')
  assert.equal(requestBody.error_code, 'source_rejected')
})

function appliedCompletion() {
  return {
    receiptKey: 'erp-order-1',
    status: 'applied',
    targetDomain: 'erp',
    targetReference: 'erp-order-1',
    completedAt: '2026-08-10T00:00:01.000Z',
  }
}

function clientReturning(payload) {
  return createAdapterHandoffClient({
    baseUrl: 'https://commerce.example.test',
    token: adapterToken,
    fetch: async () => jsonResponse(payload),
  })
}

function isContractError(code) {
  return (error) => error instanceof AdapterHandoffClientError && (!code || error.code === code)
}
