#!/usr/bin/env node
const { spawnSync } = require('node:child_process')
const { join } = require('node:path')

const files = ['contract', 'challenge', 'verify', 'cli', 'json', 'golden', 'static'].map(name =>
  join(__dirname, 'esk-sui-address-binding/tests', `${name}.test.js`))
const guard = join(__dirname, 'esk-sui-address-binding/tests/no-network-guard.js')
const result = spawnSync(process.execPath,
  ['--require', guard, '--test', '--test-reporter=spec', ...files], {
  stdio: 'inherit', timeout: 60000,
  })
process.exitCode = result.status ?? 1
if (result.status === 0) {
  console.log('ESK_SUI_ADDRESS_CONTROL_TESTS=passed')
  console.log('ESK_SUI_ADDRESS_CONTROL_NETWORK_REQUESTS=none')
  console.log('ESK_SUI_ADDRESS_CONTROL_WALLET_OR_PRIVATE_KEY_READS=none')
  console.log('ESK_SUI_ADDRESS_CONTROL_REAL_USER_ACCEPTANCE=not_performed')
}
