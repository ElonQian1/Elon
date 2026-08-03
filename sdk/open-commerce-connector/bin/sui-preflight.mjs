#!/usr/bin/env node

import { readFile } from 'node:fs/promises'
import {
  SuiPreflightContractError,
  createSuiPreflightClient,
  verifySuiAdapterHandoff,
} from '../src/sui-preflight.js'

const MAX_HANDOFF_BYTES = 1024 * 1024

main().catch((error) => {
  const known = error instanceof SuiPreflightContractError
  process.stderr.write(`${known ? error.code : 'unexpected_error'}: ${safeMessage(error)}\n`)
  process.exitCode = known && error.code === 'invalid_usage' ? 3 : known ? 2 : 4
})

async function main() {
  const options = parseArgs(process.argv.slice(2))
  const handoff = await readHandoff(options.handoff)
  const verified = verifySuiAdapterHandoff(handoff)
  if (!options.report) {
    printResult({ mode: 'verify', verified: true, ...verified })
    return
  }
  const token = process.env[options.tokenEnv]
  if (!token) usage(`environment variable ${options.tokenEnv} is required with --report`)
  const client = createSuiPreflightClient({ baseUrl: options.baseUrl, token })
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
  if (content.length > MAX_HANDOFF_BYTES) {
    usage('--handoff file exceeds the 1 MiB limit')
  }
  try {
    return JSON.parse(content.toString('utf8'))
  } catch {
    usage('--handoff file must contain valid JSON')
  }
}

function parseArgs(args) {
  const allowedValues = new Set([
    '--handoff',
    '--base-url',
    '--token-env',
    '--outcome',
    '--summary',
    '--tool-version',
    '--idempotency-key',
  ])
  const values = new Map()
  const flags = new Set()
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index]
    if (argument === '--report') {
      flags.add(argument)
      continue
    }
    if (!allowedValues.has(argument)) usage(`unsupported argument ${argument}`)
    if (!argument.startsWith('--') || index + 1 >= args.length || args[index + 1].startsWith('--')) {
      usage(`invalid argument ${argument}`)
    }
    values.set(argument, args[index + 1])
    index += 1
  }
  const report = flags.has('--report')
  const handoff = values.get('--handoff')
  if (!handoff) usage('--handoff is required')
  const options = {
    handoff,
    report,
    baseUrl: values.get('--base-url'),
    tokenEnv: values.get('--token-env') ?? 'ELON_SUI_PREFLIGHT_TOKEN',
    outcome: values.get('--outcome') ?? 'passed',
    summary: values.get('--summary') ?? 'deterministic offline preflight passed',
    toolVersion: values.get('--tool-version') ?? '@elon/open-commerce-connector/0.1.0',
    idempotencyKey: values.get('--idempotency-key'),
  }
  if (report && (!options.baseUrl || !options.idempotencyKey)) {
    usage('--base-url and --idempotency-key are required with --report')
  }
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(options.tokenEnv)) {
    usage('--token-env must be an environment variable name')
  }
  return options
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
    'usage: sui-preflight --handoff FILE [--report --base-url URL --idempotency-key KEY]\n',
  )
  process.exitCode = 3
  throw new SuiPreflightContractError('invalid_usage', message)
}
