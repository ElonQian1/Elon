const { validateObservation: validatePublication } = require('../esk-sui-publication-observer/observe')
const { CurrencyObservationError, requireValue, digest32, objectId, positiveDecimal, MAX_U64 } = require('./contract')

function record(value, code) {
  requireValue(value !== null && typeof value === 'object' && !Array.isArray(value), code)
  return value
}

function address(value, expected, code) {
  try { requireValue(objectId(value) === expected, code) }
  catch { throw new CurrencyObservationError(code) }
}

function uint53(value, code, positive = false) {
  requireValue(typeof value === 'number' && Number.isSafeInteger(value) && value >= (positive ? 1 : 0), code)
  return String(value)
}

function currencyType(value, expected) {
  requireValue(typeof value === 'string' && value.length <= 220, 'CURRENCY_MISMATCH')
  const match = /^(0x[0-9a-fA-F]{1,64})::coin_registry::Currency<(0x[0-9a-fA-F]{1,64})::esk::ESK>$/.exec(value)
  requireValue(match, 'CURRENCY_MISMATCH')
  address(match[1], objectId('0x2'), 'CURRENCY_MISMATCH')
  address(match[2], expected.package_id, 'CURRENCY_MISMATCH')
}

function sharedCurrency(object, contents, expected) {
  requireValue(object.owner?.__typename === 'Shared', 'CURRENCY_MISMATCH')
  currencyType(contents?.type?.repr, expected)
}

function metadata(value, expected) {
  record(value, 'CURRENCY_MISMATCH')
  address(value.address, expected.currency_address, 'CURRENCY_MISMATCH')
  const version = uint53(value.version, 'VERSION_MISMATCH', true)
  requireValue(value.decimals === 6 && value.symbol === 'ESK', 'CURRENCY_MISMATCH')
  const supply = positiveDecimal(value.supply, MAX_U64, 'SUPPLY_MISMATCH')
  requireValue(supply === expected.expected_supply_base_units && value.supplyState === 'FIXED', 'SUPPLY_MISMATCH')
  return { version, supply }
}

function registrationCheckpoint(transaction, publication, expected) {
  record(transaction, 'REGISTRATION_MISMATCH')
  requireValue(transaction.digest === expected.registration_digest, 'REGISTRATION_MISMATCH')
  requireValue(transaction.effects?.status === 'SUCCESS', 'TRANSACTION_NOT_SUCCESSFUL')
  const checkpoint = record(transaction.effects.checkpoint, 'CHECKPOINT_MISSING')
  const sequence = uint53(checkpoint.sequenceNumber, 'CHECKPOINT_MISSING')
  requireValue(digest32(checkpoint.digest), 'CHECKPOINT_MISSING')
  const order = BigInt(sequence) - BigInt(publication.checkpoint_sequence)
  requireValue(order >= 0n && (order !== 0n || checkpoint.digest === publication.checkpoint_digest), 'REGISTRATION_MISMATCH')
  if (expected.registration_digest === expected.publication_digest) {
    requireValue(order === 0n && checkpoint.digest === publication.checkpoint_digest, 'REGISTRATION_MISMATCH')
  }
  return { sequence, digest: checkpoint.digest }
}

function creationOutput(change, historical, expected) {
  record(change, 'REGISTRATION_MISMATCH')
  requireValue(change.__typename === 'ObjectChange' && change.idCreated === true &&
    change.idDeleted === false && change.inputState === null, 'REGISTRATION_MISMATCH')
  address(change.address, expected.currency_address, 'REGISTRATION_MISMATCH')
  const output = record(change.outputState, 'REGISTRATION_MISMATCH')
  address(output.address, expected.currency_address, 'REGISTRATION_MISMATCH')
  const version = uint53(output.version, 'REGISTRATION_MISMATCH', true)
  requireValue(version === String(historical.version) && output.digest === historical.digest, 'REGISTRATION_MISMATCH')
}

/** Read-only proof over one fixed GraphQL response. Input identity is already normalized upstream. */
function validateObservation(data, expected) {
  record(data, 'INVALID_RESPONSE')
  const publication = validatePublication({ chainIdentifier: data.chainIdentifier,
    transaction: data.publicationTransaction, object: data.packageObject }, expected)
  const checkpoint = registrationCheckpoint(data.registrationTransaction, publication, expected)
  const historical = record(data.registrationObject, 'CURRENCY_MISMATCH')
  address(historical.address, expected.currency_address, 'CURRENCY_MISMATCH')
  requireValue(digest32(historical.digest), 'CURRENCY_MISMATCH')
  const registrationVersion = uint53(historical.version, 'VERSION_MISMATCH', true)
  requireValue(registrationVersion === expected.registration_version, 'VERSION_MISMATCH')
  requireValue(historical.previousTransaction?.digest === expected.registration_digest, 'REGISTRATION_MISMATCH')
  const move = record(historical.asMoveObject, 'CURRENCY_MISMATCH')
  sharedCurrency(historical, move.contents, expected)
  const registrationMetadata = metadata(move.asCoinMetadata, expected)
  requireValue(registrationMetadata.version === registrationVersion, 'VERSION_MISMATCH')
  creationOutput(historical.asTransactionObject, historical, expected)

  const current = record(data.currentMetadata, 'CURRENCY_MISMATCH')
  sharedCurrency(current, current.contents, expected)
  requireValue(digest32(current.digest), 'CURRENCY_MISMATCH')
  const currentMetadata = metadata(current, expected)
  requireValue(BigInt(currentMetadata.version) >= BigInt(registrationVersion), 'VERSION_MISMATCH')
  requireValue(currentMetadata.version !== registrationVersion || current.digest === historical.digest, 'VERSION_MISMATCH')
  // Do not compare current.previousTransaction: later valid metadata versions are not creation evidence.
  return {
    ...publication, coin_type: expected.coin_type, currency_address: expected.currency_address,
    registration_digest: expected.registration_digest,
    registration_checkpoint_sequence: checkpoint.sequence, registration_checkpoint_digest: checkpoint.digest,
    registration_version: registrationVersion, registration_object_digest: historical.digest,
    current_version: currentMetadata.version, current_object_digest: current.digest,
    decimals: 6, symbol: 'ESK', supply_base_units: currentMetadata.supply,
    supply_state: 'FIXED', owner: 'Shared',
  }
}

module.exports = { validateObservation }
