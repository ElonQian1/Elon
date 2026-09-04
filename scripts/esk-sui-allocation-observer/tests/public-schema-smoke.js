// Opt-in public schema smoke. It uses unrelated public Sui/package facts, never ESK evidence.
const { validateInput, safeCode, OFFICIAL_TESTNET } = require('../contract')
const { readObservation } = require('../graphql')
const { validateObservation } = require('../validation')
const { rawInput } = require('./fixtures')

const SAMPLE = Object.freeze({
  chain: '69WiPg3DAQiwdxfncX6wYQ2siKwAe6L9BZthQea3JNMD',
  package: '0x8f9df445446cb4568136e6a0f6ef69c36d15ce869fca1185660bcd16a616a0e3',
  publication: '52uc677bkdkD858wn6gtYkpmHWf8NQQE8nbHVjbL7Zdn',
  publicationCheckpoint: '379597347',
  publicationCheckpointDigest: '3osfKrPRPs774dY9vX4bTK714DCwPn3dT49BVKqSykVW',
  unrelatedTransaction: '9zetZYiH8yjJdGtRJeb1HTDC64KLuToitsM6rwjw64zR',
  unrelatedCheckpoint: '259084575',
  unrelatedCheckpointDigest: '85EFir8TDFpEWxihS8CW8J1oXGs5wxTXAqDLQ3F1eNtA',
})

function check(condition) {
  if (!condition) throw new Error('PUBLIC_SCHEMA_SMOKE_MISMATCH')
}

function sampleInput() {
  const input = rawInput()
  return validateInput({
    ...input,
    chain_identifier: SAMPLE.chain,
    currency_package_id: '0x2', participation_package_id: SAMPLE.package,
    participation_publication_digest: SAMPLE.publication,
    allocation_digest: SAMPLE.unrelatedTransaction,
    allocation_cap_object_id: '0x3', allocation_receipt_object_id: '0x4',
    team_vesting_object_id: '0x5', initial_supply_coin_object_id: '0x6',
    allocation_checkpoint_sequence: SAMPLE.unrelatedCheckpoint,
    allocation_checkpoint_digest: SAMPLE.unrelatedCheckpointDigest,
    observation_checkpoint_sequence: SAMPLE.publicationCheckpoint,
    observation_checkpoint_digest: SAMPLE.publicationCheckpointDigest,
  })
}

async function main(args) {
  if (args.length !== 1 || args[0] !== '--run-public-non-esk-smoke') {
    console.log('Usage: node scripts/esk-sui-allocation-observer/tests/public-schema-smoke.js --run-public-non-esk-smoke')
    console.log('No network request made. The opt-in sample is unrelated to ESK and never certifies balances or publication.')
    return args.length === 0 || (args.length === 1 && args[0] === '--help') ? 0 : 1
  }
  try {
    const expected = sampleInput()
    const data = await readObservation(OFFICIAL_TESTNET, expected)
    for (const field of [
      'chainIdentifier', 'participationPublicationTransaction', 'participationPackageObject',
      'allocationTransaction', 'observationCheckpoint', 'receiptAtObservation',
      'vestingAtObservation',
    ]) check(Object.hasOwn(data, field))
    let rejection = null
    try { validateObservation(data, expected) } catch (error) { rejection = safeCode(error) }
    check(rejection !== null)
    console.log(JSON.stringify({
      schema: 'yilong.esk.sui.allocation_public_schema_smoke.v1',
      observed_at: new Date().toISOString(), schema_pass: true,
      unrelated_sample_rejected: true, rejection_code: rejection,
      sample_relation: 'UNRELATED_NON_ESK_PACKAGE_AND_SUI_CURRENCY',
      source_count: 1, NO_BALANCE_OR_CERTIFICATION: true,
    }))
    return 0
  } catch (error) {
    console.log(JSON.stringify({ error_code: safeCode(error) }))
    return 1
  }
}

if (require.main === module) main(process.argv.slice(2)).then(code => { process.exitCode = code })

module.exports = { SAMPLE, sampleInput, main }
