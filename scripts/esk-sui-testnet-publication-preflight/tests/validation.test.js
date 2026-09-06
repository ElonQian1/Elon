'use strict'

const test = require('node:test')
const assert = require('node:assert/strict')
const { preflightCandidate } = require('../index')
const { clone, syntheticCandidate, releaseCandidate } = require('./fixtures')
const candidateSchema = require('../../../contracts/sui/esk-testnet-publication-candidate-v1.schema.json')

function expectCode(candidate, code = 'INVALID_CANDIDATE') {
  assert.throws(() => preflightCandidate(candidate), error => {
    assert.equal(error.code, code)
    assert.doesNotMatch(error.message, /0x[0-9a-f]{16}|sha256:[0-9a-f]{16}|Bearer|secret/i)
    return true
  })
}

test('fixed supply and all six allocation buckets must conserve units and basis points', () => {
  for (const mutate of [
    value => { value.asset.total_base_units = '1000000000000001' },
    value => { value.asset.decimals = 9 },
    value => { value.allocations[0].base_units = '249999999999999' },
    value => { value.allocations[5].basis_points = 501 },
    value => { value.allocations.pop() },
    value => { [value.allocations[0], value.allocations[1]] =
      [value.allocations[1], value.allocations[0]] },
  ]) {
    const candidate = releaseCandidate()
    mutate(candidate)
    expectCode(candidate)
  }
})

test('release roles reject zero, duplicate, synthetic and non-canonical addresses', () => {
  for (const mutate of [
    value => { value.roles.distribution = `0x${'0'.repeat(64)}` },
    value => { value.roles.treasury = value.roles.distribution },
    value => { value.roles.liquidity = `synthetic:sui:0x${'9'.repeat(64)}` },
    value => { value.roles.pause = `0x${'A'.repeat(64)}` },
    value => { value.roles.deployer = '0x1' },
  ]) {
    const candidate = releaseCandidate()
    mutate(candidate)
    expectCode(candidate)
  }
})

test('synthetic mode requires the synthetic flag and only isolated synthetic addresses', () => {
  for (const mutate of [
    value => { value.scope.synthetic = false },
    value => { value.roles.distribution = `0x${'1'.repeat(64)}` },
    value => { value.roles.treasury = `synthetic:sui:0x${'0'.repeat(64)}` },
    value => { value.roles.treasury = value.roles.distribution },
  ]) {
    const candidate = syntheticCandidate()
    mutate(candidate)
    expectCode(candidate)
  }
})

test('vesting and review dates require valid ordering and canonical UTC timestamps', () => {
  for (const mutate of [
    value => { value.team_vesting.cliff_ms = value.team_vesting.start_ms },
    value => { value.team_vesting.end_ms = value.team_vesting.cliff_ms },
    value => { value.team_vesting.start_ms = '1893456000000.0' },
    value => { value.scope.reviewed_at = '2026-09-06T12:00:00Z' },
    value => { value.scope.reviewed_at = '2026-02-30T12:00:00.000Z' },
    value => {
      value.team_vesting.start_ms = '10000000000000000'
      value.team_vesting.cliff_ms = '10000000000000001'
      value.team_vesting.end_ms = '10000000000000002'
    },
    value => { value.approvals.economic_parameters.approved_at =
      value.approvals.economic_parameters.expires_at },
    value => { value.approvals.role_controls.expires_at = '2026-09-06T11:59:59.999Z' },
  ]) {
    const candidate = releaseCandidate()
    mutate(candidate)
    expectCode(candidate)
  }
})

test('candidate schema and runtime share the exact millisecond timestamp domain', () => {
  const pattern = new RegExp(candidateSchema.$defs.rfc3339.pattern)
  assert.equal(pattern.test('2026-09-06T12:00:00.000Z'), true)
  for (const value of [
    '2026-09-06T12:00:00Z',
    '2026-09-06T12:00:00.0Z',
    '2026-09-06T12:00:00.0000Z',
  ]) assert.equal(pattern.test(value), false, value)
})

test('release candidate rejects pending upgrades, missing or expired approvals and excessive gas', () => {
  for (const mutate of [
    value => { value.upgrade_policies.currency = 'pending' },
    value => { value.upgrade_policies.participation = 'pending' },
    value => { value.approvals.economic_parameters.digest = null },
    value => { value.approvals.role_controls.digest = `sha256:${'0'.repeat(64)}` },
    value => { value.approvals.multisig_recovery.expires_at =
      value.scope.reviewed_at },
    value => { value.gas_budgets.currency_publish = null },
    value => { value.gas_budgets.capability_handoff = 1000000001 },
    value => { value.gas_budgets.genesis_allocation = 0 },
  ]) {
    const candidate = releaseCandidate()
    mutate(candidate)
    expectCode(candidate)
  }
})

test('chain results, unknown fields and secret material are rejected before planning', () => {
  for (const [key, value] of [
    ['chain_results', { package_id: `0x${'f'.repeat(64)}` }],
    ['currency_package_id', `0x${'f'.repeat(64)}`],
    ['object_id', `0x${'e'.repeat(64)}`],
    ['transaction_digest', '11111111111111111111111111111111'],
    ['checkpoint', 1],
  ]) {
    const chainResult = releaseCandidate()
    chainResult[key] = value
    expectCode(chainResult, 'UNKNOWN_FIELD')
  }

  const unknown = releaseCandidate()
  unknown.roles.operator = unknown.roles.deployer
  expectCode(unknown, 'UNKNOWN_FIELD')

  for (const secret of [
    { private_key: 'NEVER_PRINT' },
    { note: 'Bearer NEVER_PRINT' },
    { transaction_bytes: 'AA==' },
    { signature: 'AA==' },
    { gas_coin_object_id: `0x${'f'.repeat(64)}` },
  ]) {
    const candidate = releaseCandidate()
    Object.assign(candidate, secret)
    expectCode(candidate, 'SECRET_MATERIAL_REJECTED')
  }
})

test('pending commercial policy cannot be removed, resolved, or replaced by legacy terms', () => {
  for (const mutate of [
    value => { delete value.commercial_policy_revision },
    value => { value.commercial_policy_revision.status = 'approved' },
    value => { value.commercial_policy_revision.approved_policy_digest =
      `sha256:${'1'.repeat(64)}` },
    value => { value.commercial_policy_revision.public_sale_activation_allowed = true },
    value => { value.commercial_policy_revision.technical_testnet_preflight_allowed = false },
    value => { value.commercial_policy_revision.qshare_application = 'automatic' },
    value => { value.commercial_policy_revision.unresolved_terms.pop() },
  ]) {
    const candidate = releaseCandidate()
    mutate(candidate)
    expectCode(candidate)
  }
})

test('candidate repository and toolchain bindings cannot be substituted', () => {
  for (const mutate of [
    value => { value.repository.baseline_commit = '1'.repeat(40) },
    value => { value.toolchain.sui_release = 'mainnet-v1.79.0' },
    value => { value.toolchain.sui_cli_sha256 = `sha256:${'1'.repeat(64)}` },
    value => { value.packages.currency.production_bytecode_digest =
      `sha256:${'2'.repeat(64)}` },
    value => { value.packages.participation.dependency_binding = 'published' },
  ]) {
    const candidate = releaseCandidate()
    mutate(candidate)
    expectCode(candidate)
  }
})
