'use strict'

const { PREFLIGHT_SCHEMA } = require('./contract')
const { canonicalDigest, canonicalJson } = require('./canonical')
const { FIXED } = require('./repository')
const { ROLE_NAMES } = require('./template')
const { executionContract } = require('./execution-contract')

const CHAIN_OUTPUT_NAMES = Object.freeze([
  'chain_identifier', 'rpc_endpoint', 'transaction_bytes',
  'ptb_bytes', 'signature',
  'currency_publish_gas_payment_object_ref',
  'currency_registry_finalize_gas_payment_object_ref',
  'participation_publish_gas_payment_object_ref',
  'genesis_allocation_gas_payment_object_ref',
  'capability_handoff_gas_payment_object_ref',
  'coin_registry_initial_shared_version',
  'currency_package_id', 'pending_currency_object_id',
  'pending_currency_object_version', 'pending_currency_object_digest',
  'registered_currency_object_id', 'metadata_cap_object_id',
  'registered_currency_creation_version',
  'currency_upgrade_cap_object_id', 'initial_supply_coin_object_id',
  'currency_publish_tx_digest', 'currency_publish_checkpoint',
  'currency_registration_tx_digest', 'currency_registration_checkpoint',
  'participation_package_id', 'participation_upgrade_cap_object_id',
  'genesis_allocation_cap_object_id',
  'participation_publish_tx_digest', 'participation_publish_checkpoint',
  'allocation_receipt_object_id', 'team_vesting_object_id', 'allocation_tx_digest',
  'allocation_checkpoint', 'capability_handoff_tx_digest', 'capability_handoff_checkpoint',
])

function chainOutputs() {
  return Object.fromEntries(CHAIN_OUTPUT_NAMES.map((name) => [name, null]))
}

function step(sequence, step_id, kind, depends_on, authorization_roles, public_inputs,
  pending_outputs, max_gas_budget, unknown_result_policy) {
  return {
    sequence, step_id, kind, depends_on, authorization_roles, public_inputs,
    pending_outputs, max_gas_budget, stop_on_failure: true, unknown_result_policy,
    state: 'planned',
  }
}

const UPGRADE_POLICY_ACTIONS = Object.freeze({
  pending: 'pending_policy_selection',
  compatible: 'verify_upgrade_cap_policy_0_then_transfer',
  additive: 'call_0x2_package_only_additive_upgrades_then_transfer',
  dep_only: 'call_0x2_package_only_dep_upgrades_then_transfer',
  immutable: 'call_0x2_package_make_immutable_and_destroy_cap',
})

function upgradeCapabilityDisposition(packageName, selectedPolicy) {
  const transferable = selectedPolicy !== 'pending' && selectedPolicy !== 'immutable'
  return {
    source_capability_input: `${packageName}_upgrade_cap_object_id`,
    requested_policy: selectedPolicy,
    expected_final_policy: selectedPolicy === 'pending' ? null : selectedPolicy,
    policy_action: UPGRADE_POLICY_ACTIONS[selectedPolicy],
    disposition: selectedPolicy === 'pending'
      ? 'pending'
      : transferable ? 'transfer_to_upgrade_role' : 'destroy_without_final_owner',
    recipient_role: transferable ? `${packageName}_upgrade` : null,
    final_owner_evidence_expected: transferable,
  }
}

function upgradeCapabilityDispositions(candidate) {
  return {
    currency: upgradeCapabilityDisposition('currency', candidate.upgrade_policies.currency),
    participation: upgradeCapabilityDisposition(
      'participation', candidate.upgrade_policies.participation),
  }
}

function createDag(candidate) {
  const gas = candidate.gas_budgets
  const chainPolicy = 'query_by_known_transaction_digest_or_stable_request_key_before_retry'
  const offlinePolicy = 'not_applicable_offchain'
  return [
    step(1, 'currency_publish', 'chain_transaction', [], ['deployer', 'gas_sponsor'], [
      'packages.currency.source_path', 'packages.currency.production_bytecode_digest',
      'roles.deployer', 'roles.gas_sponsor', 'gas_budgets.currency_publish',
    ], [
      'currency_package_id', 'pending_currency_object_id', 'metadata_cap_object_id',
      'pending_currency_object_version', 'pending_currency_object_digest',
      'currency_upgrade_cap_object_id', 'initial_supply_coin_object_id',
      'currency_publish_tx_digest', 'currency_publish_checkpoint',
    ], gas.currency_publish, chainPolicy),
    step(2, 'currency_registry_finalize', 'chain_transaction', ['currency_publish'],
      ['deployer', 'gas_sponsor'], [
        'sui.coin_registry.mutable_shared_object_ref', 'currency_package_id',
        'pending_currency_object_id', 'pending_currency_object_version',
        'pending_currency_object_digest',
        'roles.deployer', 'roles.gas_sponsor',
        'gas_budgets.currency_registry_finalize',
      ], [
        'registered_currency_object_id', 'registered_currency_creation_version',
        'currency_registration_tx_digest',
        'currency_registration_checkpoint',
      ], gas.currency_registry_finalize, chainPolicy),
    step(3, 'participation_rebind_rebuild_test', 'offline_build',
      ['currency_registry_finalize'], [], [
        'currency_package_id', 'packages.participation.source_path',
        'packages.participation.dependency_binding', 'toolchain.fixed_build_test',
      ], [
        'participation.rebound_package_input_digest',
        'participation.rebound_production_bytecode_digest',
        'participation.rebuild_test_receipt_digest',
    ], null, offlinePolicy),
    step(4, 'participation_publish', 'chain_transaction',
      ['participation_rebind_rebuild_test'], ['deployer', 'gas_sponsor'], [
        'participation.rebound_production_bytecode_digest', 'roles.deployer',
        'roles.gas_sponsor', 'gas_budgets.participation_publish',
      ], [
        'participation_package_id', 'participation_upgrade_cap_object_id',
        'genesis_allocation_cap_object_id', 'participation_publish_tx_digest',
        'participation_publish_checkpoint',
    ], gas.participation_publish, chainPolicy),
    step(5, 'genesis_allocation_and_team_vesting', 'chain_transaction',
      ['participation_publish'], ['deployer', 'gas_sponsor'], [
        'initial_supply_coin_object_id', 'genesis_allocation_cap_object_id',
        'sui.clock.immutable_shared_object_ref',
        'plan_sha256.hex_body_decoded_to_vector_u8',
        'roles.deployer', 'roles.gas_sponsor',
        'asset.total_base_units', 'allocations.six_buckets', 'roles.distribution',
        'roles.team_beneficiary', 'roles.treasury', 'roles.liquidity',
        'team_vesting.schedule', 'gas_budgets.genesis_allocation',
      ], [
        'allocation_receipt_object_id', 'team_vesting_object_id',
        'allocation.bucket_coin_object_ids', 'allocation_tx_digest', 'allocation_checkpoint',
      ], gas.genesis_allocation, chainPolicy),
    {
      ...step(6, 'capability_handoff', 'chain_transaction',
        ['genesis_allocation_and_team_vesting'], ['deployer', 'gas_sponsor'], [
        'metadata_cap_object_id', 'currency_upgrade_cap_object_id',
        'participation_upgrade_cap_object_id', 'upgrade_policies.currency',
        'upgrade_policies.participation', 'roles.deployer', 'roles.gas_sponsor',
        'roles.metadata',
        'roles.currency_upgrade', 'roles.participation_upgrade',
        'gas_budgets.capability_handoff',
      ], [
        'capability_handoff_tx_digest', 'capability_handoff_checkpoint',
        'capability.metadata.final_owner_evidence',
        'capability.currency_upgrade.final_policy',
        'capability.currency_upgrade.disposition',
        'capability.currency_upgrade.disposition_evidence',
        'capability.participation_upgrade.final_policy',
        'capability.participation_upgrade.disposition',
        'capability.participation_upgrade.disposition_evidence',
      ], gas.capability_handoff, chainPolicy),
      upgrade_capability_dispositions: upgradeCapabilityDispositions(candidate),
    },
    step(7, 'three_observer_and_three_verifier_evidence_gate', 'offline_verification',
      ['capability_handoff'], [], [
        'observer.publication', 'observer.currency', 'observer.allocation',
        'verifier.capability_handoff', 'verifier.source_correspondence',
        'verifier.committee_finality',
        'observer.dual_source_required',
      ], [
        'observer.publication.report_sha256', 'observer.currency.report_sha256',
        'observer.allocation.report_sha256',
        'verifier.capability_handoff.report_sha256',
        'verifier.source_correspondence.report_sha256',
        'verifier.committee_finality.report_sha256',
      ], null, offlinePolicy),
    step(8, 'evidence_manifest_v2_handoff', 'evidence_handoff',
      ['three_observer_and_three_verifier_evidence_gate'], [], [
        'observer_reports.three_verified',
        'verifier_reports.capability_handoff_verified',
        'verifier_reports.source_correspondence_verified',
        'verifier_reports.committee_finality_verified',
      ], [
        'evidence_v2.sha256', 'manifest_v2.sha256', 'platform_transition.review',
      ], null, offlinePolicy),
  ]
}

const OBSERVER_CLAIMS = Object.freeze({
  publication: Object.freeze([
    'package_transaction_checkpoint_dual_source_consistency',
    'trust_basis_rpc_reports_without_committee_signature_verification',
  ]),
  currency: Object.freeze([
    'registered_currency_creation_version', 'fixed_supply_and_metadata',
  ]),
  allocation: Object.freeze([
    'allocator_equals_roles.deployer', 'six_bucket_conservation_and_team_vesting',
  ]),
})

function observer(observer_id, report_schema) {
  return {
    observer_id, report_schema, status: 'not_run', dual_source_required: true,
    required_claims: [...OBSERVER_CLAIMS[observer_id]],
    official_endpoint: null, secondary_endpoint: null, expected_input_sha256: null,
    report_sha256: null, observed_at: null, error_code: null,
    manifest_transition_allowed: false,
  }
}

function observerTemplates() {
  return [
    observer('publication', 'yilong.esk.sui.publication_observation.v1'),
    observer('currency', 'yilong.esk.sui.currency_observation.v1'),
    observer('allocation', 'yilong.esk.sui.allocation_observation.v1'),
  ]
}

function recoveryGate(sequence, gate_id, recovery_action) {
  return {
    sequence, gate_id, status: 'not_run', evidence_sha256: null,
    blocks_progress: true, recovery_action,
  }
}

function recoveryChecklist() {
  return {
    attempt_journal_required: true,
    attempt_journal_append_only: true,
    attempt_journal_created: false,
    blind_resubmit_allowed: false,
    unknown_result_resolution:
      'query_by_known_transaction_digest_or_stable_request_key_before_retry',
    chain_success_database_rollback_allowed: false,
    gates: [
      recoveryGate(1, 'currency_registration_incomplete',
        'preserve_currency_evidence_and_stop_before_participation'),
      recoveryGate(2, 'participation_rebuild_or_publish_incomplete',
        'rebind_to_final_currency_package_then_rebuild_retest_or_stop'),
      recoveryGate(3, 'allocation_incomplete',
        'query_allocation_transaction_and_objects_then_stop_without_resubmission'),
      recoveryGate(4, 'capability_handoff_incomplete',
        'verify_cap_ownership_before_any_follow_on_action'),
      recoveryGate(5, 'observer_disagreement',
        'quarantine_results_until_both_sources_agree'),
      recoveryGate(6, 'finality_or_evidence_incomplete',
        'withhold_balance_and_manifest_transition_until_finality_and_v2_evidence'),
    ],
  }
}

function statusProjection(mode) {
  if (mode === 'template') return {
    candidate_status: 'user_action_required',
    blocking_reasons: [
      'REPOSITORY_BASELINE_COMMIT_REQUIRED', 'PUBLIC_ROLE_ADDRESSES_REQUIRED',
      'TEAM_VESTING_SCHEDULE_REQUIRED', 'UPGRADE_POLICIES_REQUIRED',
      'GAS_BUDGETS_REQUIRED', 'APPROVAL_EVIDENCE_REQUIRED',
      'COMMERCIAL_POLICY_REVISION_REQUIRED_BEFORE_PUBLIC_SALE',
      'EXPLICIT_EXECUTION_AUTHORIZATION_ABSENT',
    ],
    user_actions_required: [
      'FILL_PUBLIC_RELEASE_CANDIDATE_PARAMETERS',
      'REVIEW_AND_APPROVE_RELEASE_CANDIDATE',
      'CLARIFY_AND_DECIDE_ESK_COMMERCIAL_POLICY',
    ],
    next_safe_action: 'complete_and_review_candidate',
  }
  if (mode === 'synthetic_test') return {
    candidate_status: 'synthetic_verified',
    blocking_reasons: [
      'SYNTHETIC_CANDIDATE_NEVER_EXECUTABLE',
      'SAMPLE_OR_LEGACY_COMMERCIAL_TERMS_NEVER_PROMOTABLE',
      'EXPLICIT_EXECUTION_AUTHORIZATION_ABSENT',
    ],
    user_actions_required: [
      'DO_NOT_USE_SYNTHETIC_VALUES_FOR_PUBLICATION',
      'DO_NOT_PROMOTE_SAMPLE_OR_LEGACY_TERMS_TO_PUBLIC_SALE',
    ],
    next_safe_action: 'no_execution_synthetic_test_only',
  }
  return {
    candidate_status: 'prepared_not_authorized',
    blocking_reasons: [
      'EXPLICIT_EXECUTION_AUTHORIZATION_ABSENT',
      'SEPARATE_TESTNET_PUBLICATION_FEATURE_REQUIRED',
      'APPROVAL_AUTHENTICITY_AND_SUBJECT_BINDING_UNVERIFIED',
      'CAPABILITY_HANDOFF_VERIFIER_IMPLEMENTATION_REQUIRED',
      'SOURCE_CORRESPONDENCE_VERIFIER_IMPLEMENTATION_REQUIRED',
      'COMMITTEE_FINALITY_VERIFIER_IMPLEMENTATION_REQUIRED',
      'POST_PLAN_STEP_SCOPED_EXECUTION_ATTESTATION_ABSENT',
      'COMMERCIAL_POLICY_REVISION_REQUIRED_BEFORE_PUBLIC_SALE',
    ],
    user_actions_required: [
      'AUTHENTICATE_CANDIDATE_APPROVALS_AGAINST_PRE_PLAN_SUBJECTS',
      'IMPLEMENT_AND_VERIFY_CAPABILITY_HANDOFF_VERIFIER',
      'IMPLEMENT_AND_VERIFY_SOURCE_CORRESPONDENCE_VERIFIER',
      'IMPLEMENT_AND_VERIFY_COMMITTEE_FINALITY_VERIFIER',
      'CREATE_AND_VERIFY_POST_PLAN_STEP_SCOPED_EXECUTION_ATTESTATION',
      'REQUEST_SEPARATE_STEP_SCOPED_TESTNET_PUBLICATION_AUTHORIZATION',
      'CLARIFY_AND_DECIDE_ESK_COMMERCIAL_POLICY_BEFORE_PUBLIC_SALE',
    ],
    next_safe_action: 'request_separate_testnet_publication_authorization',
  }
}

function createPreflightPlan(candidate, repository) {
  // Snapshot validated input so later caller mutations cannot rewrite a plan
  // without changing its already-computed digest.
  candidate = JSON.parse(canonicalJson(candidate))
  const mode = candidate.scope.mode
  const complete = mode !== 'template'
  const configuredRoles = ROLE_NAMES.map((name) => candidate.roles[name])
    .filter((value) => value !== null)
  const parameters = {
    scope: candidate.scope,
    asset: candidate.asset,
    commercial_policy_revision: candidate.commercial_policy_revision,
    allocations: candidate.allocations,
    team_vesting: candidate.team_vesting,
    upgrade_policies: candidate.upgrade_policies,
    gas_budgets: candidate.gas_budgets,
    approvals: candidate.approvals,
  }
  const state = statusProjection(mode)
  const unsigned = {
    schema: PREFLIGHT_SCHEMA,
    candidate_id: candidate.candidate_id,
    mode,
    candidate_status: state.candidate_status,
    repository_binding: {
      baseline_commit: candidate.repository.baseline_commit,
      status: complete && repository.repository_sources_verified ? 'verified' : 'blocked',
    },
    toolchain_binding: {
      status: 'verified',
      toolchain_contract_sha256: repository.toolchain_contract_sha256,
      sui_release: FIXED.sui_release,
      sui_cli_version: FIXED.sui_cli_version,
      sui_source_commit: FIXED.sui_source_commit,
      sui_cli_sha256: FIXED.sui_cli_sha256,
      framework_source_archive_sha256: FIXED.framework_source_archive_sha256,
      framework_tracked_content_sha256: FIXED.framework_content_digest,
      currency_package_input_digest: repository.currency_package_input_digest,
      currency_production_bytecode_digest: FIXED.currency_production_bytecode_digest,
      participation_package_input_digest: repository.participation_package_input_digest,
      participation_local_production_bytecode_digest:
        FIXED.participation_local_production_bytecode_digest,
      participation_dependency_binding: 'local_0x0_not_publishable',
    },
    parameter_summary: {
      sha256: canonicalDigest(parameters),
      total_base_units: candidate.asset.total_base_units,
      total_basis_points: candidate.allocations.reduce(
        (sum, item) => sum + item.basis_points, 0),
      team_vesting_complete: complete,
      upgrade_policies_complete: complete,
      gas_budgets_complete: complete,
      approvals_complete: complete,
      commercial_policy_revision: candidate.commercial_policy_revision,
      commercial_policy_revision_resolved: false,
    },
    role_summary: {
      sha256: canonicalDigest(candidate.roles),
      required_role_count: ROLE_NAMES.length,
      configured_role_count: configuredRoles.length,
      unique_role_count: new Set(configuredRoles).size,
      synthetic: mode === 'synthetic_test',
      role_controls_approval_digest_present: mode === 'release_candidate',
      multisig_recovery_approval_digest_present: mode === 'release_candidate',
      approval_authenticity_verified: false,
      approval_subject_binding_verified: false,
      pause_capability_status: 'reserved_without_current_capability',
    },
    execution_contract: executionContract(),
    dag: createDag(candidate),
    observer_templates: observerTemplates(),
    recovery_checklist: recoveryChecklist(),
    blocking_reasons: state.blocking_reasons,
    user_actions_required: state.user_actions_required,
    next_safe_action: state.next_safe_action,
    chain_outputs: chainOutputs(),
    safety: {
      default_sui_config_read: false,
      wallet_accessed: false,
      faucet_requested: false,
      gas_coins_queried: false,
      execution_authorized: false,
      transactions_constructed: false,
      transactions_signed: false,
      transactions_broadcast: false,
      rpc_queried: false,
      funds_moved: false,
      public_sale_activation_allowed: false,
      funds_acceptance_automation_allowed: false,
      investment_automation_allowed: false,
      return_or_top_up_automation_allowed: false,
      publication_status: 'not_performed',
      chain_finality_verified: false,
      asset_identity_verified: false,
      balance_eligible: false,
      manifest_transition_allowed: false,
    },
  }
  return { ...unsigned, plan_sha256: canonicalDigest(unsigned) }
}

module.exports = {
  CHAIN_OUTPUT_NAMES, chainOutputs, createDag, observerTemplates, recoveryChecklist,
  createPreflightPlan,
}
