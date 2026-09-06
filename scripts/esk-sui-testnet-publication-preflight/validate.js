'use strict'

const {
  CANDIDATE_SCHEMA, MAX_GAS_BUDGET_MIST, exactKeys, literal, oneOf,
  asciiIdentifier, digest, commit, decimal, integer, timestamp, rejectSecretMaterial,
  fail,
} = require('./contract')
const { APPROVED_BASELINE_COMMIT, FIXED } = require('./repository')
const {
  ALLOCATIONS, ROLE_NAMES, GAS_NAMES, APPROVAL_NAMES,
  UNRESOLVED_COMMERCIAL_TERMS, COMMERCIAL_POLICY_REVISION,
} = require('./template')

const ROOT_KEYS = [
  'schema', 'candidate_id', 'scope', 'repository', 'toolchain', 'packages', 'asset',
  'commercial_policy_revision', 'allocations', 'roles', 'team_vesting',
  'upgrade_policies', 'gas_budgets', 'approvals',
]
const POLICIES = ['pending', 'compatible', 'additive', 'dep_only', 'immutable']

function validateFixedObjects(candidate) {
  exactKeys(candidate.toolchain, [
    'sui_release', 'sui_cli_version', 'sui_source_commit', 'sui_cli_sha256',
    'framework_source_archive_sha256', 'framework_tracked_content_sha256',
  ])
  literal(candidate.toolchain.sui_release, FIXED.sui_release)
  literal(candidate.toolchain.sui_cli_version, FIXED.sui_cli_version)
  literal(candidate.toolchain.sui_source_commit, FIXED.sui_source_commit)
  literal(candidate.toolchain.sui_cli_sha256, FIXED.sui_cli_sha256)
  literal(candidate.toolchain.framework_source_archive_sha256,
    FIXED.framework_source_archive_sha256)
  literal(candidate.toolchain.framework_tracked_content_sha256, FIXED.framework_content_digest)

  exactKeys(candidate.packages, ['currency', 'participation'])
  exactKeys(candidate.packages.currency,
    ['package_id', 'source_path', 'package_input_digest', 'production_bytecode_digest'])
  exactKeys(candidate.packages.participation,
    ['package_id', 'source_path', 'package_input_digest', 'production_bytecode_digest',
      'dependency_binding'])
  const packageChecks = [
    [candidate.packages.currency.package_id, 'esk_currency'],
    [candidate.packages.currency.source_path, 'contracts/sui/esk_currency'],
    [candidate.packages.currency.package_input_digest, FIXED.currency_package_input_digest],
    [candidate.packages.currency.production_bytecode_digest,
      FIXED.currency_production_bytecode_digest],
    [candidate.packages.participation.package_id, 'yilong_participation'],
    [candidate.packages.participation.source_path, 'contracts/sui/yilong_participation'],
    [candidate.packages.participation.package_input_digest,
      FIXED.participation_package_input_digest],
    [candidate.packages.participation.production_bytecode_digest,
      FIXED.participation_local_production_bytecode_digest],
    [candidate.packages.participation.dependency_binding, 'local_0x0_not_publishable'],
  ]
  for (const [actual, expected] of packageChecks) literal(actual, expected)

  exactKeys(candidate.asset, [
    'asset_id', 'symbol', 'name', 'decimals', 'total_display_units', 'total_base_units',
  ])
  for (const [actual, expected] of [
    [candidate.asset.asset_id, 'esk'], [candidate.asset.symbol, 'ESK'],
    [candidate.asset.name, 'Yilong ESK'], [candidate.asset.decimals, 6],
    [candidate.asset.total_display_units, '1000000000'],
    [candidate.asset.total_base_units, '1000000000000000'],
  ]) literal(actual, expected)
}

function validateAllocations(candidate) {
  if (!Array.isArray(candidate.allocations) || candidate.allocations.length !== ALLOCATIONS.length) {
    fail('INVALID_CANDIDATE')
  }
  let unitSum = 0n
  let pointSum = 0
  candidate.allocations.forEach((allocation, index) => {
    exactKeys(allocation, ['bucket_id', 'base_units', 'basis_points', 'recipient_role'])
    const [bucket, units, points, role] = ALLOCATIONS[index]
    literal(allocation.bucket_id, bucket)
    literal(allocation.base_units, units)
    literal(allocation.basis_points, points)
    literal(allocation.recipient_role, role)
    unitSum += decimal(allocation.base_units, { minimum: 1n })
    pointSum += integer(allocation.basis_points, 1, 10_000)
    if (BigInt(allocation.base_units) * 10_000n !==
        BigInt(candidate.asset.total_base_units) * BigInt(allocation.basis_points)) {
      fail('INVALID_CANDIDATE')
    }
  })
  if (unitSum !== BigInt(candidate.asset.total_base_units) || pointSum !== 10_000) {
    fail('INVALID_CANDIDATE')
  }
}

function validateCommercialPolicyRevision(candidate) {
  const policy = candidate.commercial_policy_revision
  exactKeys(policy, [
    'schema', 'status', 'scope', 'known_intent', 'intended_term_months',
    'qshare_application', 'unresolved_terms', 'approved_policy_digest',
    'legacy_no_protection_terms_promotable', 'sample_terms_promotable',
    'public_sale_activation_allowed', 'technical_testnet_preflight_allowed',
    'funds_acceptance_investment_or_return_automation_allowed',
  ])
  for (const key of Object.keys(COMMERCIAL_POLICY_REVISION)) {
    if (key === 'unresolved_terms') continue
    literal(policy[key], COMMERCIAL_POLICY_REVISION[key])
  }
  if (!Array.isArray(policy.unresolved_terms) ||
      policy.unresolved_terms.length !== UNRESOLVED_COMMERCIAL_TERMS.length) {
    fail('INVALID_CANDIDATE')
  }
  policy.unresolved_terms.forEach((term, index) => {
    literal(term, UNRESOLVED_COMMERCIAL_TERMS[index])
  })
}

function roleAddress(value, mode) {
  if (mode === 'template') {
    if (value !== null) fail('INVALID_CANDIDATE')
    return value
  }
  const pattern = mode === 'synthetic_test'
    ? /^synthetic:sui:0x[0-9a-f]{64}$/
    : /^0x[0-9a-f]{64}$/
  if (typeof value !== 'string' || !pattern.test(value)) fail('INVALID_CANDIDATE')
  const raw = value.slice(value.lastIndexOf('0x') + 2)
  if (/^0{64}$/.test(raw)) fail('INVALID_CANDIDATE')
  return value
}

function validateRoles(candidate, mode) {
  exactKeys(candidate.roles, ROLE_NAMES)
  const addresses = ROLE_NAMES.map((name) => roleAddress(candidate.roles[name], mode))
  if (mode !== 'template' && new Set(addresses).size !== addresses.length) {
    fail('INVALID_CANDIDATE')
  }
}

function validateVesting(candidate, mode, reviewedMs) {
  const vesting = candidate.team_vesting
  exactKeys(vesting, [
    'model', 'beneficiary_role', 'start_ms', 'cliff_ms', 'end_ms', 'revocable',
    'recoverable', 'beneficiary_changeable', 'early_unlock', 'admin_claim',
  ])
  literal(vesting.model, 'linear_floor_u128')
  literal(vesting.beneficiary_role, 'team_beneficiary')
  for (const key of [
    'revocable', 'recoverable', 'beneficiary_changeable', 'early_unlock', 'admin_claim',
  ]) literal(vesting[key], false)
  if (mode === 'template') {
    for (const key of ['start_ms', 'cliff_ms', 'end_ms']) literal(vesting[key], null)
    return
  }
  const timestampBounds = {
    minimum: 1_000_000_000_000n,
    maximum: 9_999_999_999_999_999n,
  }
  const start = decimal(vesting.start_ms, timestampBounds)
  const cliff = decimal(vesting.cliff_ms, timestampBounds)
  const end = decimal(vesting.end_ms, timestampBounds)
  if (!(start < cliff && cliff < end) || start < BigInt(reviewedMs)) fail('INVALID_CANDIDATE')
}

function validatePoliciesAndGas(candidate, mode) {
  exactKeys(candidate.upgrade_policies, ['currency', 'participation'])
  exactKeys(candidate.gas_budgets, GAS_NAMES)
  for (const key of ['currency', 'participation']) {
    oneOf(candidate.upgrade_policies[key], POLICIES)
    if (mode === 'template') literal(candidate.upgrade_policies[key], 'pending')
    else if (candidate.upgrade_policies[key] === 'pending') fail('INVALID_CANDIDATE')
  }
  for (const key of GAS_NAMES) {
    if (mode === 'template') literal(candidate.gas_budgets[key], null)
    else integer(candidate.gas_budgets[key], 1, Number(MAX_GAS_BUDGET_MIST))
  }
}

function validateApprovals(candidate, mode, reviewedMs) {
  exactKeys(candidate.approvals, APPROVAL_NAMES)
  for (const name of APPROVAL_NAMES) {
    const approval = candidate.approvals[name]
    exactKeys(approval, ['digest', 'approved_at', 'expires_at'])
    if (mode === 'template') {
      literal(approval.digest, null)
      literal(approval.approved_at, null)
      literal(approval.expires_at, null)
      continue
    }
    digest(approval.digest)
    const approvedMs = timestamp(approval.approved_at)
    const expiresMs = timestamp(approval.expires_at)
    if (approvedMs > reviewedMs || reviewedMs >= expiresMs || approvedMs >= expiresMs) {
      fail('INVALID_CANDIDATE')
    }
  }
}

function validateCandidate(candidate) {
  rejectSecretMaterial(candidate)
  exactKeys(candidate, ROOT_KEYS)
  literal(candidate.schema, CANDIDATE_SCHEMA)
  asciiIdentifier(candidate.candidate_id, /^esk-sui-testnet-[a-z0-9-]{3,48}-v[1-9][0-9]*$/)
  exactKeys(candidate.scope, ['network', 'mode', 'synthetic', 'reviewed_at'])
  literal(candidate.scope.network, 'testnet')
  const mode = oneOf(candidate.scope.mode, ['template', 'synthetic_test', 'release_candidate'])
  literal(candidate.scope.synthetic, mode === 'synthetic_test')
  exactKeys(candidate.repository, ['baseline_commit'])
  let reviewedMs = null
  if (mode === 'template') {
    literal(candidate.scope.reviewed_at, null)
    literal(candidate.repository.baseline_commit, null)
  } else {
    reviewedMs = timestamp(candidate.scope.reviewed_at)
    literal(commit(candidate.repository.baseline_commit), APPROVED_BASELINE_COMMIT)
  }
  validateFixedObjects(candidate)
  validateCommercialPolicyRevision(candidate)
  validateAllocations(candidate)
  validateRoles(candidate, mode)
  validateVesting(candidate, mode, reviewedMs)
  validatePoliciesAndGas(candidate, mode)
  validateApprovals(candidate, mode, reviewedMs)
  return candidate
}

module.exports = { ROOT_KEYS, POLICIES, validateCandidate }
