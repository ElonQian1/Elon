import assert from 'node:assert/strict'
import test from 'node:test'

import {
  SuiPreflightReleaseError,
  createSuiPreflightWorker,
} from '../src/index.js'
import {
  handoff,
  issue,
  job,
  report,
} from './sui-preflight-fixtures.mjs'

test('worker retries completion with one stable idempotency key', async () => {
  const bundle = handoff()
  const claimed = issue(bundle)
  const completions = []
  const releases = []
  const client = fakeClient({
    issue: claimed,
    complete: async (_jobId, _leaseToken, input) => {
      completions.push(input)
      if (completions.length === 1) throw new Error('temporary completion outage')
      return completedResponse(bundle, input)
    },
    release: async (...args) => releases.push(args),
  })
  const observed = []
  const worker = createSuiPreflightWorker({
    client,
    toolVersion: 'test-worker/1.0.0',
    completionAttempts: 2,
    handler: async (_handoff, context) => {
      observed.push(context)
      return { outcome: 'passed', summary: 'offline checks passed' }
    },
  })

  const result = await worker.runOnce()
  assert.equal(result.claimed, true)
  assert.equal(result.status, 'completed')
  assert.equal(completions.length, 2)
  assert.equal(completions[0].idempotencyKey, completions[1].idempotencyKey)
  assert.equal(completions[0].idempotencyKey, observed[0].idempotencyKey)
  assert.match(completions[0].idempotencyKey, /^sui-preflight-[a-f0-9]{40}$/)
  assert.equal(releases.length, 0)
})

test('worker renews an expiring lease before completing', async () => {
  const bundle = handoff()
  const claimed = issue(bundle, {
    job: job(bundle, {
      lease_expires_at: new Date(Date.now() + 20).toISOString(),
      lease_deadline_at: new Date(Date.now() + 60_000).toISOString(),
    }),
  })
  let renewals = 0
  let completedJobId
  const client = fakeClient({
    issue: claimed,
    renew: async () => {
      renewals += 1
      return {
        renewed: true,
        job: job(bundle, {
          lease_expires_at: new Date(Date.now() + 60_000).toISOString(),
          lease_deadline_at: claimed.job.lease_deadline_at,
        }),
      }
    },
    complete: async (jobId, _leaseToken, input) => {
      completedJobId = jobId
      return completedResponse(bundle, input)
    },
  })
  const worker = createSuiPreflightWorker({
    client,
    toolVersion: 'test-worker/1.0.0',
    leaseSeconds: 60,
    renewBeforeSeconds: 10,
    handler: async () => {
      await sleep(350)
      return { outcome: 'passed', summary: 'renewed offline checks passed' }
    },
  })

  const result = await worker.runOnce()
  assert.equal(result.claimed, true)
  assert.equal(renewals, 1)
  assert.equal(completedJobId, claimed.job.id)
})

test('stopping an in-flight renewal after handler success does not become failure', async () => {
  const bundle = handoff()
  const claimed = issue(bundle, {
    job: job(bundle, {
      lease_expires_at: new Date(Date.now() + 20).toISOString(),
      lease_deadline_at: new Date(Date.now() + 60_000).toISOString(),
    }),
  })
  let renewStarted = false
  let completions = 0
  let releases = 0
  const client = fakeClient({
    issue: claimed,
    renew: async (_jobId, _leaseToken, { signal }) => {
      renewStarted = true
      await waitForAbort(signal)
      throw abortError()
    },
    complete: async (_jobId, _leaseToken, input) => {
      completions += 1
      return completedResponse(bundle, input)
    },
    release: async () => { releases += 1 },
  })
  const worker = createSuiPreflightWorker({
    client,
    toolVersion: 'test-worker/1.0.0',
    leaseSeconds: 60,
    renewBeforeSeconds: 10,
    handler: async () => {
      await sleep(320)
      return { outcome: 'passed', summary: 'offline checks passed after renewal race' }
    },
  })

  const result = await worker.runOnce()
  assert.equal(result.claimed, true)
  assert.equal(renewStarted, true)
  assert.equal(completions, 1)
  assert.equal(releases, 0)
})

test('worker abort and explicit release errors release the bounded lease', async (t) => {
  await t.test('outer abort releases once and surfaces AbortError', async () => {
    const bundle = handoff()
    const claimed = issue(bundle)
    const releases = []
    const client = fakeClient({
      issue: claimed,
      release: async (_jobId, _leaseToken, options) => releases.push(options.reason),
    })
    const worker = createSuiPreflightWorker({
      client,
      toolVersion: 'test-worker/1.0.0',
      handler: async (_handoff, context) => {
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
    assert.deepEqual(releases, ['preflight worker shutdown before completion'])
  })

  await t.test('handler can request a bounded explicit release reason', async () => {
    const bundle = handoff()
    const releases = []
    const client = fakeClient({
      issue: issue(bundle),
      release: async (_jobId, _leaseToken, options) => releases.push(options.reason),
    })
    const worker = createSuiPreflightWorker({
      client,
      toolVersion: 'test-worker/1.0.0',
      handler: async () => {
        throw new SuiPreflightReleaseError('local verifier capacity exhausted')
      },
    })
    await assert.rejects(
      () => worker.runOnce(),
      (error) => error instanceof SuiPreflightReleaseError,
    )
    assert.deepEqual(releases, ['local verifier capacity exhausted'])
  })
})

function fakeClient(overrides) {
  return {
    claimNext: async () => ({ claimed: true, issue: overrides.issue }),
    renew: overrides.renew ?? (async () => {
      throw new Error('unexpected renewal')
    }),
    complete: overrides.complete ?? (async () => {
      throw new Error('unexpected completion')
    }),
    release: overrides.release ?? (async () => undefined),
  }
}

function completedResponse(bundle, input) {
  const completedReport = report(bundle, {
    outcome: input.outcome,
    summary: input.summary,
    tool_version: input.toolVersion,
    idempotency_key: input.idempotencyKey,
  })
  return {
    completed: true,
    job: job(bundle, {
      status: 'completed',
      report_id: completedReport.id,
      completed_at: '2026-08-10T00:00:01.000Z',
    }),
    report: completedReport,
  }
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
