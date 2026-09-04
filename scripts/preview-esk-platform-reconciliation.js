#!/usr/bin/env node
'use strict'

const { parseStrictJson, InputError } = require('./esk-paid-reconciliation/strict-json')
const { readStandardInput } = require('./preview-esk-paid-reconciliation')
const { failure, previewWithPlatformSnapshot } = require('./esk-paid-reconciliation/platform-preview')

async function main(args) {
  if (args.length === 1 && args[0] === '--help') {
    console.log('Offline formal-payment snapshot reconciliation. Pipe strict UTF-8 JSON to stdin; no network, credentials, files or commit mode.')
    return
  }
  let report
  try {
    if (args.length || process.stdin.isTTY) throw new InputError()
    report = previewWithPlatformSnapshot(parseStrictJson(await readStandardInput(process.stdin)))
  } catch (error) { report = failure(error) }
  console.log(JSON.stringify(report, null, 2))
  process.exitCode = report.status === 'invalid_input' ? 1 : report.status === 'needs_review' ? 2 : 0
}

if (require.main === module) main(process.argv.slice(2)).catch(() => {
  console.error('ESK_PLATFORM_RECONCILIATION_INTERNAL_ERROR')
  process.exitCode = 1
})

module.exports = { main }
