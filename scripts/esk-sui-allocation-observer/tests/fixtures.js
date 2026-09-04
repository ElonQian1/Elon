// Synthetic evidence only. These are not ESK publication parameters or wallet addresses.
const { objectId, OFFICIAL_TESTNET } = require('../../esk-sui-publication-observer/contract')

const address = value => objectId(value)
const CURRENCY = address('0xc')
const PARTICIPATION = address('0xd')
const ALLOCATOR = address('0x11')
const DISTRIBUTION = address('0x12')
const TEAM = address('0x13')
const TREASURY = address('0x14')
const LIQUIDITY = address('0x15')
const CAP = address('0x20')
const RECEIPT = address('0x21')
const VESTING = address('0x22')
const SUPPLY = address('0x23')
const USER = address('0x24')
const PROJECT = address('0x25')
const LIQUIDITY_COIN = address('0x26')
const COMMUNITY = address('0x27')
const GAS = address('0x28')

const digest = suffix => `${'1'.repeat(31)}${suffix}`
const CHAIN = digest('2')
const PUBLISH = digest('3')
const PACKAGE_DIGEST = digest('4')
const PUBLISH_CHECKPOINT = digest('5')
const ALLOCATION = digest('6')
const EFFECTS = digest('7')
const ALLOCATION_CHECKPOINT = digest('8')
const OBSERVATION_CHECKPOINT = digest('9')
const RECEIPT_DIGEST = digest('A')
const CAP_PUBLISH_DIGEST = digest('B')
const CAP_INPUT_DIGEST = digest('C')
const SUPPLY_INPUT_DIGEST = digest('D')
const SUPPLY_OUTPUT_DIGEST = digest('E')
const VESTING_DIGEST = digest('F')
const VESTING_CURRENT_DIGEST = digest('G')
const CLAIM_DIGEST = digest('H')
const GAS_INPUT_DIGEST = digest('J')
const GAS_OUTPUT_DIGEST = digest('K')
const PUBLISH_EFFECTS = digest('U')

const MANIFEST = `sha256:${'ab'.repeat(32)}`
const TYPES = Object.freeze({
  cap: `${PARTICIPATION}::genesis_allocation::GenesisAllocationCap`,
  receipt: `${PARTICIPATION}::genesis_allocation::GenesisAllocationReceipt`,
  vesting: `${PARTICIPATION}::team_vesting::TeamVesting`,
  coin: `${address('0x2')}::coin::Coin<${CURRENCY}::esk::ESK>`,
  gas: `${address('0x2')}::coin::Coin<${address('0x2')}::sui::SUI>`,
})

function u64(value) {
  const bytes = Buffer.alloc(8)
  bytes.writeBigUInt64LE(BigInt(value))
  return bytes
}

function id(value) {
  return Buffer.from(address(value).slice(2), 'hex')
}

function vector(bytes) {
  if (bytes.length >= 128) throw new Error('fixture vector is intentionally single-byte ULEB')
  return Buffer.concat([Buffer.from([bytes.length]), bytes])
}

function coinBcs(objectId, amount) {
  return Buffer.concat([id(objectId), u64(amount)]).toString('base64')
}

function capBcs() {
  return id(CAP).toString('base64')
}

function vestingBcs(claimed = '0', remaining = '200') {
  return Buffer.concat([
    id(VESTING), id(TEAM), u64('200'), u64(claimed),
    u64('2000000000000'), u64('2100000000000'), u64('2200000000000'),
    u64(remaining),
  ]).toString('base64')
}

function receiptBcs() {
  return Buffer.concat([
    id(RECEIPT), vector(Buffer.from(MANIFEST.slice(7), 'hex')), u64('1000'),
    id(DISTRIBUTION), id(TEAM), id(TREASURY), id(LIQUIDITY),
    u64('250'), u64('200'), u64('250'), u64('150'), u64('100'), u64('50'),
    u64('2000000000000'), u64('2100000000000'), u64('2200000000000'),
    u64('1999999999000'),
    id(USER), id(VESTING), id(PROJECT), id(LIQUIDITY_COIN), id(COMMUNITY), id(SUPPLY),
  ]).toString('base64')
}

function owner(value) {
  return value === 'Immutable' ? { __typename: 'Immutable' } :
    { __typename: 'AddressOwner', address: { address: value } }
}

function state({ objectId, version = 10, objectDigest, holder, type, bcs, previous,
  transferable = true }) {
  return {
    address: objectId, version, digest: objectDigest, owner: owner(holder),
    previousTransaction: { digest: previous },
    asMoveObject: { hasPublicTransfer: transferable,
      contents: { type: { repr: type }, bcs } },
    asMovePackage: null,
  }
}

function packageState(objectId, objectDigest, previous) {
  return {
    address: objectId, version: 1, digest: objectDigest, owner: owner('Immutable'),
    previousTransaction: { digest: previous }, asMoveObject: null,
    asMovePackage: { address: objectId, version: 1 },
  }
}

function change(objectId, inputState, outputState, { created = false, deleted = false } = {}) {
  return { __typename: 'ObjectChange', address: objectId,
    idCreated: created, idDeleted: deleted, inputState, outputState }
}

function rawInput() {
  return {
    network: 'testnet', chain_identifier: CHAIN,
    currency_package_id: CURRENCY, participation_package_id: PARTICIPATION,
    participation_publication_digest: PUBLISH, allocation_digest: ALLOCATION,
    allocation_cap_object_id: CAP, allocation_receipt_object_id: RECEIPT,
    team_vesting_object_id: VESTING, initial_supply_coin_object_id: SUPPLY,
    allocation_checkpoint_sequence: '101', allocation_checkpoint_digest: ALLOCATION_CHECKPOINT,
    observation_checkpoint_sequence: '110', observation_checkpoint_digest: OBSERVATION_CHECKPOINT,
    manifest_digest: MANIFEST, expected_supply_base_units: '1000',
    holders: { allocator: ALLOCATOR, distribution: DISTRIBUTION,
      team_beneficiary: TEAM, treasury: TREASURY, liquidity_recipient: LIQUIDITY },
    buckets: { user_migration_and_ecosystem: '250', team_vesting: '200',
      project_treasury: '250', liquidity: '150', community_contributors: '100',
      security_operations_reserve: '50' },
    team_vesting: { start_ms: '2000000000000', cliff_ms: '2100000000000',
      end_ms: '2200000000000' },
    endpoints: [OFFICIAL_TESTNET, 'https://reviewed-provider.org/graphql'],
  }
}

function observation(expected = rawInput()) {
  const capPublished = state({ objectId: CAP, version: 1, objectDigest: CAP_PUBLISH_DIGEST,
    holder: ALLOCATOR, type: TYPES.cap, bcs: capBcs(), previous: PUBLISH })
  const capInput = state({ objectId: CAP, version: 2, objectDigest: CAP_INPUT_DIGEST,
    holder: ALLOCATOR, type: TYPES.cap, bcs: capBcs(), previous: digest('L') })
  const receipt = state({ objectId: RECEIPT, objectDigest: RECEIPT_DIGEST,
    holder: 'Immutable', type: TYPES.receipt, bcs: receiptBcs(), previous: ALLOCATION,
    transferable: false })
  const vesting = state({ objectId: VESTING, objectDigest: VESTING_DIGEST,
    holder: TEAM, type: TYPES.vesting, bcs: vestingBcs(), previous: ALLOCATION,
    transferable: false })
  const currentVesting = state({ objectId: VESTING, version: 11,
    objectDigest: VESTING_CURRENT_DIGEST, holder: TEAM, type: TYPES.vesting,
    bcs: vestingBcs('50', '150'), previous: CLAIM_DIGEST, transferable: false })
  const direct = [
    [USER, '250', DISTRIBUTION, digest('M')],
    [PROJECT, '250', TREASURY, digest('N')],
    [LIQUIDITY_COIN, '150', LIQUIDITY, digest('P')],
    [COMMUNITY, '100', DISTRIBUTION, digest('Q')],
  ].map(([objectId, amount, holder, objectDigest]) => change(objectId, null,
    state({ objectId, objectDigest, holder, type: TYPES.coin,
      bcs: coinBcs(objectId, amount), previous: ALLOCATION }), { created: true }))
  const supplyInput = state({ objectId: SUPPLY, version: 9,
    objectDigest: SUPPLY_INPUT_DIGEST, holder: ALLOCATOR, type: TYPES.coin,
    bcs: coinBcs(SUPPLY, '1000'), previous: digest('R') })
  const supplyOutput = state({ objectId: SUPPLY, objectDigest: SUPPLY_OUTPUT_DIGEST,
    holder: TREASURY, type: TYPES.coin, bcs: coinBcs(SUPPLY, '50'),
    previous: ALLOCATION })
  const gasInput = state({ objectId: GAS, version: 9, objectDigest: GAS_INPUT_DIGEST,
    holder: ALLOCATOR, type: TYPES.gas, bcs: coinBcs(GAS, '900'), previous: digest('S') })
  const gasOutput = state({ objectId: GAS, objectDigest: GAS_OUTPUT_DIGEST,
    holder: ALLOCATOR, type: TYPES.gas, bcs: coinBcs(GAS, '800'), previous: ALLOCATION })
  const nodes = [
    change(CAP, capInput, null, { deleted: true }),
    change(RECEIPT, null, receipt, { created: true }),
    change(VESTING, null, vesting, { created: true }),
    change(SUPPLY, supplyInput, supplyOutput), ...direct,
    change(GAS, gasInput, gasOutput),
  ]
  const publicationNodes = [
    change(CAP, null, capPublished, { created: true }),
    change(PARTICIPATION, null,
      packageState(PARTICIPATION, PACKAGE_DIGEST, PUBLISH), { created: true }),
  ]
  return {
    chainIdentifier: expected.chain_identifier,
    participationPublicationTransaction: { digest: expected.participation_publication_digest,
      effects: { status: 'SUCCESS', effectsDigest: PUBLISH_EFFECTS, lamportVersion: 1,
        checkpoint: { sequenceNumber: 100, digest: PUBLISH_CHECKPOINT },
        objectChanges: { nodes: publicationNodes,
          pageInfo: { hasNextPage: false, hasPreviousPage: false } } } },
    participationPackageObject: { address: expected.participation_package_id,
      version: 1, digest: PACKAGE_DIGEST,
      asMovePackage: { address: expected.participation_package_id, version: 1 },
      previousTransaction: { digest: expected.participation_publication_digest } },
    allocationTransaction: { digest: expected.allocation_digest,
      sender: { address: expected.holders.allocator }, effects: {
        status: 'SUCCESS', effectsDigest: EFFECTS, lamportVersion: 10,
        timestamp: new Date(1_999_999_999_000).toISOString(),
        checkpoint: { sequenceNumber: Number(expected.allocation_checkpoint_sequence),
          digest: expected.allocation_checkpoint_digest },
        objectChanges: { nodes, pageInfo: { hasNextPage: false, hasPreviousPage: false } },
      } },
    observationCheckpoint: { sequenceNumber: Number(expected.observation_checkpoint_sequence),
      digest: expected.observation_checkpoint_digest,
      timestamp: new Date(2_000_000_100_000).toISOString() },
    receiptAtObservation: structuredClone(receipt),
    vestingAtObservation: currentVesting,
  }
}

function set(object, path, value) {
  const parts = path.split('.')
  const last = parts.pop()
  const parent = parts.reduce((current, part) => current[part], object)
  if (value === undefined) delete parent[last]
  else parent[last] = value
}

module.exports = {
  rawInput, observation, set, address, digest, TYPES, CURRENCY, PARTICIPATION,
  ALLOCATOR, DISTRIBUTION, TEAM, TREASURY, LIQUIDITY, CAP, RECEIPT, VESTING,
  SUPPLY, USER, PROJECT, LIQUIDITY_COIN, COMMUNITY, MANIFEST, ALLOCATION,
  ALLOCATION_CHECKPOINT, OBSERVATION_CHECKPOINT,
}
