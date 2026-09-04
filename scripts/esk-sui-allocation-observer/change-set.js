const { requireValue, digest32 } = require('./contract')
const {
  record, sameAddress, responseObjectId, uint53, coinType, receiptType,
  vestingType, capType, objectChange, created, deleted,
} = require('./state')

function completeState(value, expectedAddress, code) {
  const state = record(value, code)
  sameAddress(state.address, expectedAddress, code)
  const version = uint53(state.version, code, true)
  requireValue(digest32(state.digest), code)
  const previous = record(state.previousTransaction, code)
  requireValue(digest32(previous.digest), code)
  const owner = record(state.owner, code)
  requireValue(typeof owner.__typename === 'string' && owner.__typename.length > 0, code)
  requireValue(Object.hasOwn(state, 'asMoveObject') && Object.hasOwn(state, 'asMovePackage'), code)
  const isObject = state.asMoveObject !== null
  const isPackage = state.asMovePackage !== null
  requireValue(isObject !== isPackage, code)
  if (isObject) {
    const move = record(state.asMoveObject, code)
    requireValue(typeof move.hasPublicTransfer === 'boolean', code)
    const contents = record(move.contents, code)
    const type = record(contents.type, code)
    requireValue(typeof type.repr === 'string' && type.repr.length > 0 &&
      type.repr.length <= 320 && typeof contents.bcs === 'string', code)
    return { kind: 'move_object', version, type: type.repr }
  }
  const pkg = record(state.asMovePackage, code)
  sameAddress(pkg.address, expectedAddress, code)
  requireValue(uint53(pkg.version, code, true) === version, code)
  return { kind: 'move_package', version, type: null }
}

function completeChange(value, code) {
  const raw = record(value, code)
  const address = responseObjectId(raw.address, code)
  const change = objectChange(raw, address, code)
  requireValue(!(change.idCreated && change.idDeleted), code)
  if (change.idCreated) created(change, code)
  else if (change.idDeleted) deleted(change, code)
  else requireValue(change.inputState !== null || change.outputState !== null, code)
  const input = change.inputState === null ? null : completeState(change.inputState, address, code)
  const output = change.outputState === null ? null : completeState(change.outputState, address, code)
  return { raw: change, address, input, output }
}

function completeConnection(value, code) {
  const connection = record(value, code)
  const page = record(connection.pageInfo, code)
  requireValue(page.hasNextPage === false && page.hasPreviousPage === false, code)
  requireValue(Array.isArray(connection.nodes) && connection.nodes.length > 0 &&
    connection.nodes.length <= 50, code)
  const changes = connection.nodes.map(node => completeChange(node, code))
  requireValue(new Set(changes.map(change => change.address)).size === changes.length, code)
  return changes
}

function typeMatches(value, validator) {
  if (value === null) return false
  try { validator(value); return true } catch { return false }
}

function uniqueCreatedType(changes, validator, expectedId, code) {
  const matches = changes.filter(change =>
    typeMatches(change.input?.type, validator) || typeMatches(change.output?.type, validator))
  requireValue(matches.length === 1 && matches[0].address === expectedId, code)
  created(matches[0].raw, code)
  return matches[0].raw
}

function targetSetEvidence(changes, receipt, expected, directBuckets) {
  const directIds = directBuckets.map(([, field]) =>
    responseObjectId(receipt[field], 'RECEIPT_MISMATCH'))
  const ids = [
    expected.allocation_cap_object_id, expected.allocation_receipt_object_id,
    expected.team_vesting_object_id, expected.initial_supply_coin_object_id,
    ...directIds,
  ]
  requireValue(new Set(ids).size === ids.length, 'OUTPUT_SET_MISMATCH')
  const validators = [
    value => coinType(value, expected.currency_package_id),
    value => receiptType(value, expected.participation_package_id),
    value => vestingType(value, expected.participation_package_id),
    value => capType(value, expected.participation_package_id),
  ]
  const targets = changes.filter(change => [change.input?.type, change.output?.type]
    .some(type => validators.some(validate => typeMatches(type, validate))))
  const observed = targets.map(change => change.address)
  requireValue(observed.length === ids.length && new Set(observed).size === observed.length &&
    ids.every(id => observed.includes(id)), 'OUTPUT_SET_MISMATCH')
}

module.exports = { completeConnection, uniqueCreatedType, targetSetEvidence }
