const test = require('node:test')
const assert = require('node:assert/strict')
const { validateObservation } = require('../validation')
const { safeCode } = require('../contract')
const { objectId } = require('../../esk-sui-publication-observer/contract')
const { input, observation, set, DIGEST, REGISTRATION_DIGEST, OBJECT_DIGEST, CURRENT_DIGEST, U64_MAX } = require('./fixtures')

function rejected(name, mutate, code) {
  test(name, () => {
    const expected = input()
    const data = observation(expected)
    mutate(data, expected)
    assert.throws(() => validateObservation(data, expected), error => safeCode(error) === code)
  })
}

test('stable normalized evidence preserves publication and both currency versions', () => {
  const expected = input()
  const data = observation(expected)
  const before = structuredClone(data)
  assert.deepEqual(validateObservation(data, expected), {
    chain_identifier: expected.chain_identifier, package_id: expected.package_id,
    package_version: '1', package_digest: DIGEST, publication_digest: DIGEST,
    checkpoint_sequence: '100', checkpoint_digest: DIGEST,
    coin_type: expected.coin_type, currency_address: expected.currency_address,
    registration_digest: REGISTRATION_DIGEST, registration_checkpoint_sequence: '101',
    registration_checkpoint_digest: REGISTRATION_DIGEST, registration_version: '2',
    registration_object_digest: OBJECT_DIGEST, current_version: '2', current_object_digest: OBJECT_DIGEST,
    decimals: 6, symbol: 'ESK', supply_base_units: expected.expected_supply_base_units,
    supply_state: 'FIXED', owner: 'Shared',
  })
  assert.deepEqual(data, before)
})

test('short/full uppercase hex aliases normalize to identical evidence', () => {
  const expected = input()
  const original = observation(expected)
  const alias = observation(expected)
  const repr = `${objectId('0x2')}::coin_registry::Currency<0xA::esk::ESK>`
  alias.registrationObject.asMoveObject.contents.type.repr = repr
  alias.currentMetadata.contents.type.repr = repr
  for (const path of ['currentMetadata.address', 'registrationObject.address',
    'registrationObject.asMoveObject.asCoinMetadata.address', 'registrationObject.asTransactionObject.address',
    'registrationObject.asTransactionObject.outputState.address']) set(alias, path, `0x${expected.currency_address.slice(2).toUpperCase()}`)
  assert.deepEqual(validateObservation(alias, expected), validateObservation(original, expected))
})

test('later current version is valid and does not use its previous transaction as registration', () => {
  const expected = input()
  const data = observation(expected)
  data.currentMetadata.version = 9
  data.currentMetadata.digest = CURRENT_DIGEST
  data.currentMetadata.previousTransaction = { digest: 'not-the-registration-transaction' }
  const result = validateObservation(data, expected)
  assert.equal(result.current_version, '9')
  assert.equal(result.current_object_digest, CURRENT_DIGEST)
  assert.equal(result.registration_version, '2')
  assert.equal(result.registration_object_digest, OBJECT_DIGEST)
})

test('same checkpoint requires the same checkpoint digest', () => {
  const data = observation()
  data.registrationTransaction.effects.checkpoint = { ...data.publicationTransaction.effects.checkpoint }
  assert.equal(validateObservation(data, input()).registration_checkpoint_sequence, '100')
})

test('the same transaction must report the same checkpoint for publication and registration', () => {
  const expected = input()
  expected.registration_digest = expected.publication_digest
  const data = observation(expected)
  assert.throws(() => validateObservation(data, expected), error => safeCode(error) === 'REGISTRATION_MISMATCH')
  data.registrationTransaction.effects.checkpoint = { ...data.publicationTransaction.effects.checkpoint }
  assert.equal(validateObservation(data, expected).registration_checkpoint_sequence, '100')
})

test('maximum u64 supply and UInt53 versions are preserved without precision loss', () => {
  const expected = input()
  expected.expected_supply_base_units = U64_MAX
  expected.registration_version = String(Number.MAX_SAFE_INTEGER)
  const data = observation(expected)
  data.registrationTransaction.effects.checkpoint.sequenceNumber = Number.MAX_SAFE_INTEGER
  const result = validateObservation(data, expected)
  assert.equal(result.supply_base_units, U64_MAX)
  assert.equal(result.registration_version, String(Number.MAX_SAFE_INTEGER))
  assert.equal(result.current_version, String(Number.MAX_SAFE_INTEGER))
})

for (const [path, value, code] of [
  ['chainIdentifier', REGISTRATION_DIGEST, 'CHAIN_MISMATCH'],
  ['packageObject', null, 'PACKAGE_MISMATCH'],
  ['packageObject.address', '0xb', 'PACKAGE_MISMATCH'],
  ['packageObject.asMovePackage.version', 2, 'PACKAGE_MISMATCH'],
  ['publicationTransaction.digest', REGISTRATION_DIGEST, 'TRANSACTION_MISMATCH'],
  ['publicationTransaction.effects.status', 'FAILURE', 'TRANSACTION_NOT_SUCCESSFUL'],
  ['registrationTransaction', null, 'REGISTRATION_MISMATCH'],
  ['registrationTransaction.digest', DIGEST, 'REGISTRATION_MISMATCH'],
  ['registrationTransaction.effects', null, 'TRANSACTION_NOT_SUCCESSFUL'],
  ['registrationTransaction.effects.status', 'FAILURE', 'TRANSACTION_NOT_SUCCESSFUL'],
  ['registrationTransaction.effects.checkpoint', null, 'CHECKPOINT_MISSING'],
  ['registrationTransaction.effects.checkpoint.digest', 'private-invalid', 'CHECKPOINT_MISSING'],
  ['registrationTransaction.effects.checkpoint.sequenceNumber', 99, 'REGISTRATION_MISMATCH'],
  ['registrationTransaction.effects.checkpoint.sequenceNumber', 100, 'REGISTRATION_MISMATCH'],
  ['registrationObject', null, 'CURRENCY_MISMATCH'],
  ['registrationObject.address', '0xb', 'CURRENCY_MISMATCH'],
  ['registrationObject.digest', 'private-invalid', 'CURRENCY_MISMATCH'],
  ['registrationObject.version', 3, 'VERSION_MISMATCH'],
  ['registrationObject.previousTransaction', null, 'REGISTRATION_MISMATCH'],
  ['registrationObject.previousTransaction.digest', DIGEST, 'REGISTRATION_MISMATCH'],
  ['registrationObject.owner.__typename', 'AddressOwner', 'CURRENCY_MISMATCH'],
  ['registrationObject.asMoveObject', null, 'CURRENCY_MISMATCH'],
  ['registrationObject.asMoveObject.asCoinMetadata', null, 'CURRENCY_MISMATCH'],
  ['registrationObject.asMoveObject.asCoinMetadata.address', '0xb', 'CURRENCY_MISMATCH'],
  ['registrationObject.asMoveObject.asCoinMetadata.version', 3, 'VERSION_MISMATCH'],
  ['currentMetadata', null, 'CURRENCY_MISMATCH'],
  ['currentMetadata.address', '0xb', 'CURRENCY_MISMATCH'],
  ['currentMetadata.digest', 'private-invalid', 'CURRENCY_MISMATCH'],
  ['currentMetadata.owner.__typename', 'Immutable', 'CURRENCY_MISMATCH'],
  ['currentMetadata.version', 1, 'VERSION_MISMATCH'],
  ['currentMetadata.digest', CURRENT_DIGEST, 'VERSION_MISMATCH'],
  ['registrationObject.asTransactionObject', null, 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.__typename', 'Object', 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.address', '0xb', 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.idCreated', false, 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.idCreated', 'true', 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.idDeleted', true, 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.idDeleted', 0, 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.inputState', {}, 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.inputState', undefined, 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.outputState', null, 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.outputState.address', '0xb', 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.outputState.version', 3, 'REGISTRATION_MISMATCH'],
  ['registrationObject.asTransactionObject.outputState.digest', CURRENT_DIGEST, 'REGISTRATION_MISMATCH'],
]) rejected(`reject field ${path}=${String(value)}`, data => set(data, path, value), code)

for (const root of ['currentMetadata', 'registrationObject.asMoveObject.asCoinMetadata']) {
  for (const [field, values, code] of [
    ['decimals', [null, '6', 5, 7, 6.1, undefined], 'CURRENCY_MISMATCH'],
    ['symbol', [null, 'esk', ' ESK', 'ESK ', 'ESK\0', undefined], 'CURRENCY_MISMATCH'],
    ['supply', [null, 1000000000, '0', '-1', '+1', '01', '1e9', '1.0', '18446744073709551616', '999', undefined], 'SUPPLY_MISMATCH'],
    ['supplyState', [null, 'BURN_ONLY', 'UNKNOWN', 'fixed', undefined], 'SUPPLY_MISMATCH'],
  ]) for (const value of values) rejected(`reject ${root}.${field}=${String(value)}`, data => set(data, `${root}.${field}`, value), code)
}

for (const path of ['currentMetadata.contents.type.repr', 'registrationObject.asMoveObject.contents.type.repr']) {
  for (const repr of [null, 6, '0x2::coin::CoinMetadata<0xa::esk::ESK>', '0x3::coin_registry::Currency<0xa::esk::ESK>',
    '0x2::coin_registry::Currency<0xb::esk::ESK>', '0x2::coin_registry::Currency<0xa::other::ESK>',
    '0x2::coin_registry::Currency<0xa::esk::esk>', '0x2::coin_registry::Currency<0xa::esk::ESK,0xa::esk::ESK>',
    'prefix0x2::coin_registry::Currency<0xa::esk::ESK>', '0x2::coin_registry::Currency<0xa::esk::ESK>suffix',
    ' 0x2::coin_registry::Currency<0xa::esk::ESK>', `0x${'1'.repeat(65)}::coin_registry::Currency<0xa::esk::ESK>`]) {
    rejected(`reject non-exact Currency type at ${path}: ${String(repr)}`, data => set(data, path, repr), 'CURRENCY_MISMATCH')
  }
}

for (const path of ['currentMetadata.version', 'registrationObject.version', 'registrationObject.asMoveObject.asCoinMetadata.version']) {
  for (const version of [0, -1, 1.5, '2', null, true, Number.MAX_SAFE_INTEGER + 1, NaN, Infinity, undefined]) {
    rejected(`reject UInt53 object version at ${path}: ${String(version)}`, data => set(data, path, version), 'VERSION_MISMATCH')
  }
}
for (const value of [-1, 1.5, '101', null, true, Number.MAX_SAFE_INTEGER + 1, NaN, Infinity, undefined]) {
  rejected(`reject non-UInt53 checkpoint: ${String(value)}`, data => set(data,
    'registrationTransaction.effects.checkpoint.sequenceNumber', value), 'CHECKPOINT_MISSING')
}
for (const value of [null, [], 'private-response', 1]) {
  test(`reject malformed root ${String(value)}`, () => {
    assert.throws(() => validateObservation(value, input()), error => safeCode(error) === 'INVALID_RESPONSE')
  })
}

// Each field selected by the fixed query is required, including explicit null inputState.
function paths(value, prefix = '') {
  return Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key
    return [path, ...(child && typeof child === 'object' ? paths(child, path) : [])]
  })
}
const failureCodes = new Set(['INVALID_RESPONSE', 'CHAIN_MISMATCH', 'PACKAGE_MISMATCH',
  'TRANSACTION_MISMATCH', 'TRANSACTION_NOT_SUCCESSFUL', 'CHECKPOINT_MISSING',
  'REGISTRATION_MISMATCH', 'CURRENCY_MISMATCH', 'VERSION_MISMATCH', 'SUPPLY_MISMATCH'])
for (const path of paths(observation())) {
  test(`missing selected field fails closed: ${path}`, () => {
    const data = observation()
    set(data, path, undefined)
    assert.throws(() => validateObservation(data, input()), error => failureCodes.has(safeCode(error)))
  })
}

for (const value of [0, -1, 1.5, '2', null, true, Number.MAX_SAFE_INTEGER + 1, NaN, Infinity, undefined]) {
  rejected(`ObjectChange output version requires UInt53 number: ${String(value)}`, data =>
    set(data, 'registrationObject.asTransactionObject.outputState.version', value), 'REGISTRATION_MISMATCH')
}

test('zero checkpoint sequence is valid when both matching checkpoints are genesis', () => {
  const data = observation()
  data.publicationTransaction.effects.checkpoint = { sequenceNumber: 0, digest: DIGEST }
  data.registrationTransaction.effects.checkpoint = { sequenceNumber: 0, digest: DIGEST }
  assert.equal(validateObservation(data, input()).registration_checkpoint_sequence, '0')
})

test('unselected current previousTransaction cannot alter normalized evidence', () => {
  const data = observation()
  const original = validateObservation(data, input())
  data.currentMetadata.previousTransaction = { digest: REGISTRATION_DIGEST }
  assert.deepEqual(validateObservation(data, input()), original)
  data.currentMetadata.previousTransaction = null
  assert.deepEqual(validateObservation(data, input()), original)
})
