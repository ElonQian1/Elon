const {
  AllocationObservationError, requireValue, objectId, digest32,
} = require('./contract')

function record(value, code = 'INVALID_RESPONSE') {
  requireValue(value !== null && typeof value === 'object' && !Array.isArray(value), code)
  return value
}

function sameAddress(value, expected, code) {
  try { requireValue(objectId(value) === expected, code) }
  catch { throw new AllocationObservationError(code) }
}

function uint53(value, code, positive = false) {
  requireValue(typeof value === 'number' && Number.isSafeInteger(value) &&
    value >= (positive ? 1 : 0), code)
  return String(value)
}

function timestamp(value, code) {
  requireValue(typeof value === 'string' && value.length <= 40, code)
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})(?:\.(\d{1,9}))?Z$/.exec(value)
  requireValue(match, code)
  const [year, month, day, hour, minute, second] = match.slice(1, 7).map(Number)
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0)
  const monthDays = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
  requireValue(year >= 1970 && month >= 1 && month <= 12 && day >= 1 &&
    day <= monthDays[month - 1] && hour <= 23 && minute <= 59 && second <= 59, code)
  const milliseconds = Date.parse(value)
  requireValue(Number.isSafeInteger(milliseconds) && milliseconds >= 0, code)
  return { value, milliseconds: String(milliseconds) }
}

function responseObjectId(value, code) {
  try {
    const normalized = objectId(value)
    if (/^0x0{64}$/.test(normalized)) throw new Error('zero object id')
    return normalized
  }
  catch { throw new AllocationObservationError(code) }
}

function exactStructType(value, packageId, suffix, code) {
  requireValue(typeof value === 'string' && value.length <= 260, code)
  const match = /^(0x[0-9a-fA-F]{1,64})(::[A-Za-z_][A-Za-z0-9_]*::[A-Za-z_][A-Za-z0-9_]*)$/.exec(value)
  requireValue(match && match[2] === suffix, code)
  sameAddress(match[1], packageId, code)
}

function coinType(value, currencyPackageId, code = 'COIN_MISMATCH') {
  requireValue(typeof value === 'string' && value.length <= 320, code)
  const match = /^(0x[0-9a-fA-F]{1,64})::coin::Coin<(0x[0-9a-fA-F]{1,64})::esk::ESK>$/.exec(value)
  requireValue(match, code)
  sameAddress(match[1], objectId('0x2'), code)
  sameAddress(match[2], currencyPackageId, code)
}

function receiptType(value, participationPackageId, code = 'RECEIPT_MISMATCH') {
  exactStructType(value, participationPackageId,
    '::genesis_allocation::GenesisAllocationReceipt', code)
}

function vestingType(value, participationPackageId, code = 'VESTING_MISMATCH') {
  exactStructType(value, participationPackageId, '::team_vesting::TeamVesting', code)
}

function capType(value, participationPackageId, code = 'CAP_MISMATCH') {
  exactStructType(value, participationPackageId,
    '::genesis_allocation::GenesisAllocationCap', code)
}

function ownerAddress(owner, expected, code = 'OWNER_MISMATCH') {
  record(owner, code)
  requireValue(owner.__typename === 'AddressOwner', code)
  record(owner.address, code)
  sameAddress(owner.address.address, expected, code)
  return expected
}

function immutableOwner(owner, code = 'OWNER_MISMATCH') {
  record(owner, code)
  requireValue(owner.__typename === 'Immutable', code)
  return 'Immutable'
}

function moveState(value, expectedAddress, validateType, code) {
  const state = record(value, code)
  sameAddress(state.address, expectedAddress, code)
  const version = uint53(state.version, 'VERSION_MISMATCH', true)
  requireValue(digest32(state.digest), code)
  const previous = record(state.previousTransaction, code)
  requireValue(digest32(previous.digest), code)
  const move = record(state.asMoveObject, code)
  requireValue(typeof move.hasPublicTransfer === 'boolean', code)
  const contents = record(move.contents, code)
  const type = record(contents.type, code)
  validateType(type.repr)
  requireValue(typeof contents.bcs === 'string', code)
  return {
    raw: state, address: expectedAddress, version, digest: state.digest,
    previous_transaction: previous.digest, owner: state.owner,
    has_public_transfer: move.hasPublicTransfer, bcs: contents.bcs,
  }
}

function objectChange(value, expectedAddress, code) {
  const change = record(value, code)
  requireValue(change.__typename === 'ObjectChange', code)
  sameAddress(change.address, expectedAddress, code)
  requireValue(typeof change.idCreated === 'boolean' && typeof change.idDeleted === 'boolean', code)
  requireValue(Object.hasOwn(change, 'inputState') && Object.hasOwn(change, 'outputState'), code)
  return change
}

function created(change, code) {
  requireValue(change.idCreated === true && change.idDeleted === false &&
    change.inputState === null && change.outputState !== null, code)
}

function deleted(change, code) {
  requireValue(change.idCreated === false && change.idDeleted === true &&
    change.inputState !== null && change.outputState === null, code)
}

function mutated(change, code) {
  requireValue(change.idCreated === false && change.idDeleted === false &&
    change.inputState !== null && change.outputState !== null, code)
}

function findChange(nodes, id, code) {
  const matches = nodes.filter(node => {
    try { return objectId(node?.address) === id } catch { return false }
  })
  requireValue(matches.length === 1, code)
  return objectChange(matches[0], id, code)
}

module.exports = {
  record, sameAddress, uint53, timestamp, coinType, receiptType, vestingType, capType,
  responseObjectId, ownerAddress, immutableOwner, moveState, objectChange,
  created, deleted, mutated, findChange,
}
