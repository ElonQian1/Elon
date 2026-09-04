const { MAX_BYTES, TIMEOUT_MS, resolveAddresses, publicLookup, readGraphql } = require('./transport')

const QUERY = `query PublicationObservation($package: SuiAddress!, $digest: String!) {
  chainIdentifier
  transaction(digest: $digest) {
    digest
    effects { status checkpoint { sequenceNumber digest } }
  }
  object(address: $package) {
    address version digest
    asMovePackage { address version }
    previousTransaction { digest }
  }
}`

function readObservation(url, input, options = {}) {
  return readGraphql(url, () => ({
    query: QUERY, variables: { package: input.package_id, digest: input.publication_digest },
  }), options)
}

module.exports = { QUERY, MAX_BYTES, TIMEOUT_MS, resolveAddresses, publicLookup, readObservation }
