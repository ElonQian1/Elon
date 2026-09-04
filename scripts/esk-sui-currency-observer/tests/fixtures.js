// Synthetic domain evidence only: never deployment parameters or an ESK issuance receipt.
const { objectId, OFFICIAL_TESTNET } = require('../../esk-sui-publication-observer/contract')

const DIGEST = '1'.repeat(32)
const REGISTRATION_DIGEST = `${'1'.repeat(31)}2`
const OBJECT_DIGEST = `${'1'.repeat(31)}3`
const CURRENT_DIGEST = `${'1'.repeat(31)}4`
const PACKAGE = objectId('0xa')
// Derived offline with the root's pinned @mysten/sui 2.29.0 for 0xa::esk::ESK.
const CURRENCY = '0x4f3148019769e85a19187ec1635f7f3d957a017ee65e6ed7d6d2798ff2f9b2c5'
const U64_MAX = '18446744073709551615'

function rawInput() {
  return { network: 'testnet', chain_identifier: DIGEST, package_id: PACKAGE,
    publication_digest: DIGEST, registration_digest: REGISTRATION_DIGEST,
    registration_version: '2', expected_supply_base_units: '1000000000',
    endpoints: [OFFICIAL_TESTNET, 'https://reviewed-provider.org/graphql'] }
}

function input() {
  return { ...rawInput(), coin_type: `${PACKAGE}::esk::ESK`, currency_address: CURRENCY }
}

function observation(expected = input()) {
  const version = Number(expected.registration_version)
  const type = { type: { repr: `0x2::coin_registry::Currency<${expected.coin_type}>` } }
  const metadata = { address: expected.currency_address, version, decimals: 6, symbol: 'ESK',
    supply: expected.expected_supply_base_units, supplyState: 'FIXED' }
  return {
    chainIdentifier: expected.chain_identifier,
    publicationTransaction: { digest: expected.publication_digest, effects: { status: 'SUCCESS',
      checkpoint: { sequenceNumber: 100, digest: DIGEST } } },
    packageObject: { address: expected.package_id, version: 1, digest: DIGEST,
      asMovePackage: { address: expected.package_id, version: 1 },
      previousTransaction: { digest: expected.publication_digest } },
    registrationTransaction: { digest: expected.registration_digest, effects: { status: 'SUCCESS',
      checkpoint: { sequenceNumber: 101, digest: REGISTRATION_DIGEST } } },
    currentMetadata: { ...metadata, digest: OBJECT_DIGEST, owner: { __typename: 'Shared' },
      contents: structuredClone(type) },
    registrationObject: { address: expected.currency_address, version, digest: OBJECT_DIGEST,
      owner: { __typename: 'Shared' }, previousTransaction: { digest: expected.registration_digest },
      asMoveObject: { contents: structuredClone(type), asCoinMetadata: metadata },
      asTransactionObject: { __typename: 'ObjectChange', address: expected.currency_address,
        idCreated: true, idDeleted: false, inputState: null,
        outputState: { address: expected.currency_address, version, digest: OBJECT_DIGEST } } },
  }
}

function set(object, path, value) {
  const parts = path.split('.')
  const last = parts.pop()
  const parent = parts.reduce((value, key) => value[key], object)
  if (value === undefined) delete parent[last]
  else parent[last] = value
}

module.exports = { input, rawInput, observation, set, DIGEST, REGISTRATION_DIGEST,
  OBJECT_DIGEST, CURRENT_DIGEST, PACKAGE, CURRENCY, U64_MAX }
