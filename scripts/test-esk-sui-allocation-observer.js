#!/usr/bin/env node
const { spawnSync } = require('node:child_process')
const { join } = require('node:path')

const files = ['contract', 'bcs', 'graphql', 'validation', 'observe'].map(name =>
  join(__dirname, 'esk-sui-allocation-observer/tests', `${name}.test.js`))
const result = spawnSync(process.execPath, ['--test', '--test-reporter=spec', ...files], {
  stdio: 'inherit', timeout: 60000,
})
process.exitCode = result.status ?? 1
if (result.status === 0) {
  console.log('ESK_SUI_ALLOCATION_OBSERVER_TESTS=passed')
  console.log('ESK_SUI_ALLOCATION_NETWORK_WRITES=none')
  console.log('ESK_SUI_ALLOCATION_REAL_ESK_ACCEPTANCE=not_performed')
}
