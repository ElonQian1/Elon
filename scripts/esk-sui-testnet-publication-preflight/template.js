'use strict'

const { CANDIDATE_SCHEMA } = require('./contract')
const { FIXED } = require('./repository')

const ALLOCATIONS = Object.freeze([
  ['user_migration_and_ecosystem', '250000000000000', 2500, 'distribution'],
  ['team_vesting', '200000000000000', 2000, 'team_vesting'],
  ['project_treasury', '250000000000000', 2500, 'treasury'],
  ['liquidity', '150000000000000', 1500, 'liquidity'],
  ['community_contributors', '100000000000000', 1000, 'distribution'],
  ['security_operations_reserve', '50000000000000', 500, 'treasury'],
])

const ROLE_NAMES = Object.freeze([
  'distribution', 'treasury', 'liquidity', 'team_beneficiary', 'metadata',
  'currency_upgrade', 'participation_upgrade', 'pause', 'gas_sponsor', 'deployer',
])
const GAS_NAMES = Object.freeze([
  'currency_publish', 'currency_registry_finalize', 'participation_publish',
  'genesis_allocation', 'capability_handoff',
])
const APPROVAL_NAMES = Object.freeze([
  'economic_parameters', 'role_controls', 'multisig_recovery',
  'release_candidate_review',
])
const UNRESOLVED_COMMERCIAL_TERMS = Object.freeze([
  'protected_amount_scope',
  'term_start_basis',
  'settlement_asset',
  'responsible_entity',
  'loss_cover_funding_source',
  'return_calculation_and_distribution_policy',
  'transfer_effect_on_protection_right',
  'service_consumption_effect_on_protection_right',
  'early_exit_terms',
  'maturity_settlement_terms',
  'investment_loss_and_team_top_up_accounting',
])
const COMMERCIAL_POLICY_REVISION = Object.freeze({
  schema: 'yilong.esk.commercial_policy_revision.v1',
  status: 'clarification_required',
  scope: 'esk_sale_proceeds_team_investment',
  known_intent: 'team_considers_two_year_loss_backstop',
  intended_term_months: 24,
  qshare_application: 'not_automatic',
  unresolved_terms: UNRESOLVED_COMMERCIAL_TERMS,
  approved_policy_digest: null,
  legacy_no_protection_terms_promotable: false,
  sample_terms_promotable: false,
  public_sale_activation_allowed: false,
  technical_testnet_preflight_allowed: true,
  funds_acceptance_investment_or_return_automation_allowed: false,
})

function nullRecord(names) {
  return Object.fromEntries(names.map((name) => [name, null]))
}

function createTemplate() {
  return {
    schema: CANDIDATE_SCHEMA,
    candidate_id: 'esk-sui-testnet-template-v1',
    scope: { network: 'testnet', mode: 'template', synthetic: false, reviewed_at: null },
    repository: { baseline_commit: null },
    toolchain: {
      sui_release: FIXED.sui_release,
      sui_cli_version: FIXED.sui_cli_version,
      sui_source_commit: FIXED.sui_source_commit,
      sui_cli_sha256: FIXED.sui_cli_sha256,
      framework_source_archive_sha256: FIXED.framework_source_archive_sha256,
      framework_tracked_content_sha256: FIXED.framework_content_digest,
    },
    packages: {
      currency: {
        package_id: 'esk_currency',
        source_path: 'contracts/sui/esk_currency',
        package_input_digest: FIXED.currency_package_input_digest,
        production_bytecode_digest: FIXED.currency_production_bytecode_digest,
      },
      participation: {
        package_id: 'yilong_participation',
        source_path: 'contracts/sui/yilong_participation',
        package_input_digest: FIXED.participation_package_input_digest,
        production_bytecode_digest: FIXED.participation_local_production_bytecode_digest,
        dependency_binding: 'local_0x0_not_publishable',
      },
    },
    asset: {
      asset_id: 'esk', symbol: 'ESK', name: 'Yilong ESK', decimals: 6,
      total_display_units: '1000000000', total_base_units: '1000000000000000',
    },
    commercial_policy_revision: {
      ...COMMERCIAL_POLICY_REVISION,
      unresolved_terms: [...UNRESOLVED_COMMERCIAL_TERMS],
    },
    allocations: ALLOCATIONS.map(([bucket_id, base_units, basis_points, recipient_role]) => ({
      bucket_id, base_units, basis_points, recipient_role,
    })),
    roles: nullRecord(ROLE_NAMES),
    team_vesting: {
      model: 'linear_floor_u128', beneficiary_role: 'team_beneficiary',
      start_ms: null, cliff_ms: null, end_ms: null, revocable: false,
      recoverable: false, beneficiary_changeable: false, early_unlock: false,
      admin_claim: false,
    },
    upgrade_policies: { currency: 'pending', participation: 'pending' },
    gas_budgets: nullRecord(GAS_NAMES),
    approvals: Object.fromEntries(APPROVAL_NAMES.map((name) => [name, {
      digest: null, approved_at: null, expires_at: null,
    }])),
  }
}

module.exports = {
  ALLOCATIONS, ROLE_NAMES, GAS_NAMES, APPROVAL_NAMES,
  UNRESOLVED_COMMERCIAL_TERMS, COMMERCIAL_POLICY_REVISION, createTemplate,
}
