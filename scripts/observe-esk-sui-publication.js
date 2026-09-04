#!/usr/bin/env node
const { observePublication } = require('./esk-sui-publication-observer/observe')

async function main(args) {
  if (args.length === 1 && args[0] === '--help') {
    console.log('Read-only testnet observation; no keys, signatures or balance updates.')
    console.log('node scripts/observe-esk-sui-publication.js <full-genesis-digest> <package-id> <publish-digest> <second-public-graphql-url>')
    return
  }
  // Strict positional arguments prevent accepting wallet/API tokens or arbitrary queries.
  const input = args.length === 4 ? {
    network: 'testnet', chain_identifier: args[0], package_id: args[1],
    publication_digest: args[2],
    endpoints: ['https://graphql.testnet.sui.io/graphql', args[3]],
  } : null
  const report = await observePublication(input)
  console.log(JSON.stringify(report, null, 2))
  process.exitCode = report.status === 'observed' ? 0 : 1
}

if (require.main === module) main(process.argv.slice(2)).catch(() => {
  console.error('ESK_SUI_OBSERVER_INTERNAL_ERROR')
  process.exitCode = 1
})

module.exports = { main }
