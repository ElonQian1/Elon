#!/usr/bin/env node

import { readFile, writeFile } from 'node:fs/promises'
import {
  SuiPreflightContractError,
  createSuiPreflightClient,
  verifySuiAdapterHandoff,
} from '../src/sui-preflight.js'
import { createSuiPreflightJobClient } from '../src/sui-preflight-job-client.js'

const MAX_HANDOFF_BYTES = 1024 * 1024
const ACTIONS = new Set(['verify', 'report', 'claim', 'renew', 'release', 'complete'])

main().catch((error) => {
  const known = error instanceof SuiPreflightContractError
  process.stderr.write(`${known ? error.code : 'unexpected_error'}: ${safeMessage(error)}\n`)
  process.exitCode = known && error.code === 'invalid_usage' ? 3 : known ? 2 : 4
})

async function main() {
  const options = parseArgs(process.argv.slice(2))
  if (options.action === 'verify' || options.action === 'report') {
    await verifyOrReport(options)
    return
  }
  const client = createSuiPreflightJobClient({
    baseUrl: options.baseUrl,
    token: requiredEnvironment(options.tokenEnv),
  })
  if (options.action === 'claim') {
    const poll = await client.claimNext({ leaseSeconds: options.leaseSeconds })
    if (!poll.claimed) {
      printResult({ mode: 'claim', claimed: false, retryAfterSeconds: poll.retry_after_seconds })
      return
    }
    await writeJson(options.output, poll.issue.handoff)
    printResult({
      mode: 'claim',
      claimed: true,
      jobId: poll.issue.job.id,
      leaseToken: poll.issue.lease_token,
      leaseTokenVisibleOnce: true,
      leaseExpiresAt: poll.issue.job.lease_expires_at,
      handoffPath: options.output,
    })
    return
  }
  const leaseToken = requiredEnvironment(options.leaseTokenEnv)
  if (options.action === 'renew') {
    const renewed = await client.renew(options.jobId, leaseToken, {
      extendSeconds: options.extendSeconds,
    })
    printResult({
      mode: 'renew',
      jobId: renewed.job.id,
      leaseExpiresAt: renewed.job.lease_expires_at,
      leaseDeadlineAt: renewed.job.lease_deadline_at,
    })
    return
  }
  if (options.action === 'release') {
    const released = await client.release(options.jobId, leaseToken, {
      reason: options.reason,
    })
    printResult({ mode: 'release', jobId: released.job.id, status: released.job.status })
    return
  }
  const completed = await client.complete(options.jobId, leaseToken, {
    outcome: options.outcome,
    summary: options.summary,
    toolVersion: options.toolVersion,
    idempotencyKey: options.idempotencyKey,
  })
  printResult({
    mode: 'complete',
    jobId: completed.job.id,
    status: completed.job.status,
    reportId: completed.report.id,
    reportDigest: completed.report.report_digest,
    outcome: completed.report.outcome,
  })
}

async function verifyOrReport(options) {
  const handoff = await readHandoff(options.handoff)
  const verified = verifySuiAdapterHandoff(handoff)
  if (options.action === 'verify') {
    printResult({ mode: 'verify', verified: true, ...verified })
    return
  }
  const client = createSuiPreflightClient({
    baseUrl: options.baseUrl,
    token: requiredEnvironment(options.tokenEnv),
  })
  const report = await client.report(handoff, {
    outcome: options.outcome,
    summary: options.summary,
    toolVersion: options.toolVersion,
    idempotencyKey: options.idempotencyKey,
  })
  printResult({
    mode: 'report',
    verified: true,
    reportId: report.id,
    reportDigest: report.report_digest,
    outcome: report.outcome,
  })
}

async function readHandoff(path) {
  let content
  try {
    content = await readFile(path)
  } catch {
    usage('unable to read --handoff file')
  }
  if (content.length > MAX_HANDOFF_BYTES) usage('--handoff file exceeds the 1 MiB limit')
  try {
    return JSON.parse(content.toString('utf8'))
  } catch {
    usage('--handoff file must contain valid JSON')
  }
}

async function writeJson(path, value) {
  try {
    await writeFile(path, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' })
  } catch {
    usage('unable to create --output file; choose a new path')
  }
}

function parseArgs(rawArgs) {
  const args = [...rawArgs]
  let action = 'verify'
  if (args[0] && !args[0].startsWith('--')) {
    action = args.shift()
  }
  const legacyReportIndex = args.indexOf('--report')
  if (legacyReportIndex >= 0) {
    if (action !== 'verify') usage('--report cannot be combined with an action')
    action = 'report'
    args.splice(legacyReportIndex, 1)
  }
  if (!ACTIONS.has(action)) usage(`unsupported action ${action}`)

  const allowedValues = new Set([
    '--handoff', '--output', '--base-url', '--token-env', '--lease-token-env',
    '--job-id', '--lease-seconds', '--extend-seconds', '--outcome', '--summary',
    '--reason', '--tool-version', '--idempotency-key',
  ])
  const values = new Map()
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index]
    if (!allowedValues.has(argument)) usage(`unsupported argument ${argument}`)
    if (index + 1 >= args.length || args[index + 1].startsWith('--')) {
      usage(`invalid argument ${argument}`)
    }
    values.set(argument, args[index + 1])
    index += 1
  }

  const options = {
    action,
    handoff: values.get('--handoff'),
    output: values.get('--output') ?? 'sui-preflight-handoff.json',
    baseUrl: values.get('--base-url'),
    tokenEnv: values.get('--token-env') ?? 'ELON_SUI_PREFLIGHT_TOKEN',
    leaseTokenEnv: values.get('--lease-token-env') ?? 'ELON_SUI_PREFLIGHT_LEASE_TOKEN',
    jobId: values.get('--job-id'),
    leaseSeconds: integerValue(values.get('--lease-seconds') ?? '300', '--lease-seconds'),
    extendSeconds: integerValue(values.get('--extend-seconds') ?? '300', '--extend-seconds'),
    outcome: values.get('--outcome') ?? 'passed',
    summary: values.get('--summary') ?? 'deterministic offline preflight passed',
    reason: values.get('--reason') ?? 'preflight worker released the lease',
    toolVersion: values.get('--tool-version') ?? '@elon/open-commerce-connector/0.1.0',
    idempotencyKey: values.get('--idempotency-key'),
  }
  validateOptions(options)
  return options
}

function validateOptions(options) {
  for (const name of [options.tokenEnv, options.leaseTokenEnv]) {
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
      usage('token environment variable names must be valid')
    }
  }
  if ((options.action === 'verify' || options.action === 'report') && !options.handoff) {
    usage('--handoff is required for verify and report')
  }
  if (options.action !== 'verify' && !options.baseUrl) {
    usage('--base-url is required for network actions')
  }
  if (['renew', 'release', 'complete'].includes(options.action) && !options.jobId) {
    usage('--job-id is required for renew, release, and complete')
  }
  if (['report', 'complete'].includes(options.action) && !options.idempotencyKey) {
    usage('--idempotency-key is required for report and complete')
  }
}

function integerValue(value, name) {
  if (!/^\d+$/.test(value)) usage(`${name} must be an integer`)
  return Number(value)
}

function requiredEnvironment(name) {
  const value = process.env[name]
  if (!value) usage(`environment variable ${name} is required`)
  return value
}

function printResult(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`)
}

function safeMessage(error) {
  return error instanceof Error ? error.message.slice(0, 500) : 'operation failed'
}

function usage(message) {
  process.stderr.write(`${message}\n`)
  process.stderr.write(
    'usage: sui-preflight [verify|report|claim|renew|release|complete] [options]\n',
  )
  process.exitCode = 3
  throw new SuiPreflightContractError('invalid_usage', message)
}
