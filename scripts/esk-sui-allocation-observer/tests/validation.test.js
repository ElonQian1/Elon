const test = require('node:test')
const assert = require('node:assert/strict')
const { validateInput, safeCode, objectId } = require('../contract')
const { validateObservation } = require('../validation')
const {
  rawInput, observation, set, address, TYPES, ALLOCATOR, DISTRIBUTION, TEAM,
  TREASURY, CAP, RECEIPT, VESTING, SUPPLY, USER, MANIFEST, ALLOCATION,
} = require('./fixtures')

function expected() { return validateInput(rawInput()) }

function findNode(data, id) {
  return data.allocationTransaction.effects.objectChanges.nodes.find(node =>
    objectId(node.address) === objectId(id))
}

function findPublicationNode(data, id) {
  return data.participationPublicationTransaction.effects.objectChanges.nodes.find(node =>
    objectId(node.address) === objectId(id))
}

function rejected(name, mutate, code) {
  test(name, () => {
    const input = expected()
    const data = observation(input)
    mutate(data, input)
    assert.throws(() => validateObservation(data, input), error => safeCode(error) === code)
  })
}

function replaceU64(base64, offset, value) {
  const bytes = Buffer.from(base64, 'base64')
  bytes.writeBigUInt64LE(BigInt(value), offset)
  return bytes.toString('base64')
}

test('normalizes complete allocation, six buckets and current vesting evidence', () => {
  const input = expected()
  const data = observation(input)
  const before = structuredClone(data)
  const result = validateObservation(data, input)
  assert.deepEqual(data, before)
  assert.equal(result.chain_identifier, input.chain_identifier)
  assert.equal(result.participation_package.package_id, input.participation_package_id)
  assert.equal(result.participation_package.version, '1')
  assert.equal(result.allocation.digest, input.allocation_digest)
  assert.equal(result.allocation.sender, input.holders.allocator)
  assert.deepEqual(result.observation_checkpoint, {
    sequence: input.observation_checkpoint_sequence,
    digest: input.observation_checkpoint_digest,
    timestamp: data.observationCheckpoint.timestamp,
  })
  assert.equal(result.participation_package.publication_lamport_version, '1')
  assert.equal(result.allocation.lamport_version, '10')
  assert.equal(result.manifest_digest, MANIFEST)
  assert.equal(result.cap.object_id, CAP)
  assert.equal(result.receipt.object_id, RECEIPT)
  assert.equal(result.supply_input.object_id, SUPPLY)
  assert.equal(result.supply_input.base_units, '1000')
  assert.deepEqual(Object.keys(result.buckets), Object.keys(input.buckets))
  assert.equal(result.buckets.user_migration_and_ecosystem.change_kind, 'created')
  assert.equal(result.buckets.user_migration_and_ecosystem.owner, DISTRIBUTION)
  assert.equal(result.buckets.team_vesting.object_id, VESTING)
  assert.equal(result.buckets.team_vesting.owner, TEAM)
  assert.equal(result.buckets.security_operations_reserve.object_id, SUPPLY)
  assert.equal(result.buckets.security_operations_reserve.change_kind, 'mutated')
  assert.equal(result.buckets.security_operations_reserve.owner, TREASURY)
  assert.deepEqual({
    total: result.team_vesting_snapshot.total_base_units,
    claimed: result.team_vesting_snapshot.claimed_base_units,
    remaining: result.team_vesting_snapshot.remaining_base_units,
  }, { total: '200', claimed: '50', remaining: '150' })
  for (const value of [result.receipt.bcs_sha256, result.cap.bcs_sha256,
    result.team_vesting_snapshot.bcs_sha256]) assert.match(value, /^sha256:[0-9a-f]{64}$/)
  assert.doesNotMatch(JSON.stringify(result), /base64|contents|private_key|endpoint/i)
})

test('short and uppercase response addresses normalize without changing evidence', () => {
  const input = expected()
  const original = validateObservation(observation(input), input)
  const alias = observation(input)
  alias.allocationTransaction.sender.address = '0x11'
  findPublicationNode(alias, CAP).address = '0x20'
  findPublicationNode(alias, CAP).outputState.address = '0x20'
  alias.receiptAtObservation.address = `0x${RECEIPT.slice(2).toUpperCase()}`
  findNode(alias, USER).outputState.owner.address.address = '0x12'
  findNode(alias, USER).outputState.asMoveObject.contents.type.repr =
    `0x2::coin::Coin<0xC::esk::ESK>`
  assert.deepEqual(validateObservation(alias, input), original)
})

for (const [path, value, code] of [
  ['chainIdentifier', '11111111111111111111111111111113', 'CHAIN_MISMATCH'],
  ['participationPackageObject', null, 'PACKAGE_MISMATCH'],
  ['participationPackageObject.version', 2, 'PACKAGE_MISMATCH'],
  ['participationPackageObject.previousTransaction.digest', ALLOCATION, 'TRANSACTION_MISMATCH'],
  ['participationPublicationTransaction.effects.status', 'FAILURE', 'TRANSACTION_NOT_SUCCESSFUL'],
  ['participationPublicationTransaction.effects.effectsDigest', 'invalid', 'CAP_MISMATCH'],
  ['participationPublicationTransaction.effects.lamportVersion', 0, 'VERSION_MISMATCH'],
  ['participationPublicationTransaction.effects.objectChanges.pageInfo.hasNextPage', true,
    'CAP_MISMATCH'],
  ['allocationTransaction', null, 'ALLOCATION_MISMATCH'],
  ['allocationTransaction.digest', '11111111111111111111111111111117', 'ALLOCATION_MISMATCH'],
  ['allocationTransaction.sender.address', DISTRIBUTION, 'OWNER_MISMATCH'],
  ['allocationTransaction.effects.status', 'FAILURE', 'TRANSACTION_NOT_SUCCESSFUL'],
  ['allocationTransaction.effects.effectsDigest', 'invalid', 'ALLOCATION_MISMATCH'],
  ['allocationTransaction.effects.lamportVersion', 0, 'VERSION_MISMATCH'],
  ['allocationTransaction.effects.timestamp', 'not-time', 'ALLOCATION_MISMATCH'],
  ['allocationTransaction.effects.timestamp', '2033-06-31T00:00:00Z', 'ALLOCATION_MISMATCH'],
  ['allocationTransaction.effects.checkpoint.sequenceNumber', 102, 'CHECKPOINT_MISSING'],
  ['allocationTransaction.effects.checkpoint.digest', 'invalid', 'CHECKPOINT_MISSING'],
  ['allocationTransaction.effects.objectChanges.pageInfo.hasNextPage', true, 'OUTPUT_SET_MISMATCH'],
  ['allocationTransaction.effects.objectChanges.pageInfo.hasPreviousPage', true, 'OUTPUT_SET_MISMATCH'],
  ['observationCheckpoint.sequenceNumber', 111, 'CHECKPOINT_MISSING'],
  ['observationCheckpoint.digest', 'invalid', 'CHECKPOINT_MISSING'],
  ['observationCheckpoint.timestamp', 'invalid', 'CHECKPOINT_MISSING'],
  ['observationCheckpoint.timestamp', '2033-06-31T00:00:00Z', 'CHECKPOINT_MISSING'],
  ['receiptAtObservation', null, 'RECEIPT_MISMATCH'],
  ['vestingAtObservation', null, 'VESTING_MISMATCH'],
]) rejected(`reject ${path}=${String(value)}`, data => set(data, path, value), code)

rejected('receipt must be created immutable, non-transferable and allocation-bound', data => {
  const node = findNode(data, RECEIPT)
  node.idCreated = false
}, 'RECEIPT_MISMATCH')
rejected('receipt immutable owner cannot become an address owner', data => {
  findNode(data, RECEIPT).outputState.owner = {
    __typename: 'AddressOwner', address: { address: ALLOCATOR },
  }
}, 'RECEIPT_MISMATCH')
rejected('receipt cannot expose public transfer', data => {
  findNode(data, RECEIPT).outputState.asMoveObject.hasPublicTransfer = true
}, 'RECEIPT_MISMATCH')
rejected('receipt current digest must remain its immutable creation digest', data => {
  data.receiptAtObservation.digest = '1111111111111111111111111111111Z'
}, 'RECEIPT_MISMATCH')
rejected('receipt BCS manifest must equal the approved digest', data => {
  const bytes = Buffer.from(findNode(data, RECEIPT).outputState.asMoveObject.contents.bcs, 'base64')
  bytes[33] ^= 0xff
  findNode(data, RECEIPT).outputState.asMoveObject.contents.bcs = bytes.toString('base64')
}, 'MANIFEST_MISMATCH')
rejected('receipt BCS rejects trailing bytes', data => {
  const state = findNode(data, RECEIPT).outputState.asMoveObject.contents
  state.bcs = Buffer.concat([Buffer.from(state.bcs, 'base64'), Buffer.from([0])]).toString('base64')
}, 'BCS_MISMATCH')
rejected('receipt current BCS must equal its creation state', data => {
  const bytes = Buffer.from(data.receiptAtObservation.asMoveObject.contents.bcs, 'base64')
  bytes[33] ^= 0xff
  data.receiptAtObservation.asMoveObject.contents.bcs = bytes.toString('base64')
}, 'RECEIPT_MISMATCH')
rejected('receipt result object IDs must be nonzero chain object IDs', data => {
  const contents = findNode(data, RECEIPT).outputState.asMoveObject.contents
  const bytes = Buffer.from(contents.bcs, 'base64')
  bytes.fill(0, 281, 313)
  contents.bcs = bytes.toString('base64')
}, 'RECEIPT_MISMATCH')

rejected('cap must be created by the participation publication', data => {
  findPublicationNode(data, CAP).idCreated = false
}, 'CAP_MISMATCH')
rejected('cap publication output must carry the exact participation type', data => {
  findPublicationNode(data, CAP).outputState.asMoveObject.contents.type.repr =
    TYPES.receipt
}, 'CAP_MISMATCH')
rejected('publication must create exactly one allocation cap', data => {
  const extra = structuredClone(findPublicationNode(data, CAP))
  extra.address = address('0x99')
  extra.outputState.address = extra.address
  data.participationPublicationTransaction.effects.objectChanges.nodes.push(extra)
}, 'CAP_MISMATCH')
rejected('cap publication output version must equal publication lamport version', data => {
  findPublicationNode(data, CAP).outputState.version = 2
}, 'VERSION_MISMATCH')
rejected('publication object changes must all remain complete and classifiable', data => {
  findPublicationNode(data, CAP).outputState.asMoveObject = null
}, 'CAP_MISMATCH')
rejected('cap must be deleted by allocation with no output', data => {
  findNode(data, CAP).idDeleted = false
}, 'CAP_MISMATCH')
rejected('cap allocation input must be held by approved allocator', data => {
  findNode(data, CAP).inputState.owner.address.address = DISTRIBUTION
}, 'OWNER_MISMATCH')
rejected('cap input version cannot reach the allocation lamport version', data => {
  findNode(data, CAP).inputState.version = 10
}, 'VERSION_MISMATCH')
rejected('same-version cap input must equal its publication state', data => {
  findNode(data, CAP).inputState.version = 1
}, 'CAP_MISMATCH')

rejected('full supply input is the mutated security reserve object, not a new coin', data => {
  findNode(data, SUPPLY).idCreated = true
}, 'OUTPUT_SET_MISMATCH')
rejected('supply input must contain the complete fixed amount', data => {
  const state = findNode(data, SUPPLY).inputState.asMoveObject.contents
  state.bcs = replaceU64(state.bcs, 32, '999')
}, 'SUPPLY_MISMATCH')
rejected('security reserve output must contain its exact bucket amount', data => {
  const state = findNode(data, SUPPLY).outputState.asMoveObject.contents
  state.bcs = replaceU64(state.bcs, 32, '51')
}, 'SUPPLY_MISMATCH')
rejected('security reserve output owner must be treasury', data => {
  findNode(data, SUPPLY).outputState.owner.address.address = DISTRIBUTION
}, 'OWNER_MISMATCH')
rejected('security reserve output version must advance to the allocation lamport version', data => {
  findNode(data, SUPPLY).outputState.version = 9
}, 'VERSION_MISMATCH')

rejected('ordinary bucket coin must be created', data => {
  findNode(data, USER).idCreated = false
}, 'COIN_MISMATCH')
rejected('ordinary bucket coin must use the exact ESK type', data => {
  findNode(data, USER).outputState.asMoveObject.contents.type.repr = TYPES.gas
}, 'OUTPUT_SET_MISMATCH')
rejected('ordinary bucket coin must bind its historical recipient', data => {
  findNode(data, USER).outputState.owner.address.address = TREASURY
}, 'OWNER_MISMATCH')
rejected('ordinary bucket coin must bind its historical amount', data => {
  const state = findNode(data, USER).outputState.asMoveObject.contents
  state.bcs = replaceU64(state.bcs, 32, '249')
}, 'COIN_MISMATCH')
rejected('ordinary bucket output version must equal allocation lamport version', data => {
  findNode(data, USER).outputState.version = 9
}, 'VERSION_MISMATCH')

rejected('vesting creation is non-transferable and beneficiary owned', data => {
  findNode(data, VESTING).outputState.asMoveObject.hasPublicTransfer = true
}, 'VESTING_MISMATCH')
rejected('vesting creation starts with no claimed balance', data => {
  const state = findNode(data, VESTING).outputState.asMoveObject.contents
  state.bcs = replaceU64(state.bcs, 72, '1')
}, 'VESTING_MISMATCH')
rejected('vesting current owner remains the fixed beneficiary', data => {
  data.vestingAtObservation.owner.address.address = TREASURY
}, 'OWNER_MISMATCH')
rejected('vesting current claimed plus remaining conserves total', data => {
  const state = data.vestingAtObservation.asMoveObject.contents
  state.bcs = replaceU64(state.bcs, 104, '151')
}, 'VESTING_MISMATCH')
rejected('vesting immutable schedule cannot change in current state', data => {
  const state = data.vestingAtObservation.asMoveObject.contents
  state.bcs = replaceU64(state.bcs, 88, '2100000000001')
}, 'VESTING_MISMATCH')
rejected('vesting current version cannot precede creation', data => {
  data.vestingAtObservation.version = 9
}, 'VESTING_MISMATCH')
rejected('same-version vesting digest must remain the creation digest', data => {
  data.vestingAtObservation.version = 10
}, 'VESTING_MISMATCH')
rejected('same-version vesting BCS must remain the creation BCS', data => {
  const created = findNode(data, VESTING).outputState
  data.vestingAtObservation = structuredClone(created)
  const contents = data.vestingAtObservation.asMoveObject.contents
  contents.bcs = replaceU64(contents.bcs, 72, '1')
  contents.bcs = replaceU64(contents.bcs, 104, '199')
}, 'VESTING_MISMATCH')
rejected('same-version vesting previous transaction must remain allocation', data => {
  data.vestingAtObservation = structuredClone(findNode(data, VESTING).outputState)
  data.vestingAtObservation.previousTransaction.digest = '1111111111111111111111111111111Z'
}, 'VESTING_MISMATCH')

rejected('extra ESK coin object change fails the exact target set', data => {
  const extra = structuredClone(findNode(data, USER))
  extra.address = address('0x99')
  extra.outputState.address = extra.address
  data.allocationTransaction.effects.objectChanges.nodes.push(extra)
}, 'OUTPUT_SET_MISMATCH')
rejected('missing a required target object change fails closed', data => {
  data.allocationTransaction.effects.objectChanges.nodes =
    data.allocationTransaction.effects.objectChanges.nodes.filter(node => node.address !== USER)
}, 'OUTPUT_SET_MISMATCH')
rejected('an unclassifiable extra object change fails closed', data => {
  const extra = structuredClone(findNode(data, USER))
  extra.address = address('0x99')
  extra.outputState.address = extra.address
  extra.outputState.asMoveObject = null
  data.allocationTransaction.effects.objectChanges.nodes.push(extra)
}, 'OUTPUT_SET_MISMATCH')
rejected('a null object change fails closed', data => {
  data.allocationTransaction.effects.objectChanges.nodes.push(null)
}, 'OUTPUT_SET_MISMATCH')
rejected('duplicate object changes fail before target classification', data => {
  data.allocationTransaction.effects.objectChanges.nodes.push(
    structuredClone(findNode(data, USER)))
}, 'OUTPUT_SET_MISMATCH')
rejected('an object change with a missing BCS payload fails closed', data => {
  delete findNode(data, USER).outputState.asMoveObject.contents.bcs
}, 'OUTPUT_SET_MISMATCH')
rejected('observation checkpoint cannot precede the allocation time', data => {
  data.observationCheckpoint.timestamp = new Date(1_999_999_998_000).toISOString()
}, 'CHECKPOINT_MISSING')
rejected('allocation timestamp cannot precede the receipt execution time', data => {
  data.allocationTransaction.effects.timestamp = new Date(1_999_999_998_000).toISOString()
}, 'ALLOCATION_MISMATCH')

test('same allocation and observation checkpoint requires the same digest', () => {
  const input = rawInput()
  input.observation_checkpoint_sequence = input.allocation_checkpoint_sequence
  input.observation_checkpoint_digest = input.allocation_checkpoint_digest
  const normalized = validateInput(input)
  const data = observation(normalized)
  data.observationCheckpoint.timestamp = data.allocationTransaction.effects.timestamp
  assert.equal(validateObservation(data, normalized).observation_checkpoint.sequence,
    input.allocation_checkpoint_sequence)
})

test('malformed roots fail with bounded public error codes', () => {
  for (const value of [null, [], 'secret', 1]) {
    assert.throws(() => validateObservation(value, expected()), error =>
      safeCode(error) === 'INVALID_RESPONSE')
  }
})
