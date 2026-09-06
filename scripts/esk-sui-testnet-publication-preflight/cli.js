'use strict'

const { fail, safeCode } = require('./contract')
const { readCandidateFile } = require('./files')
const { createTemplate, preflightCandidate } = require('./index')

function run(args) {
  if (!Array.isArray(args)) fail('USAGE')
  if (args.length === 1 && args[0] === 'template') return createTemplate()
  if (args.length === 2 && args[0] === 'preflight') {
    return preflightCandidate(readCandidateFile(args[1]))
  }
  fail('USAGE')
}

function main(args, io = { out: console.log, error: console.error }) {
  try {
    io.out(JSON.stringify(run(args), null, 2))
    return 0
  } catch (error) {
    io.error(`ESK_SUI_TESTNET_PUBLICATION_PREFLIGHT_ERROR=${safeCode(error)}`)
    return 1
  }
}

module.exports = { run, main }
