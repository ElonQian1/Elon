const test = require('node:test')
const assert = require('node:assert/strict')
const {
  AllocationObservationError, validateInput, safeCode, objectId, digest32,
  MAX_U64, MAX_UINT53, BUCKET_NAMES, OFFICIAL_TESTNET,
} = require('../contract')
const { ObservationError } = require('../../esk-sui-publication-observer/contract')

const DIGEST = '1'.repeat(32)
const digest = suffix => '1'.repeat(31) + suffix
const id = value => `0x${value}`

function raw() {
  return {
    network: 'testnet',
    chain_identifier: DIGEST,
    currency_package_id: id('a'),
    participation_package_id: id('b'),
    participation_publication_digest: digest('2'),
    allocation_digest: digest('3'),
    allocation_cap_object_id: id('c'),
    allocation_receipt_object_id: id('d'),
    team_vesting_object_id: id('e'),
    initial_supply_coin_object_id: id('f'),
    allocation_checkpoint_sequence: '42',
    allocation_checkpoint_digest: digest('4'),
    observation_checkpoint_sequence: '51',
    observation_checkpoint_digest: digest('5'),
    manifest_digest: `sha256:${'a'.repeat(64)}`,
    expected_supply_base_units: '1000',
    holders: {
      allocator: id('10'), distribution: id('11'), team_beneficiary: id('12'),
      treasury: id('13'), liquidity_recipient: id('14'),
    },
    buckets: {
      user_migration_and_ecosystem: '250', team_vesting: '200',
      project_treasury: '250', liquidity: '150', community_contributors: '100',
      security_operations_reserve: '50',
    },
    team_vesting: {
      start_ms: '1893456000000', cliff_ms: '1924992000000', end_ms: '2019686400000',
    },
    endpoints: [OFFICIAL_TESTNET, 'https://reviewed-provider.org/graphql'],
  }
}

test('strict input preserves the V1 shape and normalizes every address', () => {
  const input = raw()
  const result = validateInput(input)
  assert.deepEqual(Object.keys(result), Object.keys(input))
  assert.deepEqual(result.endpoints, input.endpoints)
  for (const field of [
    'currency_package_id', 'participation_package_id', 'allocation_cap_object_id',
    'allocation_receipt_object_id', 'team_vesting_object_id',
    'initial_supply_coin_object_id',
  ]) assert.match(result[field], /^0x[0-9a-f]{64}$/)
  for (const value of Object.values(result.holders)) assert.match(value, /^0x[0-9a-f]{64}$/)
  assert.deepEqual(result.buckets, input.buckets)
  assert.deepEqual(result.team_vesting, input.team_vesting)
  assert.equal(result.manifest_digest, input.manifest_digest)
  assert.equal(result.coin_type, undefined)
})

test('root and nested objects reject missing and unknown keys', () => {
  for (const field of Object.keys(raw())) {
    const input = raw()
    delete input[field]
    assert.throws(() => validateInput(input), `${field} must be required`)
  }
  for (const field of ['api_key', 'query', 'private_key', 'vesting_policy_ref']) {
    assert.throws(() => validateInput({ ...raw(), [field]: 'secret' }))
  }
  for (const [field, unknown] of [
    ['holders', 'legacy_holder'], ['buckets', 'legacy_bucket'], ['team_vesting', 'revocable'],
  ]) {
    const input = raw()
    input[field] = { ...input[field], [unknown]: 'secret' }
    assert.throws(() => validateInput(input))
    delete input[field][Object.keys(input[field])[0]]
    assert.throws(() => validateInput(input))
  }
  for (const input of [null, [], 'secret', 42]) assert.throws(() => validateInput(input))
})

test('network, endpoints and all digest fields fail closed', () => {
  for (const value of ['mainnet', 'devnet', '', null]) {
    assert.throws(() => validateInput({ ...raw(), network: value }))
  }
  for (const field of [
    'chain_identifier', 'participation_publication_digest', 'allocation_digest',
    'allocation_checkpoint_digest', 'observation_checkpoint_digest',
  ]) {
    for (const value of ['secret', '0'.repeat(32), '1'.repeat(31), null]) {
      assert.throws(() => validateInput({ ...raw(), [field]: value }), field)
    }
  }
  for (const endpoints of [
    [], [OFFICIAL_TESTNET], [OFFICIAL_TESTNET, OFFICIAL_TESTNET],
    ['https://reviewed-provider.org/graphql', OFFICIAL_TESTNET],
    [OFFICIAL_TESTNET, 'http://reviewed-provider.org/graphql'],
    [OFFICIAL_TESTNET, 'https://user:secret@reviewed-provider.org/graphql'],
    [OFFICIAL_TESTNET, 'https://reviewed-provider.org/graphql?key=secret'],
    [OFFICIAL_TESTNET, 'https://127.0.0.1/graphql'],
  ]) assert.throws(() => validateInput({ ...raw(), endpoints }))
})

test('packages, evidence objects and real holder addresses are nonzero Sui IDs', () => {
  for (const field of [
    'currency_package_id', 'participation_package_id', 'allocation_cap_object_id',
    'allocation_receipt_object_id', 'team_vesting_object_id',
    'initial_supply_coin_object_id',
  ]) {
    for (const value of ['0x0', '0xz', `0x${'a'.repeat(65)}`, null]) {
      assert.throws(() => validateInput({ ...raw(), [field]: value }), field)
    }
  }
  for (const field of Object.keys(raw().holders)) {
    const input = raw()
    input.holders = { ...input.holders, [field]: `synthetic:sui:${id('1'.repeat(64))}` }
    assert.throws(() => validateInput(input), field)
  }
  assert.throws(() => validateInput({ ...raw(), participation_package_id: id('a') }))
  for (const field of [
    'allocation_receipt_object_id', 'team_vesting_object_id', 'initial_supply_coin_object_id',
  ]) assert.throws(() => validateInput({ ...raw(), [field]: id('c') }))
  assert.throws(() => validateInput({ ...raw(), allocation_cap_object_id: id('a') }))
  assert.throws(() => validateInput({ ...raw(), allocation_receipt_object_id: id('b') }))
})

test('four destination roles are pairwise distinct while allocator may be one of them', () => {
  const input = raw()
  input.holders = { ...input.holders, allocator: input.holders.distribution }
  assert.equal(validateInput(input).holders.allocator, objectId(input.holders.distribution))
  for (const [left, right] of [
    ['distribution', 'team_beneficiary'], ['distribution', 'treasury'],
    ['distribution', 'liquidity_recipient'], ['team_beneficiary', 'treasury'],
    ['team_beneficiary', 'liquidity_recipient'], ['treasury', 'liquidity_recipient'],
  ]) {
    const candidate = raw()
    candidate.holders = { ...candidate.holders, [right]: candidate.holders[left] }
    assert.throws(() => validateInput(candidate), `${left}/${right}`)
  }
})

test('manifest must be a nonzero lowercase sha256 digest', () => {
  for (const value of [
    `sha256:${'0'.repeat(64)}`, `sha256:${'A'.repeat(64)}`, 'a'.repeat(64),
    `sha256:${'a'.repeat(63)}`, `sha256:${'g'.repeat(64)}`, null,
  ]) assert.throws(() => validateInput({ ...raw(), manifest_digest: value }))
})

test('six named positive u64 buckets conserve the exact expected supply', () => {
  assert.deepEqual(BUCKET_NAMES, [
    'user_migration_and_ecosystem', 'team_vesting', 'project_treasury', 'liquidity',
    'community_contributors', 'security_operations_reserve',
  ])
  for (const bucket of BUCKET_NAMES) {
    for (const value of [0, '0', '01', '+1', '-1', '1.0', '1e2',
      '18446744073709551616', null]) {
      const input = raw()
      input.buckets = { ...input.buckets, [bucket]: value }
      assert.throws(() => validateInput(input), bucket)
    }
  }
  assert.throws(() => validateInput({ ...raw(), expected_supply_base_units: '1001' }))
  assert.throws(() => validateInput({ ...raw(), expected_supply_base_units: '0' }))
  assert.throws(() => validateInput({ ...raw(), expected_supply_base_units: '18446744073709551616' }))

  const maximum = raw()
  maximum.expected_supply_base_units = MAX_U64.toString()
  maximum.buckets = Object.fromEntries(BUCKET_NAMES.map(name => [name, '1']))
  maximum.buckets.user_migration_and_ecosystem = (MAX_U64 - 5n).toString()
  assert.equal(validateInput(maximum).expected_supply_base_units, MAX_U64.toString())
})

test('checkpoint UInt53 strings are ordered and bind equal sequence to equal digest', () => {
  const maximum = raw()
  maximum.allocation_checkpoint_sequence = MAX_UINT53.toString()
  maximum.observation_checkpoint_sequence = MAX_UINT53.toString()
  maximum.observation_checkpoint_digest = maximum.allocation_checkpoint_digest
  assert.equal(validateInput(maximum).observation_checkpoint_sequence, MAX_UINT53.toString())

  for (const field of ['allocation_checkpoint_sequence', 'observation_checkpoint_sequence']) {
    for (const value of [0, '0', '01', '1.0', '9007199254740992', null]) {
      assert.throws(() => validateInput({ ...raw(), [field]: value }), field)
    }
  }
  assert.throws(() => validateInput({ ...raw(), observation_checkpoint_sequence: '41' }))
  assert.throws(() => validateInput({ ...raw(), observation_checkpoint_sequence: '42' }))
  const same = raw()
  same.observation_checkpoint_sequence = same.allocation_checkpoint_sequence
  same.observation_checkpoint_digest = same.allocation_checkpoint_digest
  assert.equal(validateInput(same).observation_checkpoint_sequence, '42')
})

test('vesting timestamps are positive u64 strings in strict ascending order', () => {
  for (const field of ['start_ms', 'cliff_ms', 'end_ms']) {
    for (const value of [0, '0', '01', '-1', '1.5', '18446744073709551616', null]) {
      const input = raw()
      input.team_vesting = { ...input.team_vesting, [field]: value }
      assert.throws(() => validateInput(input), field)
    }
  }
  for (const team_vesting of [
    { start_ms: '2', cliff_ms: '2', end_ms: '3' },
    { start_ms: '2', cliff_ms: '1', end_ms: '3' },
    { start_ms: '1', cliff_ms: '3', end_ms: '3' },
  ]) assert.throws(() => validateInput({ ...raw(), team_vesting }))
  const maximum = raw()
  maximum.team_vesting = {
    start_ms: (MAX_U64 - 2n).toString(), cliff_ms: (MAX_U64 - 1n).toString(),
    end_ms: MAX_U64.toString(),
  }
  assert.deepEqual(validateInput(maximum).team_vesting, maximum.team_vesting)
})

test('digest and object helpers retain strict public contracts', () => {
  assert.ok(digest32(DIGEST) && digest32(digest('2')))
  assert.equal(digest32('0'.repeat(32)), false)
  assert.equal(objectId('0xA'), `0x${'a'.padStart(64, '0')}`)
  assert.throws(() => objectId('0x0'))
})

test('safe error projection preserves only registered domain and transport codes', () => {
  assert.equal(safeCode(new Error('secret endpoint URL')), 'NETWORK_ERROR')
  assert.equal(safeCode({ code: 'OWNER_MISMATCH', message: 'secret' }), 'NETWORK_ERROR')
  assert.equal(safeCode(new AllocationObservationError('secret')), 'INVALID_RESPONSE')
  for (const code of [
    'INVALID_INPUT', 'BCS_MISMATCH', 'CAP_MISMATCH', 'COIN_MISMATCH',
    'OWNER_MISMATCH', 'SOURCE_DISAGREEMENT',
  ]) assert.equal(safeCode(new AllocationObservationError(code)), code)
  for (const code of [
    'INVALID_ENDPOINT', 'PRIVATE_ADDRESS', 'TIMEOUT', 'HTTP_ERROR',
    'RESPONSE_TOO_LARGE', 'GRAPHQL_ERROR', 'CHAIN_MISMATCH',
  ]) assert.equal(safeCode(new ObservationError(code)), code)
})
