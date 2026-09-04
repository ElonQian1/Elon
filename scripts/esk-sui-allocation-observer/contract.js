const publication = require('../esk-sui-publication-observer/contract')

const MAX_U64 = 18446744073709551615n
const MAX_UINT53 = 9007199254740991n
const BUCKET_NAMES = Object.freeze([
  'user_migration_and_ecosystem',
  'team_vesting',
  'project_treasury',
  'liquidity',
  'community_contributors',
  'security_operations_reserve',
])
const ROOT_FIELDS = Object.freeze([
  'network', 'chain_identifier', 'currency_package_id', 'participation_package_id',
  'participation_publication_digest', 'allocation_digest', 'allocation_cap_object_id',
  'allocation_receipt_object_id', 'team_vesting_object_id',
  'initial_supply_coin_object_id', 'allocation_checkpoint_sequence',
  'allocation_checkpoint_digest', 'observation_checkpoint_sequence',
  'observation_checkpoint_digest', 'manifest_digest', 'expected_supply_base_units',
  'holders', 'buckets', 'team_vesting', 'endpoints',
])
const HOLDER_FIELDS = Object.freeze([
  'allocator', 'distribution', 'team_beneficiary', 'treasury', 'liquidity_recipient',
])
const VESTING_FIELDS = Object.freeze(['start_ms', 'cliff_ms', 'end_ms'])
const CODES = new Set([
  'INVALID_INPUT', 'INVALID_RESPONSE', 'PACKAGE_MISMATCH', 'TRANSACTION_MISMATCH',
  'TRANSACTION_NOT_SUCCESSFUL', 'CHECKPOINT_MISSING', 'ALLOCATION_MISMATCH',
  'RECEIPT_MISMATCH', 'CAP_MISMATCH', 'SUPPLY_MISMATCH', 'OUTPUT_SET_MISMATCH',
  'COIN_MISMATCH', 'OWNER_MISMATCH', 'MANIFEST_MISMATCH', 'VESTING_MISMATCH',
  'VERSION_MISMATCH', 'SOURCE_DISAGREEMENT', 'BCS_MISMATCH',
])

class AllocationObservationError extends Error {
  constructor(code) {
    const safe = CODES.has(code) ? code : 'INVALID_RESPONSE'
    super(safe)
    this.code = safe
  }
}

function requireValue(condition, code = 'INVALID_INPUT') {
  if (!condition) throw new AllocationObservationError(code)
}

function exactRecord(value, fields) {
  requireValue(value !== null && typeof value === 'object' && !Array.isArray(value) &&
    Object.keys(value).length === fields.length && fields.every(field => Object.hasOwn(value, field)))
  return value
}

function objectId(value) {
  try { return publication.objectId(value) }
  catch { throw new AllocationObservationError('INVALID_INPUT') }
}

function digest32(value) {
  return publication.digest32(value)
}

function positiveDecimal(value, maximum) {
  requireValue(typeof value === 'string' && value.length <= 20 && /^[1-9][0-9]*$/.test(value))
  requireValue(BigInt(value) <= maximum)
  return value
}

function normalizedIds(input) {
  const ids = {
    currency_package_id: objectId(input.currency_package_id),
    participation_package_id: objectId(input.participation_package_id),
    allocation_cap_object_id: objectId(input.allocation_cap_object_id),
    allocation_receipt_object_id: objectId(input.allocation_receipt_object_id),
    team_vesting_object_id: objectId(input.team_vesting_object_id),
    initial_supply_coin_object_id: objectId(input.initial_supply_coin_object_id),
  }
  requireValue(new Set(Object.values(ids)).size === Object.keys(ids).length)
  return ids
}

function normalizedHolders(value) {
  const input = exactRecord(value, HOLDER_FIELDS)
  const holders = Object.fromEntries(HOLDER_FIELDS.map(field => [field, objectId(input[field])]))
  const destinations = [
    holders.distribution, holders.team_beneficiary, holders.treasury,
    holders.liquidity_recipient,
  ]
  requireValue(new Set(destinations).size === destinations.length)
  return holders
}

function normalizedBuckets(value, supply) {
  const input = exactRecord(value, BUCKET_NAMES)
  const buckets = Object.fromEntries(BUCKET_NAMES.map(name =>
    [name, positiveDecimal(input[name], MAX_U64)]))
  const sum = BUCKET_NAMES.reduce((total, name) => total + BigInt(buckets[name]), 0n)
  requireValue(sum === BigInt(supply))
  return buckets
}

function normalizedVesting(value) {
  const input = exactRecord(value, VESTING_FIELDS)
  const vesting = Object.fromEntries(VESTING_FIELDS.map(field =>
    [field, positiveDecimal(input[field], MAX_U64)]))
  requireValue(BigInt(vesting.start_ms) < BigInt(vesting.cliff_ms) &&
    BigInt(vesting.cliff_ms) < BigInt(vesting.end_ms))
  return vesting
}

function validateInput(input) {
  exactRecord(input, ROOT_FIELDS)
  requireValue(digest32(input.allocation_digest) &&
    digest32(input.allocation_checkpoint_digest) &&
    digest32(input.observation_checkpoint_digest))
  requireValue(input.participation_publication_digest !== input.allocation_digest)
  requireValue(typeof input.manifest_digest === 'string' &&
    /^sha256:[0-9a-f]{64}$/.test(input.manifest_digest) &&
    input.manifest_digest !== `sha256:${'0'.repeat(64)}`)

  const ids = normalizedIds(input)
  const publicationInput = publication.validateInput({
    network: input.network,
    chain_identifier: input.chain_identifier,
    package_id: ids.participation_package_id,
    publication_digest: input.participation_publication_digest,
    endpoints: input.endpoints,
  })
  const allocationSequence = positiveDecimal(input.allocation_checkpoint_sequence, MAX_UINT53)
  const observationSequence = positiveDecimal(input.observation_checkpoint_sequence, MAX_UINT53)
  requireValue(BigInt(observationSequence) >= BigInt(allocationSequence))
  if (observationSequence === allocationSequence) {
    requireValue(input.observation_checkpoint_digest === input.allocation_checkpoint_digest)
  }
  const supply = positiveDecimal(input.expected_supply_base_units, MAX_U64)
  const holders = normalizedHolders(input.holders)
  const buckets = normalizedBuckets(input.buckets, supply)
  const teamVesting = normalizedVesting(input.team_vesting)

  return {
    network: publicationInput.network,
    chain_identifier: publicationInput.chain_identifier,
    currency_package_id: ids.currency_package_id,
    participation_package_id: publicationInput.package_id,
    participation_publication_digest: input.participation_publication_digest,
    allocation_digest: input.allocation_digest,
    allocation_cap_object_id: ids.allocation_cap_object_id,
    allocation_receipt_object_id: ids.allocation_receipt_object_id,
    team_vesting_object_id: ids.team_vesting_object_id,
    initial_supply_coin_object_id: ids.initial_supply_coin_object_id,
    allocation_checkpoint_sequence: allocationSequence,
    allocation_checkpoint_digest: input.allocation_checkpoint_digest,
    observation_checkpoint_sequence: observationSequence,
    observation_checkpoint_digest: input.observation_checkpoint_digest,
    manifest_digest: input.manifest_digest,
    expected_supply_base_units: supply,
    holders,
    buckets,
    team_vesting: teamVesting,
    endpoints: publicationInput.endpoints,
  }
}

function safeCode(error) {
  if (error instanceof AllocationObservationError) return error.code
  return publication.safeCode(error)
}

module.exports = {
  AllocationObservationError, requireValue, validateInput, safeCode, objectId, digest32,
  MAX_U64, MAX_UINT53, BUCKET_NAMES, OFFICIAL_TESTNET: publication.OFFICIAL_TESTNET,
}
