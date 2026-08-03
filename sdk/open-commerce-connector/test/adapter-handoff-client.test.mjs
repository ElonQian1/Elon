import assert from 'node:assert/strict'
import test from 'node:test'

import {
  AdapterHandoffClientError,
  createAdapterHandoffClient,
} from '../src/index.js'

const token = `oc_adapter_${'a'.repeat(64)}`

function jsonResponse(payload, status = 200) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: { 'content-type': 'application/json' },
  })
}

function issue() {
  return {
    claim: {
      schema: 'open_commerce.adapter_business_handoff_claim.v1',
      id: 'handoffclaim-1',
    },
    lease_token: `oc_claim_${'b'.repeat(64)}`,
    lease_token_visible_once: true,
    task: { evidence: { id: 'evidence-1' }, result: { order: { id: 'order-1' } } },
  }
}

test('claim, complete, and release keep identity boundaries in the SDK', async () => {
  const requests = []
  const client = createAdapterHandoffClient({
    baseUrl: 'https://commerce.example.test',
    token,
    fetch: async (url, init) => {
      requests.push({ url: String(url), init, body: JSON.parse(init.body) })
      if (String(url).endsWith('/business-handoff-claims')) {
        return jsonResponse({
          schema: 'open_commerce.adapter_business_handoff_claim_poll.v1',
          claimed: true,
          issue: issue(),
          retry_after_seconds: 0,
          boundary: [],
        })
      }
      if (String(url).endsWith('/complete')) return jsonResponse({ id: 'receipt-1' })
      if (String(url).endsWith('/renew')) {
        return jsonResponse({
          schema: 'open_commerce.adapter_business_handoff_claim_renew.v1',
          claim: { ...issue().claim, status: 'active' },
          renewed: true,
          boundary: [],
        })
      }
      return jsonResponse({
        schema: 'open_commerce.adapter_business_handoff_claim_release.v1',
        claim: { ...issue().claim, status: 'released' },
        retryable: true,
        boundary: [],
      })
    },
  })

  const poll = await client.claimNext({ leaseSeconds: 180 })
  await client.complete(poll.issue, {
    receiptKey: 'erp-order-1',
    status: 'applied',
    targetDomain: 'erp',
    targetReference: 'erp-order-1',
    completedAt: '2026-08-03T08:00:00Z',
  })
  const released = await client.release(issue(), 'capacity_pressure')
  const renewed = await client.renew(issue(), { extendSeconds: 600 })

  assert.equal(released.retryable, true)
  assert.equal(renewed.renewed, true)
  assert.equal(requests[0].body.lease_seconds, 180)
  assert.equal(requests[1].body.lease_token, poll.issue.lease_token)
  assert.equal(requests[2].body.reason_code, 'capacity_pressure')
  assert.equal(requests[3].body.extend_seconds, 600)
  assert.equal(requests.every((request) => request.init.headers.authorization === `Bearer ${token}`), true)
  assert.equal(requests.some((request) => 'project_id' in request.body), false)
  assert.equal(requests.some((request) => 'merchant_id' in request.body), false)
  assert.equal(requests.some((request) => 'integration_id' in request.body), false)
})

test('the SDK rejects insecure remote origins and unbounded release reasons', async () => {
  assert.throws(
    () => createAdapterHandoffClient({ baseUrl: 'http://commerce.example.test', token }),
    (error) => error instanceof AdapterHandoffClientError && error.code === 'insecure_base_url',
  )
  const client = createAdapterHandoffClient({
    baseUrl: 'http://localhost:3000',
    token,
    fetch: async () => jsonResponse({}),
  })
  await assert.rejects(
    () => client.release(issue(), 'never_retry'),
    (error) => error instanceof AdapterHandoffClientError && error.code === 'invalid_release_reason',
  )
})

test('the SDK caps response bodies before returning business results', async () => {
  const client = createAdapterHandoffClient({
    baseUrl: 'https://commerce.example.test',
    token,
    fetch: async () => new Response('x'.repeat(256 * 1024 + 1)),
  })
  await assert.rejects(
    () => client.claimNext(),
    (error) => error instanceof AdapterHandoffClientError && error.code === 'response_too_large',
  )
})
