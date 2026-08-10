import assert from 'node:assert/strict'
import test from 'node:test'

import {
  AdapterHandoffRejectError,
  AdapterHandoffReleaseError,
  createAdapterHandoffWorker,
} from '../src/index.js'
import { claim, issue, receipt } from './adapter-handoff-fixtures.mjs'

test('worker retries one idempotent applied completion', async () => {
  const completions = []
  const releases = []
  const client = fakeClient({
    complete: async (claimed, completion) => {
      completions.push({ claimed, completion })
      if (completions.length === 1) throw new Error('temporary completion outage')
      return receipt(claimed.claim, {
        receipt_key: completion.receiptKey,
        completed_at: completion.completedAt,
      })
    },
    release: async (...args) => releases.push(args),
  })
  const contexts = []
  const worker = createAdapterHandoffWorker({
    client,
    targetDomain: 'erp',
    completionAttempts: 2,
    handler: async (_task, context) => {
      contexts.push(context)
      return { status: 'applied', targetReference: 'erp-order-1' }
    },
  })

  const result = await worker.runOnce()
  assert.equal(result.claimed, true)
  assert.equal(result.status, 'applied')
  assert.equal(completions.length, 2)
  assert.equal(completions[0].completion.receiptKey, completions[1].completion.receiptKey)
  assert.match(completions[0].completion.receiptKey, /^adapter-[a-f0-9]{40}$/)
  assert.equal(contexts[0].idempotencyKey, 'invocation-demo-1')
  assert.equal(releases.length, 0)
})

test('worker renews an expiring lease before completion', async () => {
  const initial = issue({
    claim: claim({
      lease_expires_at: new Date(Date.now() + 20).toISOString(),
      lease_deadline_at: new Date(Date.now() + 60_000).toISOString(),
    }),
  })
  let renewals = 0
  let completedIssue
  const client = fakeClient({
    issue: initial,
    renew: async (current) => {
      renewals += 1
      return {
        claim: {
          ...current.claim,
          lease_expires_at: new Date(Date.now() + 60_000).toISOString(),
        },
      }
    },
    complete: async (current, completion) => {
      completedIssue = current
      return receipt(current.claim, {
        receipt_key: completion.receiptKey,
        completed_at: completion.completedAt,
      })
    },
  })
  const worker = createAdapterHandoffWorker({
    client,
    targetDomain: 'erp',
    leaseSeconds: 60,
    renewBeforeSeconds: 10,
    handler: async () => {
      await sleep(350)
      return { status: 'applied', targetReference: 'erp-order-1' }
    },
  })

  await worker.runOnce()
  assert.equal(renewals, 1)
  assert.notEqual(completedIssue.claim.lease_expires_at, initial.claim.lease_expires_at)
})

test('handler success can stop an in-flight renewal without becoming a failure', async () => {
  const initial = issue({
    claim: claim({
      lease_expires_at: new Date(Date.now() + 20).toISOString(),
      lease_deadline_at: new Date(Date.now() + 60_000).toISOString(),
    }),
  })
  let renewStarted = false
  let completions = 0
  let releases = 0
  const client = fakeClient({
    issue: initial,
    renew: async (_current, { signal }) => {
      renewStarted = true
      await waitForAbort(signal)
      throw abortError()
    },
    complete: async (current, completion) => {
      completions += 1
      return receipt(current.claim, {
        receipt_key: completion.receiptKey,
        completed_at: completion.completedAt,
      })
    },
    release: async () => { releases += 1 },
  })
  const worker = createAdapterHandoffWorker({
    client,
    targetDomain: 'erp',
    leaseSeconds: 60,
    renewBeforeSeconds: 10,
    handler: async () => {
      await sleep(320)
      return { status: 'applied', targetReference: 'erp-order-1' }
    },
  })

  await worker.runOnce()
  assert.equal(renewStarted, true)
  assert.equal(completions, 1)
  assert.equal(releases, 0)
})

test('renewal failure and hard deadline both release instead of completing', async (t) => {
  await t.test('renewal failure', async () => {
    const initial = expiringIssue()
    const releases = []
    let completions = 0
    const client = fakeClient({
      issue: initial,
      renew: async () => { throw new Error('renewal transport failed') },
      complete: async () => { completions += 1 },
      release: async (_issue, reason) => releases.push(reason),
    })
    const worker = createAdapterHandoffWorker({
      client,
      targetDomain: 'erp',
      leaseSeconds: 60,
      renewBeforeSeconds: 10,
      handler: async (_task, context) => {
        await waitForAbort(context.signal)
        throw abortError()
      },
    })
    await assert.rejects(() => worker.runOnce(), /renewal transport failed/)
    assert.equal(completions, 0)
    assert.deepEqual(releases, ['transient_failure'])
  })

  await t.test('hard deadline', async () => {
    const initial = expiringIssue()
    const releases = []
    let completions = 0
    const client = fakeClient({
      issue: initial,
      renew: async (current) => ({
        claim: {
          ...current.claim,
          lease_expires_at: new Date(Date.now() - 1_000).toISOString(),
          lease_deadline_at: new Date(Date.now() - 1_000).toISOString(),
        },
      }),
      complete: async () => { completions += 1 },
      release: async (_issue, reason) => releases.push(reason),
    })
    const worker = createAdapterHandoffWorker({
      client,
      targetDomain: 'erp',
      leaseSeconds: 60,
      renewBeforeSeconds: 10,
      handler: async () => {
        await sleep(320)
        return { status: 'applied', targetReference: 'erp-order-1' }
      },
    })
    await assert.rejects(() => worker.runOnce(), /hard lease deadline reached/)
    assert.equal(completions, 0)
    assert.deepEqual(releases, ['transient_failure'])
  })
})

test('permanent rejection completes while release and shutdown remain retryable', async (t) => {
  await t.test('permanent rejection', async () => {
    const completions = []
    const releases = []
    const client = fakeClient({
      complete: async (current, completion) => {
        completions.push(completion)
        return receipt(current.claim, {
          status: 'rejected',
          receipt_key: completion.receiptKey,
          target_reference_sha256: null,
          error_code: completion.errorCode,
          completed_at: completion.completedAt,
        })
      },
      release: async (...args) => releases.push(args),
    })
    const worker = createAdapterHandoffWorker({
      client,
      targetDomain: 'erp',
      handler: async () => { throw new AdapterHandoffRejectError('invalid_external_order') },
    })
    const result = await worker.runOnce()
    assert.equal(result.status, 'rejected')
    assert.equal(completions[0].errorCode, 'invalid_external_order')
    assert.equal(releases.length, 0)
  })

  await t.test('explicit release', async () => {
    const releases = []
    const client = fakeClient({
      release: async (_issue, reason) => releases.push(reason),
    })
    const worker = createAdapterHandoffWorker({
      client,
      targetDomain: 'erp',
      handler: async () => { throw new AdapterHandoffReleaseError('capacity_pressure') },
    })
    await assert.rejects(() => worker.runOnce(), AdapterHandoffReleaseError)
    assert.deepEqual(releases, ['capacity_pressure'])
  })

  await t.test('outer shutdown', async () => {
    const releases = []
    const client = fakeClient({
      release: async (_issue, reason) => releases.push(reason),
    })
    const worker = createAdapterHandoffWorker({
      client,
      targetDomain: 'erp',
      handler: async (_task, context) => {
        await waitForAbort(context.signal)
        throw abortError()
      },
    })
    const controller = new AbortController()
    setTimeout(() => controller.abort('operator shutdown'), 20)
    await assert.rejects(
      () => worker.runOnce({ signal: controller.signal }),
      (error) => error?.name === 'AbortError',
    )
    assert.deepEqual(releases, ['adapter_shutdown'])
  })
})

function fakeClient(overrides = {}) {
  return {
    claimNext: async () => ({ claimed: true, issue: overrides.issue ?? issue() }),
    renew: overrides.renew ?? (async () => { throw new Error('unexpected renewal') }),
    complete: overrides.complete ?? (async () => { throw new Error('unexpected completion') }),
    release: overrides.release ?? (async () => undefined),
  }
}

function expiringIssue() {
  return issue({
    claim: claim({
      lease_expires_at: new Date(Date.now() + 20).toISOString(),
      lease_deadline_at: new Date(Date.now() + 60_000).toISOString(),
    }),
  })
}

function waitForAbort(signal) {
  if (signal.aborted) return Promise.resolve()
  return new Promise((resolve) => signal.addEventListener('abort', resolve, { once: true }))
}

function abortError() {
  const error = new Error('operation aborted')
  error.name = 'AbortError'
  return error
}

function sleep(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds))
}
