const assert = require('node:assert/strict')
const crypto = require('node:crypto')
const fs = require('node:fs')
const path = require('node:path')

const root = path.resolve(__dirname, '..')
const fromRoot = (relative) => path.join(root, relative)
const read = (relative) => fs.readFileSync(fromRoot(relative), 'utf8')
const parse = (relative) => JSON.parse(read(relative))
const canonicalBytes = (bytes) => Buffer.from(bytes.toString('utf8').replace(/\r\n?/g, '\n'), 'utf8')
const sha256 = (bytes) => `sha256:${crypto.createHash('sha256').update(bytes).digest('hex')}`
const canonicalSha256 = (relative) => sha256(canonicalBytes(fs.readFileSync(fromRoot(relative))))

const schemaPath = 'contracts/sui/esk-allocation-policy-v1.schema.json'
const fixturePath = 'contracts/sui/esk-allocation-policy-v1.fixture.json'
const packageRoot = 'contracts/sui/yilong_participation'
const moveTomlPath = `${packageRoot}/Move.toml`
const genesisSourcePath = `${packageRoot}/sources/genesis_allocation.move`
const vestingSourcePath = `${packageRoot}/sources/team_vesting.move`
const genesisTestsPath = `${packageRoot}/tests/genesis_allocation_tests.move`
const vestingTestsPath = `${packageRoot}/tests/team_vesting_tests.move`
const moveTestEvidencePath = `${packageRoot}/evidence/move-test-output-v1.txt`
const currencyTestEvidencePath = `${packageRoot}/evidence/esk-currency-regression-output-v1.txt`
const currencySourcePath = 'contracts/sui/esk_currency/sources/esk.move'
const currencyTestsPath = 'contracts/sui/esk_currency/tests/esk_tests.move'
const currencyManifestPath = 'contracts/sui/esk-genesis-manifest-v1.fixture.json'
const requirementPath = 'docs/requirements/esk-sui-allocation-vesting-v1.md'
const decisionPath = 'docs/decisions/esk-sui-allocation-vesting-v1.md'

const schema = parse(schemaPath)
const fixture = parse(fixturePath)
const currencyManifest = parse(currencyManifestPath)
const moveToml = read(moveTomlPath)
const genesisSource = read(genesisSourcePath)
const vestingSource = read(vestingSourcePath)
const genesisTests = read(genesisTestsPath)
const vestingTests = read(vestingTestsPath)
// Git stores evidence as LF. Canonicalize Windows checkout bytes before raw receipt checks.
const moveTestEvidence = canonicalBytes(fs.readFileSync(fromRoot(moveTestEvidencePath))).toString('utf8')
const currencyTestEvidence = canonicalBytes(fs.readFileSync(fromRoot(currencyTestEvidencePath))).toString('utf8')
const requirement = read(requirementPath)
const decision = read(decisionPath)

function deepEqual(left, right) {
  try {
    assert.deepEqual(left, right)
    return true
  } catch {
    return false
  }
}

function resolveRef(ref) {
  assert.match(ref, /^#\/\$defs\/[A-Za-z0-9_-]+$/, `unsupported schema ref: ${ref}`)
  return ref.slice(8).split('/').reduce((value, key) => value[key], schema.$defs)
}

function typeMatches(expected, value) {
  if (expected === 'null') return value === null
  if (expected === 'array') return Array.isArray(value)
  if (expected === 'object') return value !== null && typeof value === 'object' && !Array.isArray(value)
  if (expected === 'integer') return Number.isInteger(value)
  return typeof value === expected
}

function validate(node, value, location = '$') {
  const errors = []
  const add = (message) => errors.push(`${location}: ${message}`)
  if (node.$ref) errors.push(...validate(resolveRef(node.$ref), value, location))
  const types = node.type === undefined ? [] : Array.isArray(node.type) ? node.type : [node.type]
  if (types.length && !types.some((type) => typeMatches(type, value))) {
    add(`expected ${types.join('|')}`)
    return errors
  }
  if (Object.hasOwn(node, 'const') && !deepEqual(value, node.const)) add(`must equal ${JSON.stringify(node.const)}`)
  if (node.enum && !node.enum.some((candidate) => deepEqual(candidate, value))) add('must match enum')
  if (typeof value === 'string') {
    if (node.pattern && !new RegExp(node.pattern).test(value)) add(`must match ${node.pattern}`)
    if (node.format === 'date-time' && !Number.isFinite(Date.parse(value))) add('must be a real date-time')
  }
  if (typeof value === 'number') {
    if (node.minimum !== undefined && value < node.minimum) add(`must be >= ${node.minimum}`)
    if (node.maximum !== undefined && value > node.maximum) add(`must be <= ${node.maximum}`)
  }
  if (Array.isArray(value)) {
    if (node.minItems !== undefined && value.length < node.minItems) add(`must have at least ${node.minItems} items`)
    if (node.maxItems !== undefined && value.length > node.maxItems) add(`must have at most ${node.maxItems} items`)
    if (node.uniqueItems && new Set(value.map(JSON.stringify)).size !== value.length) add('items must be unique')
    if (node.items) value.forEach((item, index) => errors.push(...validate(node.items, item, `${location}[${index}]`)))
    if (node.contains) {
      const matches = value.filter((item) => validate(node.contains, item, location).length === 0).length
      if (node.minContains !== undefined && matches < node.minContains) add(`contains fewer than ${node.minContains} matches`)
      if (node.maxContains !== undefined && matches > node.maxContains) add(`contains more than ${node.maxContains} matches`)
    }
  }
  if (value !== null && typeof value === 'object' && !Array.isArray(value)) {
    for (const key of node.required || []) if (!Object.hasOwn(value, key)) add(`missing ${key}`)
    for (const [key, child] of Object.entries(node.properties || {})) {
      if (Object.hasOwn(value, key)) errors.push(...validate(child, value[key], `${location}.${key}`))
    }
    if (node.additionalProperties === false) {
      for (const key of Object.keys(value)) if (!Object.hasOwn(node.properties || {}, key)) add(`unexpected ${key}`)
    }
  }
  for (const child of node.allOf || []) errors.push(...validate(child, value, location))
  return errors
}

function packageInputDigest(relativeRoot) {
  const absoluteRoot = fromRoot(relativeRoot)
  const files = []
  const visit = (directory) => fs.readdirSync(directory, { withFileTypes: true }).forEach((entry) => {
    const absolute = path.join(directory, entry.name)
    if (entry.isDirectory() && entry.name !== 'build') visit(absolute)
    else if (entry.isFile() && (entry.name === 'Move.toml' || entry.name === 'Move.lock' || entry.name.endsWith('.move'))) files.push(absolute)
  })
  visit(absoluteRoot)
  const hash = crypto.createHash('sha256')
  for (const file of files.sort()) {
    hash.update(path.relative(absoluteRoot, file).replaceAll('\\', '/')).update('\0')
    hash.update(canonicalBytes(fs.readFileSync(file))).update('\0')
  }
  return `sha256:${hash.digest('hex')}`
}

const expectedBuckets = {
  user_migration_and_ecosystem: 'distribution',
  team_vesting: 'team_vesting',
  project_treasury: 'treasury',
  liquidity: 'liquidity',
  community_contributors: 'distribution',
  security_operations_reserve: 'treasury',
}
const expectedSyntheticExample = {
  user_migration_and_ecosystem: ['250000000000000', 2500, 'distribution'],
  team_vesting: ['200000000000000', 2000, 'team_vesting'],
  project_treasury: ['250000000000000', 2500, 'treasury'],
  liquidity: ['150000000000000', 1500, 'liquidity'],
  community_contributors: ['100000000000000', 1000, 'distribution'],
  security_operations_reserve: ['50000000000000', 500, 'treasury'],
}

function semanticErrors(candidate) {
  const errors = []
  const add = (location, message) => errors.push(`${location}: ${message}`)
  let total
  try {
    total = BigInt(candidate.asset.total_base_units)
  } catch {
    add('$.asset.total_base_units', 'must be an integer string')
  }
  const allocations = candidate.allocations || []
  const ids = allocations.map((item) => item.bucket_id)
  if (new Set(ids).size !== ids.length) add('$.allocations', 'bucket ids must be unique')
  if (!deepEqual([...ids].sort(), Object.keys(expectedBuckets).sort())) add('$.allocations', 'must contain the exact six buckets')
  try {
    const allocated = allocations.reduce((sum, item) => sum + BigInt(item.base_units), 0n)
    const basisPoints = allocations.reduce((sum, item) => sum + item.basis_points, 0)
    if (allocated !== total) add('$.allocations', 'base units must conserve total supply')
    if (basisPoints !== 10_000) add('$.allocations', 'basis points must total 10000')
    allocations.forEach((item, index) => {
      if (item.recipient_role !== expectedBuckets[item.bucket_id]) add(`$.allocations[${index}]`, 'recipient role mismatch')
      if (BigInt(item.base_units) <= 0n) add(`$.allocations[${index}].base_units`, 'must be positive')
      if (BigInt(item.base_units) * 10_000n !== total * BigInt(item.basis_points)) add(`$.allocations[${index}]`, 'amount and basis points differ')
    })
  } catch {
    add('$.allocations', 'amounts must be valid integers')
  }
  const holders = Object.values(candidate.holders || {})
  if (new Set(holders).size !== holders.length) add('$.holders', 'holders must be distinct')
  if (holders.some((holder) => !/^synthetic:sui:0x[0-9a-f]{64}$/.test(holder))) add('$.holders', 'only synthetic holder references are allowed')
  try {
    const start = BigInt(candidate.team_vesting.start_ms)
    const cliff = BigInt(candidate.team_vesting.cliff_ms)
    const end = BigInt(candidate.team_vesting.end_ms)
    if (!(start < cliff && cliff < end)) add('$.team_vesting', 'must satisfy start < cliff < end')
  } catch {
    add('$.team_vesting', 'timestamps must be integer strings')
  }
  if (candidate.runtime_verification?.package_input_digest !== packageInputDigest(packageRoot)) add('$.runtime_verification.package_input_digest', 'does not bind current package inputs')
  if (candidate.runtime_verification?.move_test?.total !== candidate.runtime_verification?.move_test?.passed + candidate.runtime_verification?.move_test?.failed) add('$.runtime_verification.move_test', 'test totals do not reconcile')
  if (candidate.chain_evidence?.status !== 'not_present') add('$.chain_evidence.status', 'local fixture cannot claim chain evidence')
  for (const [key, value] of Object.entries(candidate.chain_evidence || {})) {
    if (key !== 'status' && value !== null) add(`$.chain_evidence.${key}`, 'must remain null')
  }
  for (const [key, value] of Object.entries(candidate.assurances || {})) {
    if (value !== false) add(`$.assurances.${key}`, 'must remain false')
  }
  allocations.forEach((item, index) => {
    const actual = [item.base_units, item.basis_points, item.recipient_role]
    if (!deepEqual(actual, expectedSyntheticExample[item.bucket_id])) add(`$.allocations[${index}]`, 'synthetic example changed without review')
  })
  if (candidate.asset.total_base_units !== currencyManifest.supply.base_units) add('$.asset.total_base_units', 'must describe the verified ESK fixed supply')
  if (currencyManifest.toolchain.move_source_digest !== canonicalSha256(currencySourcePath)) add('$.currency_manifest', 'currency source digest is stale')
  if (currencyManifest.toolchain.move_test_source_digest !== canonicalSha256(currencyTestsPath)) add('$.currency_manifest', 'currency test digest is stale')
  if (currencyManifest.toolchain.package_input_digest !== packageInputDigest('contracts/sui/esk_currency')) add('$.currency_manifest', 'currency package input digest is stale')
  return errors
}

function allErrors(candidate) {
  return [...validate(schema, candidate), ...semanticErrors(candidate)]
}

function clone(value) {
  return JSON.parse(JSON.stringify(value))
}

function assertInvalid(mutator, expectedFragment) {
  const candidate = clone(fixture)
  mutator(candidate)
  const errors = allErrors(candidate)
  assert.ok(errors.length > 0, `negative case unexpectedly passed: ${expectedFragment}`)
  assert.ok(errors.some((error) => error.includes(expectedFragment)), `negative case missed ${expectedFragment}: ${errors.join(' | ')}`)
}

function mustMatch(text, pattern, message) {
  assert.match(text, pattern, message)
}

const schemaErrors = validate(schema, fixture)
assert.deepEqual(schemaErrors, [], schemaErrors.join('\n'))
const fixtureErrors = semanticErrors(fixture)
assert.deepEqual(fixtureErrors, [], fixtureErrors.join('\n'))
assert.equal(schema.additionalProperties, false)
assert.equal(schema.properties.runtime_verification.additionalProperties, false)

mustMatch(moveToml, /name\s*=\s*"yilong_participation"/, 'package name must remain independent')
mustMatch(moveToml, /Sui\s*=\s*\{[^\n]*rev\s*=\s*"46f18562f1f5af2438d35828e8b62d5e0b972db7"[^\n]*\}/, 'Sui dependency must stay pinned')
mustMatch(moveToml, /esk_currency\s*=\s*\{\s*local\s*=\s*"\.\.\/esk_currency"\s*\}/, 'package must use the local ESK type')
mustMatch(moveToml, /yilong_participation\s*=\s*"0x0"/, 'package address must remain unpublished')

mustMatch(genesisSource, /public struct GenesisAllocationCap has key, store\s*\{/, 'one-shot cap must be a key object')
mustMatch(genesisSource, /public struct GenesisAllocationReceipt has key\s*\{/, 'receipt must lack store')
mustMatch(genesisSource, /let GenesisAllocationCap \{ id \} = cap;\s*id\.delete\(\);/, 'allocation must consume the unique cap')
mustMatch(genesisSource, /transfer::freeze_object\(receipt\);/, 'allocation receipt must be frozen')
mustMatch(genesisSource, /total_base_units == esk::total_supply_base_units\(\)/, 'input must be the complete fixed supply')
assert.equal((genesisSource.match(/ as u128/g) || []).length >= 7, true, 'sum must use u128 intermediates')
assert.equal((genesisSource.match(/coin::split\(/g) || []).length, 5, 'five splits plus the remainder must create six buckets')
for (const transfer of [
  /public_transfer\(user_migration_coin, distribution\)/,
  /public_transfer\(community_coin, distribution\)/,
  /public_transfer\(project_treasury_coin, treasury\)/,
  /public_transfer\(supply, treasury\)/,
  /public_transfer\(liquidity_coin, liquidity_recipient\)/,
]) mustMatch(genesisSource, transfer, 'ordinary bucket transfer mapping changed')

const vestingAbilities = vestingSource.match(/public struct TeamVesting has\s+([^\{]+)\{/)?.[1]?.trim()
assert.equal(vestingAbilities, 'key', 'TeamVesting must have key and must not have store')
mustMatch(vestingSource, /remaining:\s*Balance<ESK>/, 'vesting must custody ESK balance')
mustMatch(vestingSource, /public\(package\) fun create_and_transfer\(/, 'only this package may create a team lock')
mustMatch(vestingSource, /transfer::transfer\(vesting, beneficiary\)/, 'lock must go directly to its fixed beneficiary')
mustMatch(vestingSource, /ctx\.sender\(\) == vesting\.beneficiary/, 'claim must authenticate the beneficiary')
mustMatch(vestingSource, /public_transfer\(claimed_coin, vesting\.beneficiary\)/, 'claim cannot choose another recipient')
mustMatch(vestingSource, /now_ms >= vesting\.end_ms[\s\S]*vesting\.total_base_units/, 'end must release every remaining unit')
mustMatch(vestingSource, /\(vesting\.total_base_units as u128\) \* elapsed/, 'linear calculation must multiply in u128')
mustMatch(vestingSource, /balance::value\(&vesting\.remaining\) ==[\s\S]*vesting\.total_base_units - vesting\.claimed_base_units/, 'vesting conservation invariant is required')

const productionCode = `${genesisSource}\n${vestingSource}`.replace(/\/\/.*$/gm, '').replace(/\/\*[\s\S]*?\*\//g, '')
for (const forbidden of [
  /TreasuryCap<ESK>/i,
  /coin::mint/i,
  /coin::burn/i,
  /\b(?:revoke|recover|clawback|early_unlock|change_beneficiary|change_schedule|admin_claim|withdraw)\b/i,
  /\b(?:qshare|usdt|task_sui|legacy|paper)\b/i,
]) assert.doesNotMatch(productionCode, forbidden, `forbidden production capability found: ${forbidden}`)

const testCode = `${genesisTests}\n${vestingTests}`
const expectedTests = [
  'exactly_six_buckets_are_delivered_and_receipted_once',
  'reject_any_zero_bucket',
  'reject_bucket_sum_mismatch',
  'reject_coin_that_is_not_the_complete_fixed_supply',
  'reject_non_increasing_schedule',
  'reject_past_start_or_invalid_schedule',
  'reject_wrong_manifest_digest_length',
  'reject_zero_or_repeated_roles',
  'reject_zero_role_address',
  'claim_before_cliff_fails_instead_of_minting_zero_coin',
  'cliff_midpoint_and_end_release_every_unit_without_dust',
  'non_beneficiary_cannot_claim_even_when_amount_is_available',
  'same_millisecond_second_claim_rejects_zero_amount',
]
assert.equal((testCode.match(/#\[test(?:,|\])/g) || []).length, expectedTests.length, 'Move test count changed')
for (const name of expectedTests) mustMatch(testCode, new RegExp(`fun ${name}\\(`), `missing Move test ${name}`)
assert.doesNotMatch(testCode, /\babort\s+0\s*;/, 'expected-failure tests must not fake their abort')
assert.equal(sha256(Buffer.from(moveTestEvidence, 'utf8')), fixture.runtime_verification.move_test.evidence_digest, 'committed Move test output must match its evidence digest')
mustMatch(moveTestEvidence, /Test result: OK\. Total tests: 13; passed: 13; failed: 0\n$/, 'participation test receipt must prove exactly 13 passing tests')
for (const name of expectedTests) mustMatch(moveTestEvidence, new RegExp(`::${name}(?:\\n|$)`), `runtime output is missing ${name}`)
assert.equal(sha256(Buffer.from(currencyTestEvidence, 'utf8')), currencyManifest.toolchain.test_evidence_digest, 'committed currency regression output must match its evidence digest')
mustMatch(currencyTestEvidence, /Test result: OK\. Total tests: 3; passed: 3; failed: 0\n$/, 'currency regression receipt must prove exactly 3 passing tests')

mustMatch(requirement, /implementation_status:\s*local_verified/, 'requirement status must reflect the runtime result')
mustMatch(requirement, /新版直接切换/, 'requirement must reject an old-version compatibility layer')
mustMatch(decision, /本功能只有 V1 新协议/, 'ADR must record the new-only cutover')
mustMatch(decision, /未签名、未广播/, 'ADR must preserve the no-chain-write boundary')
assert.doesNotMatch(JSON.stringify(fixture), /vesting_policy_ref/, 'new policy must not retain the old allocation field bridge')

assertInvalid((value) => { value.unexpected = true }, '$: unexpected')
assertInvalid((value) => { value.allocations[1].bucket_id = value.allocations[0].bucket_id }, '$.allocations')
assertInvalid((value) => { value.allocations[0].base_units = '250000000000001' }, '$.allocations')
assertInvalid((value) => { value.allocations[0].base_units = '0' }, '$.allocations[0].base_units')
assertInvalid((value) => { value.holders.treasury = value.holders.distribution }, '$.holders')
assertInvalid((value) => { value.holders.treasury = '0x22' }, '$.holders')
assertInvalid((value) => { value.team_vesting.cliff_ms = value.team_vesting.start_ms }, '$.team_vesting')
assertInvalid((value) => { value.runtime_verification.move_test.status = 'not_run' }, '$.runtime_verification.move_test.status')
assertInvalid((value) => { value.runtime_verification.package_input_digest = `sha256:${'0'.repeat(64)}` }, '$.runtime_verification.package_input_digest')
assertInvalid((value) => { value.chain_evidence.package_id = `0x${'1'.repeat(64)}` }, '$.chain_evidence.package_id')
assertInvalid((value) => { value.assurances.funds_moved = true }, '$.assurances.funds_moved')
assertInvalid((value) => { value.manifest_digest = 'sha256:00' }, '$.manifest_digest')

console.log('PASS allocation policy schema and synthetic fixture')
console.log('PASS six-bucket conservation, role mapping, and source binding')
console.log('PASS one-shot allocation and immutable-beneficiary vesting boundaries')
console.log('PASS 13 Move scenarios are present and cannot fake expected failures')
console.log('PASS local verification is recorded without chain or real-holder claims')
