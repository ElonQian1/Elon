const { validateObservation: validatePublication } = require('../esk-sui-publication-observer/observe')
const {
  AllocationObservationError, requireValue, objectId, digest32, BUCKET_NAMES,
} = require('./contract')
const { decodeReceipt, decodeVesting, decodeCoin, decodeCap } = require('./bcs')
const {
  record, sameAddress, uint53, timestamp, coinType, receiptType, vestingType, capType,
  responseObjectId, ownerAddress, immutableOwner, moveState,
  created, deleted, mutated, findChange,
} = require('./state')
const { completeConnection, uniqueCreatedType, targetSetEvidence } = require('./change-set')
const { formatEvidence } = require('./evidence')

const DIRECT_BUCKETS = Object.freeze([
  ['user_migration_and_ecosystem', 'user_migration_and_ecosystem_coin_id', 'distribution'],
  ['project_treasury', 'project_treasury_coin_id', 'treasury'],
  ['liquidity', 'liquidity_coin_id', 'liquidity_recipient'],
  ['community_contributors', 'community_contributors_coin_id', 'distribution'],
])

function requirePrevious(state, digest, code) {
  requireValue(state.previous_transaction === digest, code)
}

function addressOwnerValue(owner, code) {
  record(owner, code)
  requireValue(owner.__typename === 'AddressOwner', code)
  record(owner.address, code)
  try { return objectId(owner.address.address) }
  catch { throw new AllocationObservationError(code) }
}

function stateFromChange(change, side, id, typeValidator, code) {
  return moveState(change[side], id, typeValidator, code)
}

function publicationEvidence(data, expected) {
  const publicationExpected = {
    chain_identifier: expected.chain_identifier,
    package_id: expected.participation_package_id,
    publication_digest: expected.participation_publication_digest,
  }
  const evidence = validatePublication({
    chainIdentifier: data.chainIdentifier,
    transaction: data.participationPublicationTransaction,
    object: data.participationPackageObject,
  }, publicationExpected)
  requireValue(evidence.package_version === '1', 'PACKAGE_MISMATCH')
  const effects = record(data.participationPublicationTransaction.effects, 'CAP_MISMATCH')
  requireValue(digest32(effects.effectsDigest), 'CAP_MISMATCH')
  const lamportVersion = uint53(effects.lamportVersion, 'VERSION_MISMATCH', true)
  const changes = completeConnection(effects.objectChanges, 'CAP_MISMATCH')
  return { ...evidence, effects_digest: effects.effectsDigest,
    lamport_version: lamportVersion, changes }
}

function transactionEvidence(data, expected, publication) {
  const transaction = record(data.allocationTransaction, 'ALLOCATION_MISMATCH')
  requireValue(transaction.digest === expected.allocation_digest, 'ALLOCATION_MISMATCH')
  const sender = record(transaction.sender, 'OWNER_MISMATCH')
  sameAddress(sender.address, expected.holders.allocator, 'OWNER_MISMATCH')
  const effects = record(transaction.effects, 'ALLOCATION_MISMATCH')
  requireValue(effects.status === 'SUCCESS', 'TRANSACTION_NOT_SUCCESSFUL')
  requireValue(digest32(effects.effectsDigest), 'ALLOCATION_MISMATCH')
  const lamportVersion = uint53(effects.lamportVersion, 'VERSION_MISMATCH', true)
  const observedTime = timestamp(effects.timestamp, 'ALLOCATION_MISMATCH')
  const checkpoint = record(effects.checkpoint, 'CHECKPOINT_MISSING')
  requireValue(uint53(checkpoint.sequenceNumber, 'CHECKPOINT_MISSING') ===
    expected.allocation_checkpoint_sequence, 'CHECKPOINT_MISSING')
  requireValue(checkpoint.digest === expected.allocation_checkpoint_digest &&
    digest32(checkpoint.digest), 'CHECKPOINT_MISSING')
  const publicationOrder = BigInt(expected.allocation_checkpoint_sequence) -
    BigInt(publication.checkpoint_sequence)
  requireValue(publicationOrder >= 0n && (publicationOrder !== 0n ||
    checkpoint.digest === publication.checkpoint_digest), 'ALLOCATION_MISMATCH')
  const changes = completeConnection(effects.objectChanges, 'OUTPUT_SET_MISMATCH')
  return {
    sender: expected.holders.allocator, effects_digest: effects.effectsDigest,
    timestamp: observedTime, lamport_version: lamportVersion,
    changes, nodes: changes.map(change => change.raw),
  }
}

function checkpointEvidence(data, expected, allocationTime) {
  const checkpoint = record(data.observationCheckpoint, 'CHECKPOINT_MISSING')
  requireValue(uint53(checkpoint.sequenceNumber, 'CHECKPOINT_MISSING') ===
    expected.observation_checkpoint_sequence, 'CHECKPOINT_MISSING')
  requireValue(checkpoint.digest === expected.observation_checkpoint_digest &&
    digest32(checkpoint.digest), 'CHECKPOINT_MISSING')
  requireValue(BigInt(expected.observation_checkpoint_sequence) >=
    BigInt(expected.allocation_checkpoint_sequence), 'CHECKPOINT_MISSING')
  if (expected.observation_checkpoint_sequence === expected.allocation_checkpoint_sequence) {
    requireValue(expected.observation_checkpoint_digest ===
      expected.allocation_checkpoint_digest, 'CHECKPOINT_MISSING')
  }
  const observedTime = timestamp(checkpoint.timestamp, 'CHECKPOINT_MISSING')
  requireValue(BigInt(observedTime.milliseconds) >= BigInt(allocationTime.milliseconds),
    'CHECKPOINT_MISSING')
  if (expected.observation_checkpoint_sequence === expected.allocation_checkpoint_sequence) {
    requireValue(observedTime.milliseconds === allocationTime.milliseconds, 'CHECKPOINT_MISSING')
  }
  return { sequence: expected.observation_checkpoint_sequence,
    digest: expected.observation_checkpoint_digest, timestamp: observedTime }
}

function receiptEvidence(data, nodes, expected, allocation, observationCheckpoint) {
  const change = findChange(nodes, expected.allocation_receipt_object_id, 'RECEIPT_MISMATCH')
  created(change, 'RECEIPT_MISMATCH')
  const validateType = value => receiptType(value, expected.participation_package_id)
  const creation = stateFromChange(change, 'outputState', expected.allocation_receipt_object_id,
    validateType, 'RECEIPT_MISMATCH')
  requireValue(creation.version === allocation.lamport_version, 'VERSION_MISMATCH')
  requirePrevious(creation, expected.allocation_digest, 'RECEIPT_MISMATCH')
  immutableOwner(creation.owner, 'RECEIPT_MISMATCH')
  requireValue(creation.has_public_transfer === false, 'RECEIPT_MISMATCH')
  const receipt = decodeReceipt(creation.bcs)
  sameAddress(receipt.id, expected.allocation_receipt_object_id, 'RECEIPT_MISMATCH')
  requireValue(receipt.manifest_digest === expected.manifest_digest, 'MANIFEST_MISMATCH')
  requireValue(receipt.total_base_units === expected.expected_supply_base_units,
    'SUPPLY_MISMATCH')
  for (const field of ['distribution', 'team_beneficiary', 'treasury', 'liquidity_recipient']) {
    sameAddress(receipt[field], expected.holders[field], 'OWNER_MISMATCH')
  }
  for (const bucket of BUCKET_NAMES) {
    requireValue(receipt[`${bucket}_units`] === expected.buckets[bucket],
      'ALLOCATION_MISMATCH')
  }
  for (const field of ['start_ms', 'cliff_ms', 'end_ms']) {
    requireValue(receipt[field] === expected.team_vesting[field], 'VESTING_MISMATCH')
  }
  requireValue(BigInt(receipt.executed_at_ms) <= BigInt(receipt.start_ms) &&
    BigInt(receipt.executed_at_ms) <= BigInt(allocation.timestamp.milliseconds) &&
    BigInt(receipt.executed_at_ms) <= BigInt(observationCheckpoint.timestamp.milliseconds),
  'ALLOCATION_MISMATCH')
  sameAddress(receipt.team_vesting_id, expected.team_vesting_object_id, 'VESTING_MISMATCH')
  sameAddress(receipt.security_operations_reserve_coin_id,
    expected.initial_supply_coin_object_id, 'SUPPLY_MISMATCH')

  const current = moveState(data.receiptAtObservation,
    expected.allocation_receipt_object_id, validateType, 'RECEIPT_MISMATCH')
  requirePrevious(current, expected.allocation_digest, 'RECEIPT_MISMATCH')
  immutableOwner(current.owner, 'RECEIPT_MISMATCH')
  requireValue(current.has_public_transfer === false && current.version === creation.version &&
    current.digest === creation.digest, 'RECEIPT_MISMATCH')
  const currentReceipt = decodeReceipt(current.bcs)
  requireValue(JSON.stringify(currentReceipt) === JSON.stringify(receipt), 'RECEIPT_MISMATCH')
  return { receipt, creation }
}

function capEvidence(publication, nodes, expected, allocationLamport) {
  const validateType = value => capType(value, expected.participation_package_id)
  const publicationChange = uniqueCreatedType(publication.changes, validateType,
    expected.allocation_cap_object_id, 'CAP_MISMATCH')
  const published = stateFromChange(publicationChange, 'outputState',
    expected.allocation_cap_object_id, validateType, 'CAP_MISMATCH')
  requireValue(published.version === publication.lamport_version, 'VERSION_MISMATCH')
  requirePrevious(published, expected.participation_publication_digest, 'CAP_MISMATCH')
  requireValue(published.has_public_transfer === true, 'CAP_MISMATCH')
  const publishedOwner = addressOwnerValue(published.owner, 'CAP_MISMATCH')
  const publishedCap = decodeCap(published.bcs)
  sameAddress(publishedCap.id, expected.allocation_cap_object_id, 'CAP_MISMATCH')

  const change = findChange(nodes, expected.allocation_cap_object_id, 'CAP_MISMATCH')
  deleted(change, 'CAP_MISMATCH')
  const consumed = stateFromChange(change, 'inputState', expected.allocation_cap_object_id,
    validateType, 'CAP_MISMATCH')
  ownerAddress(consumed.owner, expected.holders.allocator, 'OWNER_MISMATCH')
  requireValue(consumed.has_public_transfer === true, 'CAP_MISMATCH')
  const consumedCap = decodeCap(consumed.bcs)
  sameAddress(consumedCap.id, expected.allocation_cap_object_id, 'CAP_MISMATCH')
  requireValue(BigInt(consumed.version) >= BigInt(published.version) &&
    BigInt(consumed.version) < BigInt(allocationLamport), 'VERSION_MISMATCH')
  if (consumed.version === published.version) {
    requireValue(consumed.digest === published.digest && consumed.bcs === published.bcs &&
      consumed.previous_transaction === published.previous_transaction &&
      publishedOwner === expected.holders.allocator, 'CAP_MISMATCH')
  }
  return { published, publishedOwner, publishedCap, consumed, consumedCap }
}

function coinState(state, id, owner, amount, expected, code = 'COIN_MISMATCH') {
  const parsed = moveState(state, id,
    value => coinType(value, expected.currency_package_id, code), code)
  ownerAddress(parsed.owner, owner, 'OWNER_MISMATCH')
  requireValue(parsed.has_public_transfer === true, code)
  const coin = decodeCoin(parsed.bcs)
  sameAddress(coin.id, id, code)
  requireValue(coin.balance === amount, code)
  return { ...parsed, coin }
}

function directCoinEvidence(nodes, receipt, expected, allocationLamport) {
  const evidence = {}
  for (const [bucket, idField, holder] of DIRECT_BUCKETS) {
    const id = responseObjectId(receipt[idField], 'RECEIPT_MISMATCH')
    const change = findChange(nodes, id, 'COIN_MISMATCH')
    created(change, 'COIN_MISMATCH')
    const output = coinState(change.outputState, id, expected.holders[holder],
      expected.buckets[bucket], expected)
    requireValue(output.version === allocationLamport, 'VERSION_MISMATCH')
    requirePrevious(output, expected.allocation_digest, 'COIN_MISMATCH')
    evidence[bucket] = { id, output }
  }
  return evidence
}

function supplyEvidence(nodes, expected, allocationLamport) {
  const change = findChange(nodes, expected.initial_supply_coin_object_id, 'SUPPLY_MISMATCH')
  mutated(change, 'SUPPLY_MISMATCH')
  const input = coinState(change.inputState, expected.initial_supply_coin_object_id,
    expected.holders.allocator, expected.expected_supply_base_units, expected, 'SUPPLY_MISMATCH')
  const output = coinState(change.outputState, expected.initial_supply_coin_object_id,
    expected.holders.treasury, expected.buckets.security_operations_reserve,
    expected, 'SUPPLY_MISMATCH')
  requirePrevious(output, expected.allocation_digest, 'SUPPLY_MISMATCH')
  requireValue(BigInt(input.version) < BigInt(output.version) &&
    output.version === allocationLamport, 'VERSION_MISMATCH')
  return { input, output }
}

function vestingEvidence(data, nodes, receipt, expected, allocationLamport) {
  const change = findChange(nodes, expected.team_vesting_object_id, 'VESTING_MISMATCH')
  created(change, 'VESTING_MISMATCH')
  const validateType = value => vestingType(value, expected.participation_package_id)
  const creation = stateFromChange(change, 'outputState', expected.team_vesting_object_id,
    validateType, 'VESTING_MISMATCH')
  requireValue(creation.version === allocationLamport, 'VERSION_MISMATCH')
  requirePrevious(creation, expected.allocation_digest, 'VESTING_MISMATCH')
  ownerAddress(creation.owner, expected.holders.team_beneficiary, 'OWNER_MISMATCH')
  requireValue(creation.has_public_transfer === false, 'VESTING_MISMATCH')
  const initial = decodeVesting(creation.bcs)
  validateVesting(initial, expected, true)

  const current = moveState(data.vestingAtObservation, expected.team_vesting_object_id,
    validateType, 'VESTING_MISMATCH')
  ownerAddress(current.owner, expected.holders.team_beneficiary, 'OWNER_MISMATCH')
  requireValue(current.has_public_transfer === false &&
    BigInt(current.version) >= BigInt(creation.version), 'VESTING_MISMATCH')
  if (current.version === creation.version) {
    requireValue(current.digest === creation.digest &&
      current.previous_transaction === expected.allocation_digest, 'VESTING_MISMATCH')
  }
  const snapshot = decodeVesting(current.bcs)
  validateVesting(snapshot, expected, false)
  if (current.version === creation.version) {
    requireValue(current.bcs === creation.bcs, 'VESTING_MISMATCH')
  }
  for (const field of ['id', 'beneficiary', 'total_base_units', 'start_ms', 'cliff_ms', 'end_ms']) {
    requireValue(snapshot[field] === initial[field], 'VESTING_MISMATCH')
  }
  requireValue(receipt.team_vesting_units === snapshot.total_base_units, 'VESTING_MISMATCH')
  return { creation, initial, current, snapshot }
}

function validateVesting(value, expected, initial) {
  sameAddress(value.id, expected.team_vesting_object_id, 'VESTING_MISMATCH')
  sameAddress(value.beneficiary, expected.holders.team_beneficiary, 'VESTING_MISMATCH')
  requireValue(value.total_base_units === expected.buckets.team_vesting &&
    value.start_ms === expected.team_vesting.start_ms &&
    value.cliff_ms === expected.team_vesting.cliff_ms &&
    value.end_ms === expected.team_vesting.end_ms, 'VESTING_MISMATCH')
  const claimed = BigInt(value.claimed_base_units)
  const remaining = BigInt(value.remaining_base_units)
  const total = BigInt(value.total_base_units)
  requireValue(claimed <= total && claimed + remaining === total, 'VESTING_MISMATCH')
  if (initial) requireValue(claimed === 0n && remaining === total, 'VESTING_MISMATCH')
}

/** Validate one fixed GraphQL response and return deterministic public evidence. */
function validateObservation(data, expected) {
  record(data, 'INVALID_RESPONSE')
  const publication = publicationEvidence(data, expected)
  const allocation = transactionEvidence(data, expected, publication)
  const observationCheckpoint = checkpointEvidence(data, expected, allocation.timestamp)
  const { receipt, creation: receiptCreation } = receiptEvidence(
    data, allocation.nodes, expected, allocation, observationCheckpoint)
  targetSetEvidence(allocation.changes, receipt, expected, DIRECT_BUCKETS)
  const cap = capEvidence(publication, allocation.nodes, expected, allocation.lamport_version)
  const supply = supplyEvidence(allocation.nodes, expected, allocation.lamport_version)
  const direct = directCoinEvidence(allocation.nodes, receipt, expected,
    allocation.lamport_version)
  const vesting = vestingEvidence(data, allocation.nodes, receipt, expected,
    allocation.lamport_version)

  return formatEvidence({ expected, publication, allocation,
    checkpoint: observationCheckpoint, receipt, receiptCreation, cap, supply, direct, vesting })
}

module.exports = { validateObservation, DIRECT_BUCKETS }
