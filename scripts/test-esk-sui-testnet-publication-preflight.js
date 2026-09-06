#!/usr/bin/env node
'use strict'

const { spawnSync } = require('node:child_process')
const { join } = require('node:path')

const names = ['contract', 'validation', 'plan', 'cli', 'static']
const files = names.map(name => join(__dirname,
  'esk-sui-testnet-publication-preflight/tests', `${name}.test.js`))
const result = spawnSync(process.execPath,
  ['--test', '--test-reporter=spec', ...files], {
    stdio: 'inherit', timeout: 60000,
  })
process.exitCode = result.status ?? 1
if (result.status === 0) {
  console.log('ESK_SUI_TESTNET_PUBLICATION_PREFLIGHT_TESTS=passed')
  console.log('ESK_SUI_TESTNET_PUBLICATION_NETWORK_REQUESTS=none')
  console.log('ESK_SUI_TESTNET_PUBLICATION_WALLET_OR_PRIVATE_KEY_READS=none')
  console.log('ESK_SUI_TESTNET_PUBLICATION_TRANSACTIONS=not_constructed_or_submitted')
  console.log('ESK_SUI_TESTNET_PUBLICATION_REAL_CHAIN_ACCEPTANCE=not_performed')
}
