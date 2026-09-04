#!/usr/bin/env node
const { observeCurrency } = require('./esk-sui-currency-observer/observe')
const { OFFICIAL_TESTNET } = require('./esk-sui-currency-observer/contract')

async function main(args) {
  if (args.length === 1 && args[0] === '--help') {
    console.log('Read-only testnet Currency observation; no wallet, signatures or balance updates.')
    console.log('node scripts/observe-esk-sui-currency.js <full-genesis-digest> <package-id> <publish-digest> <registration-digest> <registration-version> <expected-supply-base-units> <second-public-graphql-url>')
    return
  }
  const input = args.length === 7 ? {
    network: 'testnet', chain_identifier: args[0], package_id: args[1],
    publication_digest: args[2], registration_digest: args[3],
    registration_version: args[4], expected_supply_base_units: args[5],
    endpoints: [OFFICIAL_TESTNET, args[6]],
  } : null
  const report = await observeCurrency(input)
  console.log(JSON.stringify(report, null, 2))
  process.exitCode = report.status === 'observed' ? 0 : 1
}

if (require.main === module) main(process.argv.slice(2)).catch(() => {
  console.error('ESK_SUI_CURRENCY_OBSERVER_INTERNAL_ERROR')
  process.exitCode = 1
})

module.exports = { main }
