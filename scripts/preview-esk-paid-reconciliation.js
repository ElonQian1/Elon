#!/usr/bin/env node
const { parseInput, MAX_BYTES, InputError } = require('./esk-paid-reconciliation/input')
const { preview, failureReport } = require('./esk-paid-reconciliation/preview')

async function readStandardInput(stream) {
  const chunks = []
  let size = 0
  const timer = setTimeout(() => stream.destroy(new InputError('INPUT_TIMEOUT')), 30000)
  try {
    for await (const chunk of stream) {
      size += chunk.length
      if (size > MAX_BYTES) throw new InputError('INPUT_TOO_LARGE')
      chunks.push(chunk)
    }
    return Buffer.concat(chunks)
  } finally { clearTimeout(timer) }
}

async function main(args) {
  if (args.length === 1 && args[0] === '--help') {
    console.log('Offline consistency preview only. Pipe UTF-8 JSON to stdin; no file paths, credentials or commit mode.')
    return
  }
  let report
  try {
    if (args.length || process.stdin.isTTY) throw new InputError('INVALID_INPUT')
    report = preview(parseInput(await readStandardInput(process.stdin)))
  } catch (error) { report = failureReport(error) }
  console.log(JSON.stringify(report, null, 2))
  process.exitCode = report.status === 'invalid_input' ? 1 : report.status === 'needs_review' ? 2 : 0
}

if (require.main === module) main(process.argv.slice(2)).catch(() => {
  console.error('ESK_RECONCILIATION_INTERNAL_ERROR')
  process.exitCode = 1
})

module.exports = { main, readStandardInput }
