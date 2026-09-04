#!/usr/bin/env node
const { lstatSync, readFileSync } = require('node:fs')
const { TextDecoder } = require('node:util')
const { observeAllocation } = require('./esk-sui-allocation-observer/observe')

const MAX_INPUT_BYTES = 64 * 1024

function readInput(path) {
  if (typeof path !== 'string' || path.length < 1 || path.length > 500 || path.includes('\0')) {
    return null
  }
  try {
    const stat = lstatSync(path)
    if (!stat.isFile() || stat.isSymbolicLink() || stat.size < 2 || stat.size > MAX_INPUT_BYTES) {
      return null
    }
    const source = new TextDecoder('utf-8', { fatal: true }).decode(readFileSync(path))
    const value = JSON.parse(source)
    return value && typeof value === 'object' && !Array.isArray(value) ? value : null
  } catch { return null }
}

async function main(args) {
  if (args.length === 1 && args[0] === '--help') {
    console.log('Read-only ESK Sui testnet allocation observation; no wallet, signing, broadcast or balance update.')
    console.log('node scripts/observe-esk-sui-allocation.js <public-observation-input.json>')
    return 0
  }
  const input = args.length === 1 ? readInput(args[0]) : null
  const report = await observeAllocation(input)
  console.log(JSON.stringify(report, null, 2))
  return report.status === 'observed' ? 0 : 1
}

if (require.main === module) main(process.argv.slice(2)).then(code => {
  process.exitCode = code
}).catch(() => {
  console.error('ESK_SUI_ALLOCATION_OBSERVER_INTERNAL_ERROR')
  process.exitCode = 1
})

module.exports = { MAX_INPUT_BYTES, readInput, main }
