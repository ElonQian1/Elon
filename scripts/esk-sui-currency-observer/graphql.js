const { readGraphql } = require('../esk-sui-publication-observer/transport')

const QUERY = `query CurrencyRegistrationObservation(
  $package: SuiAddress!, $publication: String!, $coinType: String!,
  $currency: SuiAddress!, $registration: String!, $registrationVersion: UInt53!
) {
  chainIdentifier
  publicationTransaction: transaction(digest: $publication) {
    digest
    effects { status checkpoint { sequenceNumber digest } }
  }
  packageObject: object(address: $package) {
    address version digest
    asMovePackage { address version }
    previousTransaction { digest }
  }
  registrationTransaction: transaction(digest: $registration) {
    digest
    effects { status checkpoint { sequenceNumber digest } }
  }
  currentMetadata: coinMetadata(coinType: $coinType) {
    address version digest decimals symbol supply supplyState
    owner { __typename }
    contents { type { repr } }
  }
  registrationObject: object(address: $currency, version: $registrationVersion) {
    address version digest
    owner { __typename }
    previousTransaction { digest }
    asMoveObject {
      contents { type { repr } }
      asCoinMetadata { address version decimals symbol supply supplyState }
    }
    asTransactionObject(transactionDigest: $registration) {
      __typename
      ... on ObjectChange {
        address idCreated idDeleted
        inputState { address version digest }
        outputState { address version digest }
      }
    }
  }
}`

function readObservation(url, input, options) {
  return readGraphql(url, () => ({
    query: QUERY,
    variables: {
      package: input.package_id, publication: input.publication_digest,
      coinType: input.coin_type, currency: input.currency_address,
      registration: input.registration_digest,
      // Input is already bounded to UInt53; only version, never supply, becomes a Number.
      registrationVersion: Number(input.registration_version),
    },
  }), options)
}

module.exports = { QUERY, readObservation }
