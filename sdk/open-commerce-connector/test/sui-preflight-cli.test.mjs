import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import test from 'node:test'

import {
  adapterToken,
  handoff,
  issue,
  job,
  leaseToken,
  report,
} from './sui-preflight-fixtures.mjs'

const execute = promisify(execFile)
const cliPath = fileURLToPath(new URL('../bin/sui-preflight.mjs', import.meta.url))

test('CLI verifies a handoff and exercises every offline HTTP command', async () => {
  const bundle = handoff()
  const directory = await mkdtemp(join(tmpdir(), 'sui-preflight-cli-'))
  const handoffPath = join(directory, 'handoff.json')
  const claimedPath = join(directory, 'claimed.json')
  await writeFile(handoffPath, JSON.stringify(bundle), 'utf8')

  const requests = []
  const server = createServer(async (request, response) => {
    const body = await readJsonBody(request)
    requests.push({
      path: request.url,
      authorization: request.headers.authorization,
      body,
    })
    const completedReport = report(bundle)
    if (request.url.endsWith('/reports')) {
      sendJson(response, completedReport)
    } else if (request.url.endsWith('/claim')) {
      sendJson(response, {
        schema: 'task_economy.sui_preflight_job_poll.v1',
        claimed: true,
        issue: issue(bundle),
        retry_after_seconds: 0,
        boundary: [],
      })
    } else if (request.url.endsWith('/renew')) {
      sendJson(response, {
        schema: 'task_economy.sui_preflight_job_renew.v1',
        renewed: true,
        job: job(bundle),
        boundary: [],
      })
    } else if (request.url.endsWith('/release')) {
      sendJson(response, {
        schema: 'task_economy.sui_preflight_job_release.v1',
        released: true,
        job: job(bundle, {
          status: 'pending',
          lease_expires_at: null,
          lease_deadline_at: null,
        }),
        boundary: [],
      })
    } else if (request.url.endsWith('/complete')) {
      sendJson(response, {
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
    } else {
      sendJson(response, { error: 'not found' }, 404)
    }
  })

  try {
    const baseUrl = await listen(server)
    const environment = {
      ELON_SUI_PREFLIGHT_TOKEN: adapterToken,
      ELON_SUI_PREFLIGHT_LEASE_TOKEN: leaseToken,
    }
    const verified = await runCli(['verify', '--handoff', handoffPath], environment)
    assert.equal(verified.mode, 'verify')
    assert.equal(verified.verified, true)

    const reported = await runCli([
      'report',
      '--handoff', handoffPath,
      '--base-url', baseUrl,
      '--tool-version', 'test-adapter/1.0.0',
      '--idempotency-key', 'preflight-test-001',
    ], environment)
    assert.equal(reported.reportId, 'preflight-report-1')

    const claimed = await runCli([
      'claim',
      '--base-url', baseUrl,
      '--lease-seconds', '180',
      '--output', claimedPath,
    ], environment)
    assert.equal(claimed.jobId, 'preflight-job-1')
    assert.equal(claimed.leaseToken, leaseToken)
    assert.deepEqual(JSON.parse(await readFile(claimedPath, 'utf8')), bundle)

    const renewed = await runCli([
      'renew',
      '--base-url', baseUrl,
      '--job-id', 'preflight-job-1',
      '--extend-seconds', '600',
    ], environment)
    assert.equal(renewed.mode, 'renew')

    const released = await runCli([
      'release',
      '--base-url', baseUrl,
      '--job-id', 'preflight-job-1',
      '--reason', 'operator pause',
    ], environment)
    assert.equal(released.status, 'pending')

    const completed = await runCli([
      'complete',
      '--base-url', baseUrl,
      '--job-id', 'preflight-job-1',
      '--tool-version', 'test-adapter/1.0.0',
      '--idempotency-key', 'preflight-test-001',
    ], environment)
    assert.equal(completed.status, 'completed')
    assert.equal(completed.reportId, 'preflight-report-1')

    assert.equal(requests.length, 5)
    assert.equal(
      requests.every(({ authorization }) => authorization === `Bearer ${adapterToken}`),
      true,
    )
    assert.equal(requests[1].body.lease_seconds, 180)
    assert.equal(requests[2].body.extend_seconds, 600)
    assert.equal(requests[3].body.reason, 'operator pause')
    assert.equal(requests.some(({ body }) => 'project_id' in body), false)
  } finally {
    server.closeAllConnections?.()
    await new Promise((resolve) => server.close(resolve))
    await rm(directory, { recursive: true, force: true })
  }
})

test('CLI rejects command-line credentials before making a request', async () => {
  await assert.rejects(
    () => execute(process.execPath, [cliPath, 'claim', '--token', adapterToken], {
      windowsHide: true,
    }),
    (error) => error.code === 3 && error.stderr.includes('unsupported argument --token'),
  )
})

async function runCli(args, environment) {
  const result = await execute(process.execPath, [cliPath, ...args], {
    env: { ...process.env, ...environment },
    windowsHide: true,
  })
  return JSON.parse(result.stdout)
}

async function readJsonBody(request) {
  const chunks = []
  for await (const chunk of request) chunks.push(chunk)
  return JSON.parse(Buffer.concat(chunks).toString('utf8'))
}

function sendJson(response, value, status = 200) {
  response.writeHead(status, { 'content-type': 'application/json' })
  response.end(JSON.stringify(value))
}

function listen(server) {
  return new Promise((resolve, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', () => {
      const address = server.address()
      resolve(`http://127.0.0.1:${address.port}`)
    })
  })
}
