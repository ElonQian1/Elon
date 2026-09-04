// Opt-in, read-only schema smoke: unrelated public package and SUI samples, never ESK evidence.
const { QUERY } = require('../graphql')
const { deriveCurrencyId } = require('../canonical')
const { validateObservation } = require('../validation')
const { readGraphql } = require('../../esk-sui-publication-observer/transport')
const { CurrencyObservationError, safeCode, digest32, objectId, OFFICIAL_TESTNET } = require('../contract')

const SAMPLE = Object.freeze({
  chain: '69WiPg3DAQiwdxfncX6wYQ2siKwAe6L9BZthQea3JNMD',
  package: '0x8f9df445446cb4568136e6a0f6ef69c36d15ce869fca1185660bcd16a616a0e3',
  publication: '52uc677bkdkD858wn6gtYkpmHWf8NQQE8nbHVjbL7Zdn',
  packageDigest: 'BZ82uvMxwJ7NLTaBtERQvrEK34reNk484ySikGA48aF1',
  publicationCheckpoint: 379597347,
  publicationCheckpointDigest: '3osfKrPRPs774dY9vX4bTK714DCwPn3dT49BVKqSykVW',
  coinType: '0x2::sui::SUI',
  currency: '0xf256d3fb6a50eaa748d94335b34f2982fbc3b63ceec78cafaa29ebc9ebaf2bbc',
  registration: '9zetZYiH8yjJdGtRJeb1HTDC64KLuToitsM6rwjw64zR',
  version: 634711800,
  digest: 'CVN9P6nwak2s8FM585gZtMS8hT1qVY51S9zCEzUXFX2M',
  registrationCheckpoint: 259084575,
  registrationCheckpointDigest: '85EFir8TDFpEWxihS8CW8J1oXGs5wxTXAqDLQ3F1eNtA',
  supply: '10000000000000000000',
})

function check(value) {
  if (!value) throw new CurrencyObservationError('INVALID_RESPONSE')
}
function shape(value, fields) {
  check(value !== null && typeof value === 'object' && !Array.isArray(value))
  check(Object.keys(value).length === fields.length && fields.every(key => Object.hasOwn(value, key)))
}
function contents(value) {
  shape(value, ['type'])
  shape(value.type, ['repr'])
  const framework = objectId('0x2')
  check(value.type.repr === `${framework}::coin_registry::Currency<${framework}::sui::SUI>`)
}
function reference(value, address, version, digest) {
  check(value.address === address && value.version === version && value.digest === digest)
}
function metadata(value, version) {
  check(value.address === SAMPLE.currency && value.version === version)
  check(value.decimals === 9 && value.symbol === 'SUI' && value.supply === SAMPLE.supply && value.supplyState === 'FIXED')
}
function transaction(value, digest, sequence, checkpointDigest) {
  shape(value, ['digest', 'effects'])
  shape(value.effects, ['status', 'checkpoint'])
  shape(value.effects.checkpoint, ['sequenceNumber', 'digest'])
  check(value.digest === digest && value.effects.status === 'SUCCESS')
  check(value.effects.checkpoint.sequenceNumber === sequence && value.effects.checkpoint.digest === checkpointDigest)
}
function verifyPublicShape(data) {
  shape(data, ['chainIdentifier', 'publicationTransaction', 'packageObject', 'registrationTransaction', 'currentMetadata', 'registrationObject'])
  check(data.chainIdentifier === SAMPLE.chain)
  transaction(data.publicationTransaction, SAMPLE.publication, SAMPLE.publicationCheckpoint, SAMPLE.publicationCheckpointDigest)
  transaction(data.registrationTransaction, SAMPLE.registration, SAMPLE.registrationCheckpoint, SAMPLE.registrationCheckpointDigest)
  const pkg = data.packageObject
  shape(pkg, ['address', 'version', 'digest', 'asMovePackage', 'previousTransaction'])
  reference(pkg, SAMPLE.package, 1, SAMPLE.packageDigest)
  shape(pkg.asMovePackage, ['address', 'version'])
  check(pkg.asMovePackage.address === SAMPLE.package && pkg.asMovePackage.version === 1)
  shape(pkg.previousTransaction, ['digest'])
  check(pkg.previousTransaction.digest === SAMPLE.publication)
  const historical = data.registrationObject
  shape(historical, ['address', 'version', 'digest', 'owner', 'previousTransaction', 'asMoveObject', 'asTransactionObject'])
  reference(historical, SAMPLE.currency, SAMPLE.version, SAMPLE.digest)
  shape(historical.owner, ['__typename'])
  check(historical.owner.__typename === 'Shared')
  shape(historical.previousTransaction, ['digest'])
  check(historical.previousTransaction.digest === SAMPLE.registration)
  shape(historical.asMoveObject, ['contents', 'asCoinMetadata'])
  contents(historical.asMoveObject.contents)
  shape(historical.asMoveObject.asCoinMetadata, ['address', 'version', 'decimals', 'symbol', 'supply', 'supplyState'])
  metadata(historical.asMoveObject.asCoinMetadata, SAMPLE.version)
  const change = historical.asTransactionObject
  shape(change, ['__typename', 'address', 'idCreated', 'idDeleted', 'inputState', 'outputState'])
  check(change.__typename === 'ObjectChange' && change.address === SAMPLE.currency)
  check(change.idCreated === true && change.idDeleted === false && change.inputState === null)
  shape(change.outputState, ['address', 'version', 'digest'])
  reference(change.outputState, SAMPLE.currency, SAMPLE.version, SAMPLE.digest)
  const current = data.currentMetadata
  shape(current, ['address', 'version', 'digest', 'decimals', 'symbol', 'supply', 'supplyState', 'owner', 'contents'])
  check(Number.isSafeInteger(current.version) && current.version >= SAMPLE.version && digest32(current.digest))
  check(current.version !== SAMPLE.version || current.digest === SAMPLE.digest)
  metadata(current, current.version)
  shape(current.owner, ['__typename'])
  check(current.owner.__typename === 'Shared')
  contents(current.contents)
}

async function main(args) {
  if (args.length !== 1 || args[0] !== '--run-public-non-esk-smoke') {
    console.log('Usage: node scripts/esk-sui-currency-observer/tests/public-schema-smoke.js --run-public-non-esk-smoke')
    console.log('No network request made. Opt-in checks two unrelated NON-ESK public samples; NO_BALANCE_OR_CERTIFICATION.')
    return args.length === 0 || (args.length === 1 && args[0] === '--help') ? 0 : 1
  }
  try {
    const currency = await deriveCurrencyId(SAMPLE.coinType)
    check(currency === SAMPLE.currency)
    const data = await readGraphql(OFFICIAL_TESTNET, () => ({ query: QUERY, variables: {
      package: SAMPLE.package, publication: SAMPLE.publication, coinType: SAMPLE.coinType,
      currency, registration: SAMPLE.registration, registrationVersion: SAMPLE.version,
    } }))
    verifyPublicShape(data)
    const coinType = `${SAMPLE.package}::esk::ESK`
    const expected = { network: 'testnet', chain_identifier: SAMPLE.chain, package_id: SAMPLE.package,
      publication_digest: SAMPLE.publication, registration_digest: SAMPLE.registration,
      registration_version: String(SAMPLE.version), expected_supply_base_units: SAMPLE.supply,
      coin_type: coinType, currency_address: await deriveCurrencyId(coinType) }
    let rejection = null
    try { validateObservation(data, expected) } catch (error) { rejection = safeCode(error) }
    // These unrelated samples have incompatible transaction order, before the ESK type check.
    check(rejection === 'REGISTRATION_MISMATCH')
    console.log(JSON.stringify({ schema: 'yilong.esk.sui.public_schema_smoke.v1', observed_at: new Date().toISOString(),
      schema_pass: true, sdk_address_match: true, non_esk_reject: true, rejection_code: rejection,
      sample_relation: 'UNRELATED_NON_ESK_PACKAGE_AND_SUI_CURRENCY', source_count: 1,
      NO_BALANCE_OR_CERTIFICATION: true }))
    return 0
  } catch (error) {
    console.log(JSON.stringify({ error_code: safeCode(error) }))
    return 1
  }
}

if (require.main === module) main(process.argv.slice(2)).then(code => { process.exitCode = code })
module.exports = { main }
