const test = require('node:test')
const assert = require('node:assert/strict')
const { readFileSync } = require('node:fs')
const { join } = require('node:path')
const { validateInput, positiveDecimal, safeCode, MAX_U64, MAX_UINT53,
  CurrencyObservationError, OFFICIAL_TESTNET } = require('../contract')
const { deriveCurrencyId } = require('../canonical')
const { ObservationError } = require('../../esk-sui-publication-observer/contract')

function raw() {
  return {
    network: 'testnet', chain_identifier: '1'.repeat(32), package_id: '0xa',
    publication_digest: '1'.repeat(31) + '2', registration_digest: '1'.repeat(31) + '3',
    registration_version: '2', expected_supply_base_units: '1000000000000000',
    endpoints: [OFFICIAL_TESTNET, 'https://second.sui.io/graphql'],
  }
}

test('strict input normalizes package and derives ESK type without implicit supply', () => {
  const result = validateInput(raw())
  assert.equal(result.package_id, '0x' + 'a'.padStart(64, '0'))
  assert.equal(result.coin_type, result.package_id + '::esk::ESK')
  assert.equal(result.expected_supply_base_units, '1000000000000000')
  assert.equal(result.currency_address, undefined)
})

for (const [field, values] of [
  ['network', ['mainnet', 'devnet', '', null]],
  ['chain_identifier', ['69Wi', 'a'.repeat(64), null]],
  ['package_id', ['0x0', '0xz', '0x' + 'a'.repeat(65), null]],
  ['publication_digest', ['invalid', '', null]],
  ['registration_digest', ['0'.repeat(32), '2'.repeat(45), '', null]],
  ['registration_version', [0, 2, '0', '01', '-1', '1.0', '1e2', '9007199254740992', null]],
  ['expected_supply_base_units', [100, '0', '01', '+1', '1.0', '1e6', ' 1',
    '18446744073709551616', '9'.repeat(10000), null]],
  ['endpoints', [[], [OFFICIAL_TESTNET], [OFFICIAL_TESTNET, OFFICIAL_TESTNET],
    ['https://second.sui.io/graphql', OFFICIAL_TESTNET],
    [OFFICIAL_TESTNET, 'http://second.sui.io/graphql'],
    [OFFICIAL_TESTNET, 'https://user:secret@second.sui.io/graphql'],
    [OFFICIAL_TESTNET, 'https://second.sui.io/graphql?key=secret'],
    [OFFICIAL_TESTNET, 'https://127.0.0.1/graphql']]],
]) {
  values.forEach((value, index) => test(`rejects invalid ${field} case ${index + 1}`, () => {
    assert.throws(() => validateInput({ ...raw(), [field]: value }))
  }))
}

test('missing, unknown and caller-selected currency identity fields are rejected', () => {
  for (const field of Object.keys(raw())) {
    const input = raw()
    delete input[field]
    assert.throws(() => validateInput(input))
  }
  for (const field of ['currency_address', 'coin_type', 'api_key', 'query']) {
    assert.throws(() => validateInput({ ...raw(), [field]: 'secret' }))
  }
  for (const input of [null, [], 'secret', 42]) assert.throws(() => validateInput(input))
})

test('decimal boundaries preserve string integer precision', () => {
  assert.equal(positiveDecimal('18446744073709551615', MAX_U64), '18446744073709551615')
  assert.equal(positiveDecimal('9007199254740991', MAX_UINT53), '9007199254740991')
  assert.equal(positiveDecimal('9007199254740993', MAX_U64), '9007199254740993')
  assert.equal(positiveDecimal('1', MAX_U64), '1')
})

test('error projection never returns injected upstream messages', () => {
  assert.equal(safeCode(new Error('secret')), 'NETWORK_ERROR')
  assert.equal(safeCode({ code: 'secret' }), 'NETWORK_ERROR')
  assert.equal(safeCode(new CurrencyObservationError('secret')), 'INVALID_RESPONSE')
  assert.equal(safeCode(new CurrencyObservationError('SUPPLY_MISMATCH')), 'SUPPLY_MISMATCH')
  assert.equal(safeCode(new ObservationError('TIMEOUT')), 'TIMEOUT')
})

test('locked SDK canonical SUI address matches observed public on-chain registry object', async () => {
  const sui = '0xf256d3fb6a50eaa748d94335b34f2982fbc3b63ceec78cafaa29ebc9ebaf2bbc'
  assert.equal(await deriveCurrencyId('0x2::sui::SUI'), sui)
  assert.equal(await deriveCurrencyId('0x' + '2'.padStart(64, '0') + '::sui::SUI'), sui)
  assert.equal(await deriveCurrencyId('0xa::esk::ESK'),
    '0x4f3148019769e85a19187ec1635f7f3d957a017ee65e6ed7d6d2798ff2f9b2c5')
  assert.notEqual(await deriveCurrencyId('0xb::esk::ESK'), await deriveCurrencyId('0xa::esk::ESK'))
})

test('SDK version and integrity are pinned; lock contains only public npm registry archives', () => {
  const pkg = JSON.parse(readFileSync(join(__dirname, '../package.json'), 'utf8'))
  const lock = JSON.parse(readFileSync(join(__dirname, '../package-lock.json'), 'utf8'))
  assert.equal(pkg.dependencies['@mysten/sui'], '2.29.0')
  const sdk = lock.packages['node_modules/@mysten/sui']
  assert.equal(sdk.version, '2.29.0')
  assert.equal(sdk.integrity,
    'sha512-k7q22+AFQ5SZXOH+a28M1J8iFVbMcWro9mt0Bb7GI1HZNsxJIyQT5Q3iZdrAM0hTwM4XQvzm5Y0f32f0nfiGxw==')
  for (const [path, entry] of Object.entries(lock.packages)) {
    if (!path) continue
    assert.ok(entry.resolved.startsWith('https://registry.npmjs.org/'))
    assert.ok(entry.integrity.startsWith('sha512-'))
  }
})
