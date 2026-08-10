import assert from 'node:assert/strict'
import test from 'node:test'

import {
  SuiPreflightContractError,
  createSuiPreflightClient,
  createSuiPreflightJobClient,
  verifySuiAdapterHandoff,
} from '../src/index.js'
import {
  adapterToken,
  handoff,
  issue,
  job,
  jsonResponse,
  leaseToken,
  report,
} from './sui-preflight-fixtures.mjs'

test('offline handoff verification fail-closes on mutation and submission markers', () => {
  const bundle = handoff()
  const verified = verifySuiAdapterHandoff(bundle)
  assert.equal(verified.handoffDigest, bundle.handoff_digest)
  assert.equal(verified.targetNetwork, 'testnet')

  assert.throws(
    () => verifySuiAdapterHandoff({ ...bundle, source_id: 'mutated-source' }),
    errorCode('handoff_digest_mismatch'),
  )
  assert.throws(
    () => verifySuiAdapterHandoff({ ...bundle, network_submission: 'submitted' }),
    errorCode('invalid_value'),
  )
  assert.throws(
    () => verifySuiAdapterHandoff({ ...bundle, wallet_private_key: 'secret' }),
    errorCode('unexpected_field'),
  )
})

test('report client sends only the verified binding and rejects a swapped report', async () => {
  const bundle = handoff()
  const requests = []
  const client = createSuiPreflightClient({
    baseUrl: 'https://commerce.example.test',
    token: adapterToken,
    fetch: async (url, init) => {
      requests.push({ url: String(url), init, body: JSON.parse(init.body) })
      return jsonResponse(report(bundle))
    },
  })

  const result = await client.report(bundle, {
    outcome: 'passed',
    summary: 'deterministic offline preflight passed',
    toolVersion: 'test-adapter/1.0.0',
    idempotencyKey: 'preflight-test-001',
  })
  assert.equal(result.id, 'preflight-report-1')
  assert.deepEqual(Object.keys(requests[0].body).sort(), [
    'handoff_digest',
    'idempotency_key',
    'outcome',
    'package_kind',
    'projection_package_id',
    'summary',
    'tool_version',
  ])
  assert.equal(requests[0].init.headers.authorization, `Bearer ${adapterToken}`)

  const swapped = createSuiPreflightClient({
    baseUrl: 'https://commerce.example.test',
    token: adapterToken,
    fetch: async () => jsonResponse(report(bundle, { project_id: 'project-other' })),
  })
  await assert.rejects(
    () => swapped.report(bundle, {
      outcome: 'passed',
      summary: 'deterministic offline preflight passed',
      toolVersion: 'test-adapter/1.0.0',
      idempotencyKey: 'preflight-test-001',
    }),
    errorCode('invalid_value'),
  )
})

test('job client validates claim, renew, release, and completion bindings', async () => {
  const bundle = handoff()
  const requests = []
  const client = createSuiPreflightJobClient({
    baseUrl: 'https://commerce.example.test',
    token: adapterToken,
    fetch: async (url, init) => {
      const path = new URL(url).pathname
      const body = JSON.parse(init.body)
      requests.push({ path, body })
      if (path.endsWith('/claim')) {
        return jsonResponse({
          schema: 'task_economy.sui_preflight_job_poll.v1',
          claimed: true,
          issue: issue(bundle),
          retry_after_seconds: 0,
          boundary: [],
        })
      }
      if (path.endsWith('/renew')) {
        return jsonResponse({
          schema: 'task_economy.sui_preflight_job_renew.v1',
          renewed: true,
          job: job(bundle),
          boundary: [],
        })
      }
      if (path.endsWith('/release')) {
        return jsonResponse({
          schema: 'task_economy.sui_preflight_job_release.v1',
          released: true,
          job: job(bundle, {
            status: 'pending',
            lease_expires_at: null,
            lease_deadline_at: null,
          }),
          boundary: [],
        })
      }
      const completedReport = report(bundle)
      return jsonResponse({
        schema: 'task_economy.sui_preflight_job_complete.v1',
        completed: true,
        job: job(bundle, {
          status: 'completed',
          report_id: completedReport.id,
          completed_at: '2026-08-10T00:00:01.000Z',
        }),
        report: completedReport,
        boundary: [],
      })
    },
  })

  const poll = await client.claimNext({ leaseSeconds: 180 })
  await client.renew(poll.issue.job.id, poll.issue.lease_token, { extendSeconds: 600 })
  await client.release(poll.issue.job.id, poll.issue.lease_token, { reason: 'operator pause' })
  const completed = await client.complete(poll.issue.job.id, poll.issue.lease_token, {
    outcome: 'passed',
    summary: 'deterministic offline preflight passed',
    toolVersion: 'test-adapter/1.0.0',
    idempotencyKey: 'preflight-test-001',
  })

  assert.equal(completed.report.id, 'preflight-report-1')
  assert.equal(requests[0].body.lease_seconds, 180)
  assert.equal(requests[1].body.extend_seconds, 600)
  assert.equal(requests.slice(1).every(({ body }) => body.lease_token === leaseToken), true)
  assert.equal(requests.some(({ body }) => 'project_id' in body), false)
})

test('job client rejects idle issue leakage and a swapped transition response', async () => {
  const idleClient = createSuiPreflightJobClient({
    baseUrl: 'https://commerce.example.test',
    token: adapterToken,
    fetch: async () => jsonResponse({
      schema: 'task_economy.sui_preflight_job_poll.v1',
      claimed: false,
      issue: issue(),
      retry_after_seconds: 30,
      boundary: [],
    }),
  })
  await assert.rejects(() => idleClient.claimNext(), errorCode('invalid_job_response'))

  const swappedClient = createSuiPreflightJobClient({
    baseUrl: 'https://commerce.example.test',
    token: adapterToken,
    fetch: async () => jsonResponse({
      schema: 'task_economy.sui_preflight_job_renew.v1',
      renewed: true,
      job: job(handoff(), { id: 'preflight-job-other' }),
      boundary: [],
    }),
  })
  await assert.rejects(
    () => swappedClient.renew('preflight-job-1', leaseToken),
    errorCode('job_binding_mismatch'),
  )
})

function errorCode(code) {
  return (error) => error instanceof SuiPreflightContractError && error.code === code
}
