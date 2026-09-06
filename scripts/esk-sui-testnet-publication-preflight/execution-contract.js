'use strict'

const CHAIN_STEPS = Object.freeze([
  ['currency_publish', 'currency_publish', 'currency_publish_gas_payment_object_ref'],
  ['currency_registry_finalize', 'currency_registry_finalize',
    'currency_registry_finalize_gas_payment_object_ref'],
  ['participation_publish', 'participation_publish',
    'participation_publish_gas_payment_object_ref'],
  ['genesis_allocation_and_team_vesting', 'genesis_allocation',
    'genesis_allocation_gas_payment_object_ref'],
  ['capability_handoff', 'capability_handoff',
    'capability_handoff_gas_payment_object_ref'],
])

function transactionEnvelopes() {
  return CHAIN_STEPS.map(([stepId, gasBudget, gasOutput]) => ({
    step_id: stepId,
    transaction_sender_role: 'deployer',
    gas_owner_role: 'gas_sponsor',
    tx_context_sender_role: 'deployer',
    gas_payment_object_ref_source: `attempt_journal.${stepId}.gas_payment_object_ref`,
    gas_payment_chain_output_slot: gasOutput,
    gas_payment_ref_components: ['object_id', 'version', 'digest'],
    gas_payment_expected_owner_role: 'gas_sponsor',
    gas_price_source: `attempt_journal.${stepId}.reference_gas_price`,
    gas_budget_source: `gas_budgets.${gasBudget}`,
    fresh_gas_object_ref_required: true,
    both_role_signatures_required: true,
  }))
}

function publishCommand(stepId, moduleNames, moduleBytesSource, moduleDigestSource,
  dependencyIds, chainOutputSlot) {
  const resultArgument = { kind: 'Result', command_index: 0 }
  return {
    step_id: stepId,
    move_init_tx_context_sender_role: 'deployer',
    ordered_commands: [
      {
        command_index: 0,
        command: 'Command::Publish',
        ordered_module_names: moduleNames,
        module_bytes_source: moduleBytesSource,
        module_bundle_digest_source: moduleDigestSource,
        transitive_dependency_ids: dependencyIds,
      },
      {
        command_index: 1,
        command: 'Command::TransferObjects',
        object_arguments: [resultArgument],
        recipient_address_source: 'roles.deployer',
      },
    ],
    publish_result_contract: {
      producer_command_index: 0,
      result_count: 1,
      result_0_type: '0x2::package::UpgradeCap',
      result_argument: resultArgument,
      consumer_command_index: 1,
      must_be_consumed_in_same_ptb: true,
      nested_result_forbidden: true,
      chain_output_object_id_slot: chainOutputSlot,
      final_effects_object_ref_source:
        `${stepId}.final_effects.${chainOutputSlot.replace('_object_id', '_object_ref')}`,
      object_ref_components: ['object_id', 'version', 'digest'],
      expected_initial_owner_role: 'deployer',
    },
  }
}

function publishCommands() {
  return [
    publishCommand(
      'currency_publish', ['esk'], 'fixed_toolchain_verified_build_output',
      'toolchain_binding.currency_production_bytecode_digest', ['0x1', '0x2'],
      'currency_upgrade_cap_object_id'),
    publishCommand(
      'participation_publish', ['genesis_allocation', 'team_vesting'],
      'step_3_verified_rebound_build_output',
      'participation.rebound_production_bytecode_digest',
      ['0x1', '0x2', 'currency_package_id'],
      'participation_upgrade_cap_object_id'),
  ]
}

function registryFinalizeCall() {
  return {
    step_id: 'currency_registry_finalize',
    move_function: '0x2::coin_registry::finalize_registration',
    type_argument_source: 'currency_package_id::esk::ESK',
    argument_sources_in_abi_order: [
      'sui.coin_registry.mutable_shared_object_ref',
      'currency_publish.final_effects.pending_currency_receiving_object_ref',
    ],
    coin_registry_shared_ref: {
      object_id: '0xc',
      initial_shared_version_source:
        'attempt_journal.currency_registry_finalize.read_only_owner.initial_shared_version',
      chain_output_slot: 'coin_registry_initial_shared_version',
      owner_must_be_shared: true, mutable: true,
    },
    receiving_currency_ref: {
      components: ['object_id', 'version', 'digest'],
      source: 'currency_publish.final_effects.pending_currency_receiving_object_ref',
      expected_owner: 'object_owner_0xc',
      expected_type_source:
        '0x2::coin_registry::Currency<currency_package_id::esk::ESK>',
    },
    result_semantics: 'shares_derived_currency_object_at_registry_derived_address',
    creation_version_output: 'registered_currency_creation_version',
  }
}

function ownedObjectLineage() {
  const ref = ['object_id', 'version', 'digest']
  return [
    {
      consumer_step_id: 'genesis_allocation_and_team_vesting',
      input_name: 'initial_supply_coin',
      object_ref_source: 'currency_publish.final_effects.initial_supply_coin_object_ref',
      object_ref_components: ref,
      expected_owner_role: 'deployer',
      expected_type_source: '0x2::coin::Coin<currency_package_id::esk::ESK>',
      expected_package_id_source: 'currency_package_id',
    },
    {
      consumer_step_id: 'genesis_allocation_and_team_vesting',
      input_name: 'genesis_allocation_cap',
      object_ref_source:
        'participation_publish.final_effects.genesis_allocation_cap_object_ref',
      object_ref_components: ref,
      expected_owner_role: 'deployer',
      expected_type_source:
        'participation_package_id::genesis_allocation::GenesisAllocationCap',
      expected_package_id_source: 'participation_package_id',
    },
    {
      consumer_step_id: 'capability_handoff',
      input_name: 'metadata_cap',
      object_ref_source: 'currency_publish.final_effects.metadata_cap_object_ref',
      object_ref_components: ref,
      expected_owner_role: 'deployer',
      expected_type_source:
        '0x2::coin_registry::MetadataCap<currency_package_id::esk::ESK>',
      expected_package_id_source: 'currency_package_id',
    },
    {
      consumer_step_id: 'capability_handoff',
      input_name: 'currency_upgrade_cap',
      object_ref_source: 'currency_publish.final_effects.currency_upgrade_cap_object_ref',
      object_ref_components: ref,
      expected_owner_role: 'deployer',
      expected_type_source: '0x2::package::UpgradeCap',
      expected_package_id_source: 'currency_package_id',
    },
    {
      consumer_step_id: 'capability_handoff',
      input_name: 'participation_upgrade_cap',
      object_ref_source:
        'participation_publish.final_effects.participation_upgrade_cap_object_ref',
      object_ref_components: ref,
      expected_owner_role: 'deployer',
      expected_type_source: '0x2::package::UpgradeCap',
      expected_package_id_source: 'participation_package_id',
    },
  ].map((item) => ({
    ...item, source_finality_required: true,
    selection_policy: 'bind_named_producer_final_effects_never_query_by_type',
  }))
}

function allocationCall() {
  return {
    step_id: 'genesis_allocation_and_team_vesting',
    move_function_source: 'participation_package_id::genesis_allocation::allocate',
    argument_sources_in_abi_order: [
      'participation_publish.final_effects.genesis_allocation_cap_object_ref',
      'currency_publish.final_effects.initial_supply_coin_object_ref',
      'roles.distribution', 'roles.team_beneficiary', 'roles.treasury',
      'roles.liquidity', 'allocations.user_migration_and_ecosystem.base_units',
      'allocations.team_vesting.base_units', 'allocations.project_treasury.base_units',
      'allocations.liquidity.base_units', 'allocations.community_contributors.base_units',
      'allocations.security_operations_reserve.base_units', 'team_vesting.start_ms',
      'team_vesting.cliff_ms', 'team_vesting.end_ms',
      'plan_sha256.hex_body_decoded_to_vector_u8', 'sui.clock.immutable_shared_object_ref',
    ],
    manifest_digest: {
      source: 'plan_sha256',
      digest_scope: 'canonical_plan_without_plan_sha256_field',
      source_encoding: 'sha256_prefixed_lowercase_hex',
      transformation: 'strip_sha256_prefix_then_hex_decode',
      move_type: 'vector<u8>', decoded_length_bytes: 32,
      post_plan_execution_authorization_must_bind_plan_sha256: true,
    },
    clock_shared_ref: {
      object_id: '0x6', initial_shared_version: '1', mutable: false,
    },
    execution_time_gate: {
      condition: 'sui.clock.timestamp_ms<=team_vesting.start_ms',
      enforced_by: 'team_vesting.create_and_transfer',
      failure_action: 'abort_transaction_and_stop_without_resubmission',
    },
    pre_sign_read_only_gate: {
      required: true,
      future_authorized_source: 'trusted_testnet_clock_read_or_transaction_dry_run',
      condition: 'observed_clock.timestamp_ms<=team_vesting.start_ms',
      check_before: ['deployer_signature', 'gas_sponsor_signature', 'broadcast'],
      append_result_to_attempt_journal: true,
      failure_action: 'do_not_sign_or_broadcast',
    },
  }
}

function transferCommand(objectArgumentSource, recipientAddressSource) {
  return {
    command: 'TransferObjects',
    object_argument_sources: [objectArgumentSource],
    recipient_address_source: recipientAddressSource,
  }
}

function policyMoveCall(moveFunction, objectRefSource, argumentMode) {
  return {
    command: 'MoveCall',
    move_function: moveFunction,
    type_arguments: [],
    arguments_in_abi_order: [{
      object_ref_source: objectRefSource,
      argument_mode: argumentMode,
    }],
  }
}

function capabilityHandoffCommands() {
  const selectedCap = 'selected_package.upgrade_cap_object_ref_source'
  const selectedRecipient = 'selected_package.transfer_recipient_address_source'
  const transferSelectedCap = transferCommand(selectedCap, selectedRecipient)
  return {
    step_id: 'capability_handoff',
    ptb_command_order: [
      'metadata_cap_transfer',
      'currency_upgrade_cap_selected_policy_sequence',
      'participation_upgrade_cap_selected_policy_sequence',
    ],
    metadata_cap_transfer: transferCommand(
      'currency_publish.final_effects.metadata_cap_object_ref', 'roles.metadata'),
    upgrade_cap_package_bindings: [
      {
        package_name: 'currency',
        upgrade_cap_object_ref_source:
          'currency_publish.final_effects.currency_upgrade_cap_object_ref',
        expected_cap_package_id_source: 'currency_package_id',
        expected_cap_content_version_source:
          'currency_publish.final_effects.published_package_version',
        selected_policy_source: 'upgrade_policies.currency',
        selected_policy_action_source:
          'dag[step_id=capability_handoff].upgrade_capability_dispositions.currency.policy_action',
        transfer_recipient_address_source: 'roles.currency_upgrade',
      },
      {
        package_name: 'participation',
        upgrade_cap_object_ref_source:
          'participation_publish.final_effects.participation_upgrade_cap_object_ref',
        expected_cap_package_id_source: 'participation_package_id',
        expected_cap_content_version_source:
          'participation_publish.final_effects.published_package_version',
        selected_policy_source: 'upgrade_policies.participation',
        selected_policy_action_source:
          'dag[step_id=capability_handoff].upgrade_capability_dispositions.participation.policy_action',
        transfer_recipient_address_source: 'roles.participation_upgrade',
      },
    ],
    pre_transaction_cap_content_checks: {
      expected_type: '0x2::package::UpgradeCap',
      expected_package_field_source: 'selected_package.expected_cap_package_id_source',
      expected_version_field_source:
        'selected_package.expected_cap_content_version_source',
      expected_initial_policy_u8: 0,
      check_before: ['transaction_construction', 'signatures', 'broadcast'],
    },
    upgrade_policy_cases: {
      pending: {
        policy_action: 'pending_policy_selection',
        executable: false,
        command_sequence: [],
        expected_final_policy_u8: null,
        expected_final_cap_status: 'not_processed',
      },
      compatible: {
        policy_action: 'verify_upgrade_cap_policy_0_then_transfer',
        executable: true,
        command_sequence: [transferSelectedCap],
        expected_final_policy_u8: 0,
        expected_final_cap_status: 'address_owned_by_selected_upgrade_role',
      },
      additive: {
        policy_action: 'call_0x2_package_only_additive_upgrades_then_transfer',
        executable: true,
        command_sequence: [
          policyMoveCall('0x2::package::only_additive_upgrades',
            selectedCap, 'mutable_reference'),
          transferSelectedCap,
        ],
        expected_final_policy_u8: 128,
        expected_final_cap_status: 'address_owned_by_selected_upgrade_role',
      },
      dep_only: {
        policy_action: 'call_0x2_package_only_dep_upgrades_then_transfer',
        executable: true,
        command_sequence: [
          policyMoveCall('0x2::package::only_dep_upgrades',
            selectedCap, 'mutable_reference'),
          transferSelectedCap,
        ],
        expected_final_policy_u8: 192,
        expected_final_cap_status: 'address_owned_by_selected_upgrade_role',
      },
      immutable: {
        policy_action: 'call_0x2_package_make_immutable_and_destroy_cap',
        executable: true,
        command_sequence: [
          policyMoveCall('0x2::package::make_immutable',
            selectedCap, 'owned_value'),
        ],
        expected_final_policy_u8: null,
        expected_final_cap_status: 'consumed_and_deleted',
      },
    },
    post_effects_evidence_required: [
      'metadata_cap_owner_equals_roles.metadata',
      'each_upgrade_cap_package_field_equals_expected_package_id',
      'each_nonimmutable_upgrade_cap_policy_equals_selected_policy_u8',
      'each_nonimmutable_upgrade_cap_owner_equals_selected_upgrade_role',
      'each_immutable_upgrade_cap_is_deleted',
    ],
  }
}

function executionAuthorizationAttestation() {
  return {
    artifact_location: 'external_post_plan_artifact',
    included_in_plan_sha256: false,
    created_after_plan: true,
    candidate_approval_digests_are_prerequisites_not_execution_authorization: true,
    required_subject_field: 'plan_sha256',
    allowed_step_ids: CHAIN_STEPS.map(([stepId]) => stepId),
    authorization_must_include_current_step: true,
    required_fields: [
      'schema', 'plan_sha256', 'authorized_step_ids', 'signer_addresses', 'threshold',
      'approved_at', 'expires_at', 'attestation_digest', 'signatures',
    ],
    authenticity_and_subject_binding_required_before_transaction_construction: true,
  }
}

function evidenceProducerRequirements() {
  return [
    {
      evidence_id: 'capability_handoff',
      status: 'implementation_required',
      output_schema: 'yilong.esk.sui.capability_handoff_observation.v1',
    },
    {
      evidence_id: 'source_correspondence',
      status: 'implementation_required',
      output_schema: 'yilong.esk.sui.source_correspondence_observation.v1',
    },
    {
      evidence_id: 'committee_finality',
      status: 'implementation_required',
      output_schema: 'yilong.esk.sui.committee_finality_observation.v1',
    },
  ]
}

function executionContract() {
  return {
    transaction_envelopes: transactionEnvelopes(),
    publish_commands: publishCommands(),
    registry_finalize_call: registryFinalizeCall(),
    owned_object_lineage: ownedObjectLineage(),
    allocation_call: allocationCall(),
    capability_handoff_commands: capabilityHandoffCommands(),
    execution_authorization_attestation: executionAuthorizationAttestation(),
    evidence_producer_requirements: evidenceProducerRequirements(),
  }
}

module.exports = { executionContract }
