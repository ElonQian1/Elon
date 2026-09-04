const { readGraphql } = require('../esk-sui-publication-observer/transport')

const QUERY = `query AllocationObservation(
  $participationPackage: SuiAddress!, $participationPublication: String!,
  $allocation: String!, $receipt: SuiAddress!, $vesting: SuiAddress!,
  $observationCheckpoint: UInt53!
) {
  chainIdentifier
  participationPublicationTransaction: transaction(digest: $participationPublication) {
    digest
    effects {
      status effectsDigest lamportVersion
      checkpoint { sequenceNumber digest }
      objectChanges(first: 50) {
        nodes {
          __typename address idCreated idDeleted
          inputState { ...State }
          outputState { ...State }
        }
        pageInfo { hasNextPage hasPreviousPage }
      }
    }
  }
  participationPackageObject: object(address: $participationPackage) {
    address version digest
    asMovePackage { address version }
    previousTransaction { digest }
  }
  allocationTransaction: transaction(digest: $allocation) {
    digest
    sender { address }
    effects {
      status effectsDigest timestamp lamportVersion
      checkpoint { sequenceNumber digest }
      objectChanges(first: 50) {
        nodes {
          __typename address idCreated idDeleted
          inputState { ...State }
          outputState { ...State }
        }
        pageInfo { hasNextPage hasPreviousPage }
      }
    }
  }
  observationCheckpoint: checkpoint(sequenceNumber: $observationCheckpoint) {
    sequenceNumber digest timestamp
  }
  receiptAtObservation: object(address: $receipt, atCheckpoint: $observationCheckpoint) {
    ...State
  }
  vestingAtObservation: object(address: $vesting, atCheckpoint: $observationCheckpoint) {
    ...State
  }
}

fragment State on Object {
  address version digest
  owner {
    __typename
    ... on AddressOwner { address { address } }
  }
  previousTransaction { digest }
  asMoveObject {
    hasPublicTransfer
    contents { type { repr } bcs }
  }
  asMovePackage { address version }
}`

function readObservation(url, input, options) {
  return readGraphql(url, () => ({
    query: QUERY,
    variables: {
      participationPackage: input.participation_package_id,
      participationPublication: input.participation_publication_digest,
      allocation: input.allocation_digest,
      receipt: input.allocation_receipt_object_id,
      vesting: input.team_vesting_object_id,
      observationCheckpoint: Number(input.observation_checkpoint_sequence),
    },
  }), options)
}

module.exports = { QUERY, readObservation }
