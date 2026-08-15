import assert from 'node:assert/strict'
import test from 'node:test'

import {
  AdapterHandoffRejectError,
  VERIFIED_ERP_HANDOFF_READBACK_SCHEMA,
  VERIFIED_ERP_HANDOFF_SOURCE_SCHEMA,
  VerifiedErpHandoffError,
  createAdapterHandoffWorker,
  createVerifiedErpHandoffHandler,
} from '../src/index.js'

test('verified ERP handler writes, reads back, and only then completes applied', async () => {
  const claimed = claim()
  const events = []
  const completions = []
  const releases = []
  let appliedSource
  const handler = createVerifiedErpHandoffHandler({
    apply: async ({ source, result }, pluginContext) => {
      events.push('apply')
      appliedSource = source
      assert.equal(result.order.id, 'merchant-order-1')
      assert.equal(pluginContext.idempotencyKey, claimed.invocation_id)
      assert.equal(Object.isFrozen(source), true)
      assert.equal(Object.isFrozen(source.businessReceipt), true)
      assert.equal(Object.hasOwn(source, 'claimId'), false)
      return { targetReference: 'erp-order-1' }
    },
    readBack: async ({ source, targetReference }) => {
      events.push('readBack')
      assert.equal(source, appliedSource)
      return readBackProof(targetReference, source)
    },
  })
  const client = fakeWorkerClient(claimed, {
    complete: async (_issue, completion) => {
      events.push('complete')
      completions.push(completion)
      return { id: 'receipt-1' }
    },
    release: async (_issue, reason) => releases.push(reason),
  })
  const worker = createAdapterHandoffWorker({ client, targetDomain: 'erp', handler })

  const result = await worker.runOnce()

  assert.deepEqual(events, ['apply', 'readBack', 'complete'])
  assert.equal(result.status, 'applied')
  assert.equal(completions[0].targetReference, 'erp-order-1')
  assert.equal(releases.length, 0)
  assert.equal(appliedSource.schema, VERIFIED_ERP_HANDOFF_SOURCE_SCHEMA)
  assert.match(appliedSource.sourceDigest, /^[a-f0-9]{64}$/)
  assert.equal(appliedSource.fundsMoved, false)
})

test('same Invocation keeps the idempotency key and source digest across attempts', async () => {
  const writes = []
  const handler = createVerifiedErpHandoffHandler({
    apply: async ({ source }, pluginContext) => {
      writes.push({ source, pluginContext })
      return { targetReference: 'erp-order-1' }
    },
    readBack: async ({ source, targetReference }) => readBackProof(targetReference, source),
  })
  const first = claim({ id: 'handoffclaim-first', attempt_no: 1 })
  const second = claim({
    id: 'handoffclaim-second',
    attempt_no: 4,
    adapter_credential_id: 'adapter-credential-rotated',
    adapter_credential_version: 2,
  })

  await handler(task(first), context(first))
  await handler(task(second), context(second))

  assert.equal(writes[0].pluginContext.idempotencyKey, first.invocation_id)
  assert.equal(writes[1].pluginContext.idempotencyKey, first.invocation_id)
  assert.equal(writes[0].source.sourceDigest, writes[1].source.sourceDigest)
  assert.equal(writes[0].source.adapterCredential.version, 1)
  assert.equal(writes[1].source.adapterCredential.version, 2)
})

test('invalid evidence fails closed before ERP apply', async (t) => {
  const cases = [
    ['schema', (value) => { value.evidence.schema = 'open_commerce.merchant_business_evidence.v0' }],
    ['claim binding', (value) => { value.evidence.merchant_id = 'merchant-other' }],
    ['result digest', (value) => { delete value.evidence.result_sha256 }],
    ['business receipt', (value) => { value.evidence.business_receipt = null }],
    ['result receipt', (value) => {
      value.result._yilong_business_receipt.reference_id = 'merchant-order-other'
    }],
    ['source authority', (value) => { value.evidence.source_authority = 'platform_handler_result' }],
    ['funds boundary', (value) => { value.evidence.funds_moved = true }],
  ]

  for (const [name, mutate] of cases) {
    await t.test(name, async () => {
      let applyCalls = 0
      const handler = createVerifiedErpHandoffHandler({
        apply: async () => {
          applyCalls += 1
          return { targetReference: 'erp-order-1' }
        },
        readBack: async () => { throw new Error('readBack must not run') },
      })
      const claimed = claim()
      const input = task(claimed)
      mutate(input)

      await assert.rejects(
        () => handler(input, context(claimed)),
        (error) => error instanceof VerifiedErpHandoffError,
      )
      assert.equal(applyCalls, 0)
    })
  }
})

test('every ERP readback binding is required to match the written source', async (t) => {
  const mutations = [
    ['project', (proof) => { proof.source.projectId = 'project-other' }],
    ['merchant', (proof) => { proof.source.merchantId = 'merchant-other' }],
    ['Invocation', (proof) => { proof.source.invocationId = 'invocation-other' }],
    ['integration', (proof) => { proof.source.integrationId = 'integration-other' }],
    ['credential version', (proof) => { proof.source.adapterCredential.version = 99 }],
    ['evidence digest', (proof) => { proof.source.evidenceResultSha256 = '0'.repeat(64) }],
    ['source digest', (proof) => { proof.source.sourceDigest = '0'.repeat(64) }],
    ['target reference', (proof) => { proof.targetReference = 'erp-order-other' }],
  ]

  for (const [name, mutate] of mutations) {
    await t.test(name, async () => {
      const handler = createVerifiedErpHandoffHandler({
        apply: async () => ({ targetReference: 'erp-order-1' }),
        readBack: async ({ source, targetReference }) => {
          const proof = readBackProof(targetReference, source)
          mutate(proof)
          return proof
        },
      })
      const claimed = claim()
      await assert.rejects(
        () => handler(task(claimed), context(claimed)),
        (error) => error instanceof VerifiedErpHandoffError && error.code === 'readback_mismatch',
      )
    })
  }
})

test('readback mismatch and write outages release the lease without completing', async (t) => {
  await t.test('readback mismatch', async () => {
    const claimed = claim()
    const observed = { completions: 0, releases: [] }
    const handler = createVerifiedErpHandoffHandler({
      apply: async () => ({ targetReference: 'erp-order-1' }),
      readBack: async ({ source, targetReference }) => {
        const proof = readBackProof(targetReference, source)
        proof.source.resultSha256 = 'tampered-extra-field'
        return proof
      },
    })
    const worker = createAdapterHandoffWorker({
      client: fakeWorkerClient(claimed, {
        complete: async () => { observed.completions += 1 },
        release: async (_issue, reason) => observed.releases.push(reason),
      }),
      targetDomain: 'erp',
      handler,
    })

    await assert.rejects(() => worker.runOnce(), /source binding does not match/)
    assert.equal(observed.completions, 0)
    assert.deepEqual(observed.releases, ['transient_failure'])
  })

  await t.test('write outage', async () => {
    const claimed = claim()
    const releases = []
    const handler = createVerifiedErpHandoffHandler({
      apply: async () => { throw new Error('ERP unavailable') },
      readBack: async () => { throw new Error('readBack must not run') },
    })
    const worker = createAdapterHandoffWorker({
      client: fakeWorkerClient(claimed, {
        release: async (_issue, reason) => releases.push(reason),
      }),
      targetDomain: 'erp',
      handler,
    })

    await assert.rejects(() => worker.runOnce(), /ERP unavailable/)
    assert.deepEqual(releases, ['transient_failure'])
  })
})

test('explicit plugin business rejection keeps the existing permanent rejection path', async () => {
  const claimed = claim()
  const completions = []
  const releases = []
  let readBackCalls = 0
  const handler = createVerifiedErpHandoffHandler({
    apply: async () => { throw new AdapterHandoffRejectError('unsupported_external_entity') },
    readBack: async () => {
      readBackCalls += 1
      throw new Error('readBack must not run')
    },
  })
  const worker = createAdapterHandoffWorker({
    client: fakeWorkerClient(claimed, {
      complete: async (_issue, completion) => {
        completions.push(completion)
        return { id: 'receipt-rejected' }
      },
      release: async (_issue, reason) => releases.push(reason),
    }),
    targetDomain: 'erp',
    handler,
  })

  const result = await worker.runOnce()

  assert.equal(result.status, 'rejected')
  assert.equal(completions[0].errorCode, 'unsupported_external_entity')
  assert.equal(readBackCalls, 0)
  assert.equal(releases.length, 0)
})

function claim(overrides = {}) {
  return {
    schema: 'open_commerce.adapter_business_handoff_claim.v1',
    id: 'handoffclaim-verified-1',
    project_id: 'project-demo',
    merchant_id: 'merchant-demo',
    invocation_id: 'invocation-demo-1',
    integration_id: 'integration-demo',
    adapter_credential_id: 'adapter-credential-1',
    adapter_credential_version: 1,
    attempt_no: 1,
    status: 'active',
    lease_expires_at: new Date(Date.now() + 60_000).toISOString(),
    lease_deadline_at: new Date(Date.now() + 3_600_000).toISOString(),
    ...overrides,
  }
}

function task(claimed) {
  return {
    evidence: {
      schema: 'open_commerce.merchant_business_evidence.v1',
      invocation_id: claimed.invocation_id,
      merchant_id: claimed.merchant_id,
      status: 'succeeded',
      source_authority: 'merchant_runtime_asserted',
      receipt_state: 'valid',
      business_receipt: {
        schema: 'open_commerce.merchant_business_receipt.v1',
        entity_type: 'order',
        reference_id: 'merchant-order-1',
        state: 'accepted',
        occurred_at: '2026-08-15T01:00:00Z',
        amount_minor: 2600,
        currency: 'CNY',
      },
      result_available: true,
      result_sha256: 'c'.repeat(64),
      funds_moved: false,
    },
    result: {
      _yilong_business_receipt: {
        schema: 'open_commerce.merchant_business_receipt.v1',
        entity_type: 'order',
        reference_id: 'merchant-order-1',
        state: 'accepted',
        occurred_at: '2026-08-15T01:00:00Z',
        amount_minor: 2600,
        currency: 'CNY',
      },
      order: { id: 'merchant-order-1' },
    },
  }
}

function context(claimed) {
  return {
    claim: claimed,
    idempotencyKey: claimed.invocation_id,
    attemptNo: claimed.attempt_no,
    signal: new AbortController().signal,
  }
}

function issue(claimed) {
  return {
    claim: claimed,
    lease_token: `oc_claim_${'b'.repeat(64)}`,
    lease_token_visible_once: true,
    task: task(claimed),
  }
}

function readBackProof(targetReference, source) {
  return {
    schema: VERIFIED_ERP_HANDOFF_READBACK_SCHEMA,
    targetReference,
    source: structuredClone(source),
  }
}

function fakeWorkerClient(claimed, overrides = {}) {
  return {
    claimNext: async () => ({ claimed: true, issue: issue(claimed) }),
    renew: async () => { throw new Error('unexpected renewal') },
    complete: overrides.complete ?? (async () => ({ id: 'receipt-1' })),
    release: overrides.release ?? (async () => undefined),
  }
}
