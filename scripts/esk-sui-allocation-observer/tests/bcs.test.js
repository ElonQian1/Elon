const { createHash } = require('node:crypto')
const test = require('node:test')
const assert = require('node:assert/strict')
const {
  BcsDecodeError, strictBase64, decodeReceipt, decodeVesting, decodeCoin, decodeCap,
} = require('../bcs')

const U64_MAX = 18_446_744_073_709_551_615n

function address(byte) {
  return Buffer.alloc(32, byte)
}

function normalized(byte) {
  return `0x${byte.toString(16).padStart(2, '0').repeat(32)}`
}

function u64(value) {
  const result = Buffer.alloc(8)
  result.writeBigUInt64LE(BigInt(value))
  return result
}

function uleb(value) {
  let rest = BigInt(value)
  const result = []
  do {
    let byte = Number(rest & 0x7fn)
    rest >>= 7n
    if (rest !== 0n) byte |= 0x80
    result.push(byte)
  } while (rest !== 0n)
  return Buffer.from(result)
}

function vector(bytes) {
  return Buffer.concat([uleb(bytes.length), bytes])
}

function hash(bytes) {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`
}

function receiptBytes(prefix = uleb(32)) {
  const manifest = Buffer.from(Array.from({ length: 32 }, (_, index) => index))
  return Buffer.concat([
    address(1), prefix, manifest, u64(21),
    address(2), address(3), address(4), address(5),
    u64(1), u64(2), u64(3), u64(4), u64(5), u64(6),
    u64(100), u64(200), u64(300), u64(99),
    address(6), address(7), address(8), address(9), address(10), address(11),
  ])
}

function vestingBytes(total = 10n, claimed = 4n, remaining = 6n) {
  return Buffer.concat([
    address(12), address(13), u64(total), u64(claimed),
    u64(100), u64(200), u64(300), u64(remaining),
  ])
}

function coinBytes(balance = 42n) {
  return Buffer.concat([address(14), u64(balance)])
}

function expectBcsFailure(action) {
  assert.throws(action, error => error instanceof BcsDecodeError && error.code === 'BCS_MISMATCH')
}

test('decodes the exact GenesisAllocationReceipt V1 layout', () => {
  const bytes = receiptBytes()
  assert.deepEqual(decodeReceipt(bytes.toString('base64')), {
    id: normalized(1),
    manifest_digest: `sha256:${Buffer.from(Array.from({ length: 32 }, (_, index) => index)).toString('hex')}`,
    total_base_units: '21',
    distribution: normalized(2),
    team_beneficiary: normalized(3),
    treasury: normalized(4),
    liquidity_recipient: normalized(5),
    user_migration_and_ecosystem_units: '1',
    team_vesting_units: '2',
    project_treasury_units: '3',
    liquidity_units: '4',
    community_contributors_units: '5',
    security_operations_reserve_units: '6',
    start_ms: '100', cliff_ms: '200', end_ms: '300', executed_at_ms: '99',
    user_migration_and_ecosystem_coin_id: normalized(6),
    team_vesting_id: normalized(7),
    project_treasury_coin_id: normalized(8),
    liquidity_coin_id: normalized(9),
    community_contributors_coin_id: normalized(10),
    security_operations_reserve_coin_id: normalized(11),
    bcs_sha256: hash(bytes),
  })
})

test('decodes TeamVesting, Coin and GenesisAllocationCap layouts', () => {
  const vesting = vestingBytes()
  const coin = coinBytes()
  const cap = address(15)
  assert.deepEqual(decodeVesting(vesting.toString('base64')), {
    id: normalized(12), beneficiary: normalized(13), total_base_units: '10',
    claimed_base_units: '4', start_ms: '100', cliff_ms: '200', end_ms: '300',
    remaining_base_units: '6', bcs_sha256: hash(vesting),
  })
  assert.deepEqual(decodeCoin(coin.toString('base64')), {
    id: normalized(14), balance: '42', bcs_sha256: hash(coin),
  })
  assert.deepEqual(decodeCap(cap.toString('base64')), {
    id: normalized(15), bcs_sha256: hash(cap),
  })
})

test('preserves maximum u64 as a decimal string', () => {
  const coin = coinBytes(U64_MAX)
  assert.equal(decodeCoin(coin.toString('base64')).balance, U64_MAX.toString())
  const vesting = vestingBytes(U64_MAX, U64_MAX, U64_MAX)
  const decoded = decodeVesting(vesting.toString('base64'))
  assert.equal(decoded.total_base_units, U64_MAX.toString())
  assert.equal(decoded.claimed_base_units, U64_MAX.toString())
  assert.equal(decoded.remaining_base_units, U64_MAX.toString())
})

test('strict Base64 rejects aliases, whitespace, URL alphabet and unused bits', () => {
  assert.deepEqual(strictBase64('/w=='), Buffer.from([0xff]))
  for (const value of ['/w', '/w==\n', '_w==', 'AB==', 'A===', 7, null]) {
    expectBcsFailure(() => strictBase64(value))
  }
})

test('rejects truncated and trailing bytes for every known top-level layout', () => {
  const cases = [
    [decodeReceipt, receiptBytes()], [decodeVesting, vestingBytes()],
    [decodeCoin, coinBytes()], [decodeCap, address(15)],
  ]
  for (const [decoder, bytes] of cases) {
    expectBcsFailure(() => decoder(bytes.subarray(0, bytes.length - 1).toString('base64')))
    expectBcsFailure(() => decoder(Buffer.concat([bytes, Buffer.from([0])]).toString('base64')))
  }
})

test('requires a 32-byte digest vector and canonical ULEB128', () => {
  expectBcsFailure(() => decodeReceipt(receiptBytes(uleb(31)).toString('base64')))
  expectBcsFailure(() => decodeReceipt(receiptBytes(Buffer.from([0xa0, 0x00])).toString('base64')))
  expectBcsFailure(() => decodeReceipt(receiptBytes(Buffer.from([0x80, 0x80, 0x80, 0x80, 0x10])).toString('base64')))
  expectBcsFailure(() => decodeReceipt(receiptBytes(Buffer.from([0x80])).toString('base64')))
})
