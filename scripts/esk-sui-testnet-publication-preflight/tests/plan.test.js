'use strict'

const test = require('node:test')
const assert = require('node:assert/strict')
const { readFileSync } = require('node:fs')
const { join } = require('node:path')
const { preflightCandidate } = require('../index')
const { releaseCandidate, syntheticCandidate, templateCandidate } = require('./fixtures')

const STEP_IDS = [
  'currency_publish',
  'currency_registry_finalize',
  'participation_rebind_rebuild_test',
  'participation_publish',
  'genesis_allocation_and_team_vesting',
  'capability_handoff',
  'three_observer_and_three_verifier_evidence_gate',
  'evidence_manifest_v2_handoff',
]
const CHAIN_OUTPUT_KEYS = [
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
  'allocation_checkpoint', 'capability_handoff_tx_digest',
  'capability_handoff_checkpoint',
]
const SAFETY_FALSE_KEYS = [
  'default_sui_config_read', 'wallet_accessed', 'faucet_requested',
  'gas_coins_queried', 'execution_authorized', 'transactions_constructed',
  'transactions_signed', 'transactions_broadcast', 'rpc_queried', 'funds_moved',
  'chain_finality_verified', 'asset_identity_verified', 'balance_eligible',
  'manifest_transition_allowed', 'public_sale_activation_allowed',
  'funds_acceptance_automation_allowed', 'investment_automation_allowed',
  'return_or_top_up_automation_allowed',
]
const PLAN_KEYS = [
  'schema', 'candidate_id', 'mode', 'candidate_status', 'repository_binding',
  'toolchain_binding', 'parameter_summary', 'role_summary', 'execution_contract', 'dag',
  'observer_templates', 'recovery_checklist', 'blocking_reasons',
  'user_actions_required', 'next_safe_action', 'chain_outputs', 'safety',
  'plan_sha256',
]
const STEP_SCHEMA_NAMES = [
  'currencyPublishStep',
  'currencyRegistryStep',
  'participationRebuildStep',
  'participationPublishStep',
  'allocationStep',
  'capabilityHandoffStep',
  'observerStep',
  'evidenceStep',
]
const STEP_CONTRACTS = [
  {
    authorization_roles: ['deployer', 'gas_sponsor'],
    public_inputs: [
      'packages.currency.source_path', 'packages.currency.production_bytecode_digest',
      'roles.deployer', 'roles.gas_sponsor', 'gas_budgets.currency_publish',
    ],
    pending_outputs: [
      'currency_package_id', 'pending_currency_object_id', 'metadata_cap_object_id',
      'pending_currency_object_version', 'pending_currency_object_digest',
      'currency_upgrade_cap_object_id', 'initial_supply_coin_object_id',
      'currency_publish_tx_digest', 'currency_publish_checkpoint',
    ],
  },
  {
    authorization_roles: ['deployer', 'gas_sponsor'],
    public_inputs: [
      'sui.coin_registry.mutable_shared_object_ref', 'currency_package_id',
      'pending_currency_object_id', 'pending_currency_object_version',
      'pending_currency_object_digest',
      'roles.deployer', 'roles.gas_sponsor',
      'gas_budgets.currency_registry_finalize',
    ],
    pending_outputs: [
      'registered_currency_object_id', 'registered_currency_creation_version',
      'currency_registration_tx_digest',
      'currency_registration_checkpoint',
    ],
  },
  {
    authorization_roles: [],
    public_inputs: [
      'currency_package_id', 'packages.participation.source_path',
      'packages.participation.dependency_binding', 'toolchain.fixed_build_test',
    ],
    pending_outputs: [
      'participation.rebound_package_input_digest',
      'participation.rebound_production_bytecode_digest',
      'participation.rebuild_test_receipt_digest',
    ],
  },
  {
    authorization_roles: ['deployer', 'gas_sponsor'],
    public_inputs: [
      'participation.rebound_production_bytecode_digest', 'roles.deployer',
      'roles.gas_sponsor', 'gas_budgets.participation_publish',
    ],
    pending_outputs: [
      'participation_package_id', 'participation_upgrade_cap_object_id',
      'genesis_allocation_cap_object_id', 'participation_publish_tx_digest',
      'participation_publish_checkpoint',
    ],
  },
  {
    authorization_roles: ['deployer', 'gas_sponsor'],
    public_inputs: [
      'initial_supply_coin_object_id', 'genesis_allocation_cap_object_id',
      'sui.clock.immutable_shared_object_ref',
      'plan_sha256.hex_body_decoded_to_vector_u8',
      'roles.deployer', 'roles.gas_sponsor',
      'asset.total_base_units', 'allocations.six_buckets', 'roles.distribution',
      'roles.team_beneficiary', 'roles.treasury', 'roles.liquidity',
      'team_vesting.schedule', 'gas_budgets.genesis_allocation',
    ],
    pending_outputs: [
      'allocation_receipt_object_id', 'team_vesting_object_id',
      'allocation.bucket_coin_object_ids', 'allocation_tx_digest', 'allocation_checkpoint',
    ],
  },
  {
    authorization_roles: ['deployer', 'gas_sponsor'],
    public_inputs: [
      'metadata_cap_object_id', 'currency_upgrade_cap_object_id',
      'participation_upgrade_cap_object_id', 'upgrade_policies.currency',
      'upgrade_policies.participation', 'roles.deployer', 'roles.gas_sponsor',
      'roles.metadata',
      'roles.currency_upgrade', 'roles.participation_upgrade',
      'gas_budgets.capability_handoff',
    ],
    pending_outputs: [
      'capability_handoff_tx_digest', 'capability_handoff_checkpoint',
      'capability.metadata.final_owner_evidence',
      'capability.currency_upgrade.final_policy',
      'capability.currency_upgrade.disposition',
      'capability.currency_upgrade.disposition_evidence',
      'capability.participation_upgrade.final_policy',
      'capability.participation_upgrade.disposition',
      'capability.participation_upgrade.disposition_evidence',
    ],
  },
  {
    authorization_roles: [],
    public_inputs: [
      'observer.publication', 'observer.currency', 'observer.allocation',
      'verifier.capability_handoff', 'verifier.source_correspondence',
      'verifier.committee_finality',
      'observer.dual_source_required',
    ],
    pending_outputs: [
      'observer.publication.report_sha256', 'observer.currency.report_sha256',
      'observer.allocation.report_sha256',
      'verifier.capability_handoff.report_sha256',
      'verifier.source_correspondence.report_sha256',
      'verifier.committee_finality.report_sha256',
    ],
  },
  {
    authorization_roles: [],
    public_inputs: [
      'observer_reports.three_verified',
      'verifier_reports.capability_handoff_verified',
      'verifier_reports.source_correspondence_verified',
      'verifier_reports.committee_finality_verified',
    ],
    pending_outputs: [
      'evidence_v2.sha256', 'manifest_v2.sha256', 'platform_transition.review',
    ],
  },
]

test('DAG has the exact eight-stage dependency order and never marks a step executed', () => {
  const candidate = releaseCandidate()
  const plan = preflightCandidate(candidate)
  assert.deepEqual(Object.keys(plan).sort(), [...PLAN_KEYS].sort())
  assert.deepEqual(plan.dag.map(step => step.step_id), STEP_IDS)
  assert.equal(plan.toolchain_binding.toolchain_contract_sha256,
    'sha256:c7226dcb2e707a5c48d2f469b0b611296e5b65c1cc786dfd832b3b78e22065b6')
  assert.deepEqual(plan.dag.map(step => step.sequence), [1, 2, 3, 4, 5, 6, 7, 8])
  assert.deepEqual(plan.dag.map(step => step.depends_on), [
    [], ['currency_publish'], ['currency_registry_finalize'],
    ['participation_rebind_rebuild_test'], ['participation_publish'],
    ['genesis_allocation_and_team_vesting'], ['capability_handoff'],
    ['three_observer_and_three_verifier_evidence_gate'],
  ])
  assert.deepEqual(plan.dag.map(step => step.kind), [
    'chain_transaction', 'chain_transaction', 'offline_build', 'chain_transaction',
    'chain_transaction', 'chain_transaction', 'offline_verification', 'evidence_handoff',
  ])
  assert.deepEqual(plan.dag.map(step => step.max_gas_budget), [
    candidate.gas_budgets.currency_publish,
    candidate.gas_budgets.currency_registry_finalize,
    null,
    candidate.gas_budgets.participation_publish,
    candidate.gas_budgets.genesis_allocation,
    candidate.gas_budgets.capability_handoff,
    null,
    null,
  ])
  for (const step of plan.dag) {
    assert.equal(step.state, 'planned')
    assert.equal(step.stop_on_failure, true)
    assert.ok(Array.isArray(step.public_inputs) && step.public_inputs.length > 0)
    assert.ok(Array.isArray(step.pending_outputs) && step.pending_outputs.length > 0)
  }
  assert.match(JSON.stringify(plan.dag[2]), /currency.*package/i)
  assert.match(JSON.stringify(plan.dag[2]), /rebuild|test/i)
  const chainSteps = plan.dag.filter(step => step.kind === 'chain_transaction')
  for (const step of chainSteps) {
    assert.deepEqual(step.authorization_roles, ['deployer', 'gas_sponsor'])
    assert.ok(step.public_inputs.includes('roles.gas_sponsor'), step.step_id)
  }
})

test('every DAG step locks the exact authorization, public inputs, and pending outputs', () => {
  const dag = preflightCandidate(releaseCandidate()).dag
  for (const [index, expected] of STEP_CONTRACTS.entries()) {
    assert.deepEqual(dag[index].authorization_roles, expected.authorization_roles,
      `${STEP_IDS[index]} authorization_roles`)
    assert.deepEqual(dag[index].public_inputs, expected.public_inputs,
      `${STEP_IDS[index]} public_inputs`)
    assert.deepEqual(dag[index].pending_outputs, expected.pending_outputs,
      `${STEP_IDS[index]} pending_outputs`)
  }

  const schemaPath = join(__dirname, '..', '..', '..',
    'contracts', 'sui', 'esk-testnet-publication-preflight-v1.schema.json')
  const schema = JSON.parse(readFileSync(schemaPath, 'utf8'))
  assert.deepEqual(schema.properties.execution_contract.const,
    preflightCandidate(releaseCandidate()).execution_contract)
  for (const [index, schemaName] of STEP_SCHEMA_NAMES.entries()) {
    const locked = schema.$defs[schemaName].allOf[1].properties
    assert.deepEqual(locked.authorization_roles.const,
      STEP_CONTRACTS[index].authorization_roles, `${schemaName} authorization schema`)
    assert.deepEqual(locked.public_inputs.const,
      STEP_CONTRACTS[index].public_inputs, `${schemaName} public input schema`)
    assert.deepEqual(locked.pending_outputs.const,
      STEP_CONTRACTS[index].pending_outputs, `${schemaName} pending output schema`)
  }
  assert.deepEqual(dag[4].public_inputs.slice(0, 6), [
    'initial_supply_coin_object_id', 'genesis_allocation_cap_object_id',
    'sui.clock.immutable_shared_object_ref',
    'plan_sha256.hex_body_decoded_to_vector_u8',
    'roles.deployer', 'roles.gas_sponsor',
  ])
})

test('execution contract locks sponsored envelopes, Publish inputs, ABI order, and object lineage', () => {
  const plan = preflightCandidate(releaseCandidate())
  const contract = plan.execution_contract
  assert.deepEqual(contract.transaction_envelopes.map(item => item.step_id), [
    'currency_publish', 'currency_registry_finalize', 'participation_publish',
    'genesis_allocation_and_team_vesting', 'capability_handoff',
  ])
  for (const envelope of contract.transaction_envelopes) {
    assert.equal(envelope.transaction_sender_role, 'deployer')
    assert.equal(envelope.tx_context_sender_role, 'deployer')
    assert.equal(envelope.gas_owner_role, 'gas_sponsor')
    assert.equal(envelope.gas_payment_expected_owner_role, 'gas_sponsor')
    assert.deepEqual(envelope.gas_payment_ref_components,
      ['object_id', 'version', 'digest'])
    assert.match(envelope.gas_payment_object_ref_source,
      new RegExp(`^attempt_journal\\.${envelope.step_id}\\.`))
    assert.match(envelope.gas_price_source,
      new RegExp(`^attempt_journal\\.${envelope.step_id}\\.`))
    assert.equal(envelope.fresh_gas_object_ref_required, true)
    assert.equal(envelope.both_role_signatures_required, true)
  }

  assert.deepEqual(contract.publish_commands, [
    {
      step_id: 'currency_publish',
      move_init_tx_context_sender_role: 'deployer',
      ordered_commands: [
        {
          command_index: 0, command: 'Command::Publish',
          ordered_module_names: ['esk'],
          module_bytes_source: 'fixed_toolchain_verified_build_output',
          module_bundle_digest_source:
            'toolchain_binding.currency_production_bytecode_digest',
          transitive_dependency_ids: ['0x1', '0x2'],
        },
        {
          command_index: 1, command: 'Command::TransferObjects',
          object_arguments: [{ kind: 'Result', command_index: 0 }],
          recipient_address_source: 'roles.deployer',
        },
      ],
      publish_result_contract: {
        producer_command_index: 0, result_count: 1,
        result_0_type: '0x2::package::UpgradeCap',
        result_argument: { kind: 'Result', command_index: 0 },
        consumer_command_index: 1,
        must_be_consumed_in_same_ptb: true,
        nested_result_forbidden: true,
        chain_output_object_id_slot: 'currency_upgrade_cap_object_id',
        final_effects_object_ref_source:
          'currency_publish.final_effects.currency_upgrade_cap_object_ref',
        object_ref_components: ['object_id', 'version', 'digest'],
        expected_initial_owner_role: 'deployer',
      },
    },
    {
      step_id: 'participation_publish',
      move_init_tx_context_sender_role: 'deployer',
      ordered_commands: [
        {
          command_index: 0, command: 'Command::Publish',
          ordered_module_names: ['genesis_allocation', 'team_vesting'],
          module_bytes_source: 'step_3_verified_rebound_build_output',
          module_bundle_digest_source:
            'participation.rebound_production_bytecode_digest',
          transitive_dependency_ids: ['0x1', '0x2', 'currency_package_id'],
        },
        {
          command_index: 1, command: 'Command::TransferObjects',
          object_arguments: [{ kind: 'Result', command_index: 0 }],
          recipient_address_source: 'roles.deployer',
        },
      ],
      publish_result_contract: {
        producer_command_index: 0, result_count: 1,
        result_0_type: '0x2::package::UpgradeCap',
        result_argument: { kind: 'Result', command_index: 0 },
        consumer_command_index: 1,
        must_be_consumed_in_same_ptb: true,
        nested_result_forbidden: true,
        chain_output_object_id_slot: 'participation_upgrade_cap_object_id',
        final_effects_object_ref_source:
          'participation_publish.final_effects.participation_upgrade_cap_object_ref',
        object_ref_components: ['object_id', 'version', 'digest'],
        expected_initial_owner_role: 'deployer',
      },
    },
  ])

  const registry = contract.registry_finalize_call
  assert.equal(registry.move_function,
    '0x2::coin_registry::finalize_registration')
  assert.equal(registry.type_argument_source, 'currency_package_id::esk::ESK')
  assert.deepEqual(registry.argument_sources_in_abi_order, [
    'sui.coin_registry.mutable_shared_object_ref',
    'currency_publish.final_effects.pending_currency_receiving_object_ref',
  ])
  assert.deepEqual(registry.coin_registry_shared_ref, {
    object_id: '0xc',
    initial_shared_version_source:
      'attempt_journal.currency_registry_finalize.read_only_owner.initial_shared_version',
    chain_output_slot: 'coin_registry_initial_shared_version',
    owner_must_be_shared: true, mutable: true,
  })
  assert.deepEqual(registry.receiving_currency_ref.components,
    ['object_id', 'version', 'digest'])
  assert.equal(registry.receiving_currency_ref.expected_owner, 'object_owner_0xc')
  assert.equal(registry.creation_version_output,
    'registered_currency_creation_version')

  assert.deepEqual(contract.owned_object_lineage.map(item => item.input_name), [
    'initial_supply_coin', 'genesis_allocation_cap', 'metadata_cap',
    'currency_upgrade_cap', 'participation_upgrade_cap',
  ])
  for (const input of contract.owned_object_lineage) {
    assert.deepEqual(input.object_ref_components, ['object_id', 'version', 'digest'])
    assert.equal(input.expected_owner_role, 'deployer')
    assert.equal(input.source_finality_required, true)
    assert.equal(input.selection_policy,
      'bind_named_producer_final_effects_never_query_by_type')
  }
  const handoff = contract.capability_handoff_commands
  assert.deepEqual(handoff.ptb_command_order, [
    'metadata_cap_transfer',
    'currency_upgrade_cap_selected_policy_sequence',
    'participation_upgrade_cap_selected_policy_sequence',
  ])
  assert.deepEqual(handoff.metadata_cap_transfer, {
    command: 'TransferObjects',
    object_argument_sources: [
      'currency_publish.final_effects.metadata_cap_object_ref',
    ],
    recipient_address_source: 'roles.metadata',
  })
  assert.deepEqual(handoff.upgrade_cap_package_bindings, [
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
  ])
  assert.deepEqual(handoff.pre_transaction_cap_content_checks, {
    expected_type: '0x2::package::UpgradeCap',
    expected_package_field_source: 'selected_package.expected_cap_package_id_source',
    expected_version_field_source:
      'selected_package.expected_cap_content_version_source',
    expected_initial_policy_u8: 0,
    check_before: ['transaction_construction', 'signatures', 'broadcast'],
  })
  assert.equal(handoff.upgrade_policy_cases.compatible.command_sequence[0].command,
    'TransferObjects')
  assert.equal(handoff.upgrade_policy_cases.additive.command_sequence[0].move_function,
    '0x2::package::only_additive_upgrades')
  assert.deepEqual(
    handoff.upgrade_policy_cases.additive.command_sequence[0].arguments_in_abi_order,
    [{
      object_ref_source: 'selected_package.upgrade_cap_object_ref_source',
      argument_mode: 'mutable_reference',
    }])
  assert.equal(handoff.upgrade_policy_cases.additive.expected_final_policy_u8, 128)
  assert.equal(handoff.upgrade_policy_cases.dep_only.command_sequence[0].move_function,
    '0x2::package::only_dep_upgrades')
  assert.equal(handoff.upgrade_policy_cases.dep_only.expected_final_policy_u8, 192)
  assert.equal(handoff.upgrade_policy_cases.immutable.command_sequence[0].move_function,
    '0x2::package::make_immutable')
  assert.deepEqual(
    handoff.upgrade_policy_cases.immutable.command_sequence[0].arguments_in_abi_order,
    [{
      object_ref_source: 'selected_package.upgrade_cap_object_ref_source',
      argument_mode: 'owned_value',
    }])
  assert.equal(handoff.upgrade_policy_cases.immutable.expected_final_cap_status,
    'consumed_and_deleted')
  assert.deepEqual(handoff.upgrade_policy_cases.pending.command_sequence, [])
  assert.equal(handoff.upgrade_policy_cases.pending.executable, false)
  assert.deepEqual(contract.execution_authorization_attestation, {
    artifact_location: 'external_post_plan_artifact',
    included_in_plan_sha256: false,
    created_after_plan: true,
    candidate_approval_digests_are_prerequisites_not_execution_authorization: true,
    required_subject_field: 'plan_sha256',
    allowed_step_ids: [
      'currency_publish', 'currency_registry_finalize', 'participation_publish',
      'genesis_allocation_and_team_vesting', 'capability_handoff',
    ],
    authorization_must_include_current_step: true,
    required_fields: [
      'schema', 'plan_sha256', 'authorized_step_ids', 'signer_addresses', 'threshold',
      'approved_at', 'expires_at', 'attestation_digest', 'signatures',
    ],
    authenticity_and_subject_binding_required_before_transaction_construction: true,
  })
  assert.deepEqual(contract.evidence_producer_requirements.map(item => item.evidence_id),
    ['capability_handoff', 'source_correspondence', 'committee_finality'])
  for (const producer of contract.evidence_producer_requirements) {
    assert.equal(producer.status, 'implementation_required')
  }
})

test('allocation ABI decodes the plan digest to 32 bytes and binds immutable Clock', () => {
  const allocation = preflightCandidate(releaseCandidate())
    .execution_contract.allocation_call
  assert.equal(allocation.move_function_source,
    'participation_package_id::genesis_allocation::allocate')
  assert.equal(allocation.argument_sources_in_abi_order.length, 17)
  assert.equal(allocation.argument_sources_in_abi_order[15],
    'plan_sha256.hex_body_decoded_to_vector_u8')
  assert.equal(allocation.argument_sources_in_abi_order[16],
    'sui.clock.immutable_shared_object_ref')
  assert.deepEqual(allocation.manifest_digest, {
    source: 'plan_sha256',
    digest_scope: 'canonical_plan_without_plan_sha256_field',
    source_encoding: 'sha256_prefixed_lowercase_hex',
    transformation: 'strip_sha256_prefix_then_hex_decode',
    move_type: 'vector<u8>', decoded_length_bytes: 32,
    post_plan_execution_authorization_must_bind_plan_sha256: true,
  })
  assert.deepEqual(allocation.clock_shared_ref,
    { object_id: '0x6', initial_shared_version: '1', mutable: false })
  assert.deepEqual(allocation.execution_time_gate, {
    condition: 'sui.clock.timestamp_ms<=team_vesting.start_ms',
    enforced_by: 'team_vesting.create_and_transfer',
    failure_action: 'abort_transaction_and_stop_without_resubmission',
  })
  assert.deepEqual(allocation.pre_sign_read_only_gate, {
    required: true,
    future_authorized_source: 'trusted_testnet_clock_read_or_transaction_dry_run',
    condition: 'observed_clock.timestamp_ms<=team_vesting.start_ms',
    check_before: ['deployer_signature', 'gas_sponsor_signature', 'broadcast'],
    append_result_to_attempt_journal: true,
    failure_action: 'do_not_sign_or_broadcast',
  })
})

test('upgrade policies produce stable restrict-transfer or immutable-destroy dispositions', () => {
  const cases = [
    ['compatible', 'verify_upgrade_cap_policy_0_then_transfer'],
    ['additive', 'call_0x2_package_only_additive_upgrades_then_transfer'],
    ['dep_only', 'call_0x2_package_only_dep_upgrades_then_transfer'],
  ]
  for (const [policy, policyAction] of cases) {
    const candidate = releaseCandidate()
    candidate.upgrade_policies = { currency: policy, participation: policy }
    const dispositions = preflightCandidate(candidate).dag[5]
      .upgrade_capability_dispositions
    for (const [packageName, disposition] of Object.entries(dispositions)) {
      assert.equal(disposition.source_capability_input,
        `${packageName}_upgrade_cap_object_id`)
      assert.equal(disposition.requested_policy, policy)
      assert.equal(disposition.expected_final_policy, policy)
      assert.equal(disposition.policy_action, policyAction)
      assert.equal(disposition.disposition, 'transfer_to_upgrade_role')
      assert.equal(disposition.recipient_role, `${packageName}_upgrade`)
      assert.equal(disposition.final_owner_evidence_expected, true)
    }
  }

  const immutableCandidate = releaseCandidate()
  immutableCandidate.upgrade_policies = { currency: 'immutable', participation: 'immutable' }
  const immutable = preflightCandidate(immutableCandidate).dag[5]
    .upgrade_capability_dispositions
  for (const disposition of Object.values(immutable)) {
    assert.equal(disposition.requested_policy, 'immutable')
    assert.equal(disposition.expected_final_policy, 'immutable')
    assert.equal(disposition.policy_action,
      'call_0x2_package_make_immutable_and_destroy_cap')
    assert.equal(disposition.disposition, 'destroy_without_final_owner')
    assert.equal(disposition.recipient_role, null)
    assert.equal(disposition.final_owner_evidence_expected, false)
  }

  const pending = preflightCandidate(templateCandidate()).dag[5]
    .upgrade_capability_dispositions
  for (const disposition of Object.values(pending)) {
    assert.equal(disposition.requested_policy, 'pending')
    assert.equal(disposition.expected_final_policy, null)
    assert.equal(disposition.policy_action, 'pending_policy_selection')
    assert.equal(disposition.disposition, 'pending')
    assert.equal(disposition.recipient_role, null)
    assert.equal(disposition.final_owner_evidence_expected, false)
  }
})

test('all chain outputs stay exact and null in every candidate mode', () => {
  for (const candidate of [templateCandidate(), syntheticCandidate(), releaseCandidate()]) {
    const plan = preflightCandidate(candidate)
    assert.deepEqual(Object.keys(plan.chain_outputs).sort(), [...CHAIN_OUTPUT_KEYS].sort())
    for (const [key, value] of Object.entries(plan.chain_outputs)) {
      assert.equal(value, null, key)
    }
  }
})

test('all truth and money safety flags remain inert for every candidate mode', () => {
  for (const candidate of [templateCandidate(), syntheticCandidate(), releaseCandidate()]) {
    const safety = preflightCandidate(candidate).safety
    assert.deepEqual(Object.keys(safety).sort(),
      [...SAFETY_FALSE_KEYS, 'publication_status'].sort())
    for (const key of SAFETY_FALSE_KEYS) assert.equal(safety[key], false, key)
    assert.equal(safety.publication_status, 'not_performed')
  }
})

test('unresolved two-year ESK commercial policy is visible and blocks public sale only', () => {
  for (const candidate of [templateCandidate(), syntheticCandidate(), releaseCandidate()]) {
    const plan = preflightCandidate(candidate)
    const revision = plan.parameter_summary.commercial_policy_revision
    assert.equal(revision.status, 'clarification_required')
    assert.equal(revision.known_intent, 'team_considers_two_year_loss_backstop')
    assert.equal(revision.intended_term_months, 24)
    assert.equal(revision.qshare_application, 'not_automatic')
    assert.equal(revision.legacy_no_protection_terms_promotable, false)
    assert.equal(revision.sample_terms_promotable, false)
    assert.equal(revision.public_sale_activation_allowed, false)
    assert.equal(revision.technical_testnet_preflight_allowed, true)
    assert.equal(
      revision.funds_acceptance_investment_or_return_automation_allowed, false)
    assert.equal(plan.parameter_summary.commercial_policy_revision_resolved, false)
    assert.equal(plan.safety.public_sale_activation_allowed, false)
    assert.equal(plan.safety.funds_acceptance_automation_allowed, false)
    assert.equal(plan.safety.investment_automation_allowed, false)
    assert.equal(plan.safety.return_or_top_up_automation_allowed, false)
    assert.equal(plan.user_actions_required.some(action =>
      action.includes('APPROVE_ESK_TWO_YEAR_BACKSTOP_POLICY')), false)
  }
  const template = preflightCandidate(templateCandidate())
  assert.ok(template.user_actions_required.includes(
    'CLARIFY_AND_DECIDE_ESK_COMMERCIAL_POLICY'))
  const release = preflightCandidate(releaseCandidate())
  assert.ok(release.blocking_reasons.includes(
    'COMMERCIAL_POLICY_REVISION_REQUIRED_BEFORE_PUBLIC_SALE'))
  assert.ok(release.user_actions_required.includes(
    'CLARIFY_AND_DECIDE_ESK_COMMERCIAL_POLICY_BEFORE_PUBLIC_SALE'))
})

test('prepared plan snapshots candidate objects before computing its digest', () => {
  const candidate = releaseCandidate()
  const plan = preflightCandidate(candidate)
  const digest = plan.plan_sha256
  const treasury = plan.role_summary.sha256

  candidate.commercial_policy_revision.status = 'approved'
  candidate.commercial_policy_revision.public_sale_activation_allowed = true
  candidate.roles.treasury = candidate.roles.deployer

  assert.equal(plan.parameter_summary.commercial_policy_revision.status,
    'clarification_required')
  assert.equal(plan.parameter_summary.commercial_policy_revision.public_sale_activation_allowed,
    false)
  assert.equal(plan.role_summary.sha256, treasury)
  assert.equal(plan.plan_sha256, digest)
})

test('observer templates are three inert dual-source placeholders in exact order', () => {
  const observers = preflightCandidate(releaseCandidate()).observer_templates
  assert.deepEqual(observers.map(item => item.observer_id),
    ['publication', 'currency', 'allocation'])
  for (const observer of observers) {
    assert.equal(observer.status, 'not_run')
    assert.equal(observer.dual_source_required, true)
    assert.ok(observer.required_claims.length >= 2)
    assert.equal(observer.official_endpoint, null)
    assert.equal(observer.secondary_endpoint, null)
    assert.equal(observer.expected_input_sha256, null)
    assert.equal(observer.report_sha256, null)
    assert.equal(observer.observed_at, null)
    assert.equal(observer.error_code, null)
    assert.equal(observer.manifest_transition_allowed, false)
  }
  assert.ok(observers[2].required_claims.includes('allocator_equals_roles.deployer'))
  assert.deepEqual(observers[0].required_claims, [
    'package_transaction_checkpoint_dual_source_consistency',
    'trust_basis_rpc_reports_without_committee_signature_verification',
  ])
})

test('recovery policy requires an append-only journal and forbids blind resubmission', () => {
  const recovery = preflightCandidate(releaseCandidate()).recovery_checklist
  assert.equal(recovery.attempt_journal_required, true)
  assert.equal(recovery.attempt_journal_append_only, true)
  assert.equal(recovery.attempt_journal_created, false)
  assert.equal(recovery.blind_resubmit_allowed, false)
  assert.equal(recovery.chain_success_database_rollback_allowed, false)
  assert.equal(recovery.unknown_result_resolution,
    'query_by_known_transaction_digest_or_stable_request_key_before_retry')
  assert.deepEqual(recovery.gates.map(item => item.gate_id), [
    'currency_registration_incomplete',
    'participation_rebuild_or_publish_incomplete',
    'allocation_incomplete',
    'capability_handoff_incomplete',
    'observer_disagreement',
    'finality_or_evidence_incomplete',
  ])
  assert.equal(recovery.gates[2].recovery_action,
    'query_allocation_transaction_and_objects_then_stop_without_resubmission')
  for (const gate of recovery.gates) {
    assert.equal(gate.status, 'not_run')
    assert.equal(gate.evidence_sha256, null)
    assert.equal(gate.blocks_progress, true)
  }
})

test('approval digests are release-only while authenticity stays unverified', () => {
  const template = preflightCandidate(templateCandidate()).role_summary
  const synthetic = preflightCandidate(syntheticCandidate()).role_summary
  const release = preflightCandidate(releaseCandidate()).role_summary
  for (const summary of [template, synthetic]) {
    assert.equal(summary.role_controls_approval_digest_present, false)
    assert.equal(summary.multisig_recovery_approval_digest_present, false)
  }
  assert.equal(release.role_controls_approval_digest_present, true)
  assert.equal(release.multisig_recovery_approval_digest_present, true)
  for (const summary of [template, synthetic, release]) {
    assert.equal(summary.approval_authenticity_verified, false)
    assert.equal(summary.approval_subject_binding_verified, false)
    assert.equal(summary.pause_capability_status,
      'reserved_without_current_capability')
  }
  assert.doesNotMatch(JSON.stringify(release), /pause_cap_object_id|roles\.pause/)
})

test('schema locks every mode blocking reason and required user action', () => {
  const schemaPath = join(__dirname, '..', '..', '..',
    'contracts', 'sui', 'esk-testnet-publication-preflight-v1.schema.json')
  const schema = JSON.parse(readFileSync(schemaPath, 'utf8'))
  const plans = [
    preflightCandidate(templateCandidate()),
    preflightCandidate(syntheticCandidate()),
    preflightCandidate(releaseCandidate()),
  ]
  assert.equal(schema.allOf.length, plans.length)
  for (const [index, plan] of plans.entries()) {
    const locked = schema.allOf[index].then.properties
    assert.deepEqual(locked.blocking_reasons.const, plan.blocking_reasons)
    assert.deepEqual(locked.user_actions_required.const, plan.user_actions_required)
  }
})

test('plan contains no generated time or embedded chain result', () => {
  const plan = preflightCandidate(releaseCandidate())
  const text = JSON.stringify(plan)
  assert.doesNotMatch(text,
    /"generated_at"\s*:|"observed_(?:checkpoint|package_id|transaction_digest)"\s*:/i)
  assert.ok(Object.values(plan.chain_outputs).every(value => value === null))
  assert.equal(plan.safety.publication_status, 'not_performed')
})
