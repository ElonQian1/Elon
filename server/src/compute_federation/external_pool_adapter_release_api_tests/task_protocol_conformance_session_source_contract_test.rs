const SESSION_ROOTS: &str =
    include_str!("../../../external-pool-adapter-session-core/src/roots/task_conformance.rs");
const SESSION_MOD: &str =
    include_str!("../../../external-pool-adapter-session-core/src/task_protocol/mod.rs");
const SESSION_WIRE: &str =
    include_str!("../../../external-pool-adapter-session-core/src/task_protocol/wire.rs");
const SESSION_HOST: &str =
    include_str!("../../../external-pool-adapter-session-core/src/task_protocol/host.rs");
const SESSION_CHILD: &str =
    include_str!("../../../external-pool-adapter-session-core/src/task_protocol/child.rs");
const SESSION_RECEIPT: &str =
    include_str!("../../../external-pool-adapter-session-core/src/task_protocol/receipt.rs");
const LAUNCH_ARGUMENTS: &str = include_str!(
    "../external_pool_adapter_linux_supervisor/launch/task_protocol_conformance_arguments.rs"
);
const SESSION_FACADE: &str = include_str!("../external_pool_adapter_supervisor_session.rs");
const FIXTURE: &str =
    include_str!("../../external_pool_adapter_session_fixture/task_protocol_conformance.rs");
const FIXTURE_ORACLE: &str =
    include_str!("../../external_pool_adapter_session_fixture/task_protocol_conformance/oracle.rs");
const RUN_EXECUTION: &str = include_str!(
    "../../store/compute_external_pool_adapter_task_protocol_conformance/run/execution.rs"
);
const RUN_ORACLE: &str = include_str!(
    "../../store/compute_external_pool_adapter_task_protocol_conformance/run/oracle.rs"
);
const RUN_UNCERTAINTY: &str = include_str!(
    "../../store/compute_external_pool_adapter_task_protocol_conformance/run/oracle/uncertainty.rs"
);
const RUN_SUPPORT: &str = include_str!(
    "../../store/compute_external_pool_adapter_task_protocol_conformance/run/support.rs"
);
const DOMAIN_CATALOG: &str =
    include_str!("../external_pool_adapter_task_protocol_conformance/catalog.rs");

const ROOTS: &[&str] = &[
    "supervisor_session_policy_digest",
    "task_protocol_profile_digest",
    "run_nonce_digest",
    "fixture_catalog_digest",
    "registry_release_digest",
    "installation_content_digest",
    "capability_set_digest",
    "sandbox_reattestation_receipt_digest",
    "runtime_compatibility_verification_receipt_digest",
    "source_capsule_sha256",
    "launch_image_sha256",
    "public_fixture_delivery_root",
    "synthetic_fixture_lane_digest",
    "synthetic_fixture_executor_digest",
];

#[test]
fn task_protocol_conformance_session_freezes_roots_wire_and_linear_receipts() {
    assert_ordered(SESSION_ROOTS, ROOTS);
    assert!(SESSION_ROOTS
        .contains("elon.external_pool_adapter.task_protocol_conformance.session.roots.v1\\0"));
    assert!(SESSION_ROOTS
        .contains("elon.external_pool_adapter.task_protocol_conformance.session.kdf_salt.v1\\0"));
    assert_eq!(
        LAUNCH_ARGUMENTS
            .matches("--elon-task-protocol-conformance-")
            .count(),
        14
    );
    assert_eq!(
        FIXTURE.matches("--elon-task-protocol-conformance-").count(),
        14
    );
    assert!(FIXTURE.contains("let [supervisor_session_policy_digest,"));
    assert!(RUN_EXECUTION.contains("let [supervisor_session_policy_digest,"));
    for marker in ["[String; 14]", "ROOT_ARGUMENT_PREFIXES: [&str; 14]"] {
        let combined = format!("{RUN_EXECUTION}{LAUNCH_ARGUMENTS}{FIXTURE}");
        assert!(combined.contains(marker), "fixed roots lost {marker}");
    }

    for marker in [
        "const MAGIC: &[u8; 4] = b\"ELTP\"",
        "const VERSION: u8 = 1",
        "const FLAGS: u16 = 0",
        "const BEGIN: u8 = 1",
        "const REQUEST: u8 = 2",
        "const RESPONSE: u8 = 3",
        "const RECEIPT: u8 = 4",
        "MAX_SEMANTIC_BODY_BYTES: usize = 262_144",
        "MAX_UPSTREAM_REQUEST_BYTES: usize = 65_536",
        "MAX_UPSTREAM_RESPONSE_BYTES: usize = 262_144",
        "MAX_OBSERVATION_BYTES: usize = 262_144",
        "MAX_EXCHANGE_ORDINAL: u64 = 64",
        "elon.external_pool_adapter.task_protocol.request.v1\\0",
        "elon.external_pool_adapter.task_protocol.exchange.v1\\0",
    ] {
        assert!(SESSION_WIRE.contains(marker), "ELTP wire lost {marker}");
    }
    assert_ordered(
        SESSION_WIRE,
        &[
            "payload.extend_from_slice(&FLAGS.to_be_bytes())",
            "payload.extend_from_slice(&ordinal.to_be_bytes())",
            "payload.extend_from_slice(&[0_u8; 7])",
            "u16::from_be_bytes(payload[6..8].try_into()?) != FLAGS",
            "payload[17..24].iter().any(|byte| *byte != 0)",
            "u64::from_be_bytes(payload[8..16].try_into()?)",
        ],
    );
    assert_ordered(
        SESSION_MOD,
        &[
            "Prepare = 1",
            "IdempotentCommit = 2",
            "CancelNoStart = 3",
            "Reconcile = 4",
            "AuthenticatedEvents = 5",
        ],
    );
    for source in [SESSION_HOST, SESSION_CHILD] {
        assert!(source.contains("MAX_EXCHANGE_TIMEOUT"));
        assert!(source.contains("impl Drop for ExternalPoolAdapterTaskProtocol"));
    }
    assert!(!SESSION_RECEIPT.contains("#[derive(Clone"));
    assert!(!SESSION_RECEIPT.contains("#[derive(Debug"));
    assert!(!SESSION_RECEIPT.contains("Serialize"));
}

#[test]
fn task_protocol_conformance_session_freezes_eight_stateful_exchanges() {
    assert!(FIXTURE.contains("for expected_ordinal in 1..=8"));
    assert!(RUN_EXECUTION.contains("for ordinal in 1..=8"));
    for source in [RUN_SUPPORT, FIXTURE] {
        assert_ordered(
            source_block(source, "match ordinal {", "_ => bail!"),
            &[
                "1 =>", "2 =>", "3 =>", "4 =>", "5 =>", "6 =>", "7 =>", "8 =>",
            ],
        );
    }
    for required in [
        "same_idempotency_exact_replay",
        "terminal_no_start",
        "no_commit_tombstone",
        "authenticated_events",
        "started",
        "terminal",
    ] {
        let combined = format!("{RUN_ORACLE}{RUN_UNCERTAINTY}{RUN_SUPPORT}{FIXTURE_ORACLE}");
        assert!(
            combined.contains(required),
            "stateful oracle lost {required}"
        );
    }
    assert!(RUN_ORACLE.contains("receipt.ordinal() != material.ordinal"));
    assert!(RUN_ORACLE.contains("receipt.operation() != material.operation"));
}

#[test]
fn task_protocol_conformance_session_freezes_exact_ordinal_matrix() {
    let catalog = compact(DOMAIN_CATALOG);
    assert_ordered(
        source_block(
            DOMAIN_CATALOG,
            "TASK_PROTOCOL_CONFORMANCE_CAPABILITY_COUNT] = [",
            "];",
        ),
        &[
            "\"authenticated_ack\"",
            "\"authenticated_events\"",
            "\"cancel_no_start\"",
            "\"idempotent_commit\"",
            "\"prepare\"",
            "\"reconcile\"",
        ],
    );
    assert!(catalog.contains(
        "capability_exchange_ordinals:vec![(1..=8).collect(),vec![5],vec![7,8],vec![2,3],vec![1,6],vec![4,8],]"
    ));
    for mapping in [
        "exchange(1,\"synthetic_command_a\",\"prepare\",\"prepare\",\"fresh\",&[\"absent\"],&[\"prepared\"],\"nonterminal\",true,Some(1),false,&[],0,0,)",
        "exchange(2,\"synthetic_command_a\",\"idempotent_commit\",\"idempotent_commit\",\"fresh\",&[\"prepared\"],&[\"committed\"],\"nonterminal\",true,Some(2),false,&[],1,0,)",
        "exchange(3,\"synthetic_command_a\",\"idempotent_commit\",\"idempotent_commit\",\"same_idempotency_exact_replay\",&[\"committed\"],&[\"committed\"],\"nonterminal\",true,Some(2),false,&[],1,0,)",
        "exchange(4,\"synthetic_command_a\",\"reconcile\",\"reconcile\",\"fresh\",&[\"committed\"],&[\"running\"],\"nonterminal\",true,Some(2),false,&[],1,0,)",
        "exchange(5,\"synthetic_command_a\",\"authenticated_events\",\"authenticated_events\",\"fresh\",&[\"running\"],&[\"terminal_after_run\"],\"final\",true,Some(2),false,&[\"started\",\"terminal\"],1,2,)",
        "exchange(6,\"synthetic_command_b\",\"prepare\",\"prepare\",\"fresh\",&[\"absent\"],&[\"prepared\"],\"nonterminal\",true,Some(1),false,&[],0,0,)",
        "exchange(7,\"synthetic_command_b\",\"cancel_no_start\",\"cancel_no_start\",\"fresh\",&[\"prepared\"],&[\"prepared\"],\"nonterminal\",true,Some(1),false,&[],0,0,)",
        "exchange(8,\"synthetic_command_b\",\"reconcile\",\"reconcile\",\"fresh\",&[\"prepared\"],&[\"terminal_no_start\"],\"final\",true,Some(2),true,&[],0,0,)",
    ] {
        assert!(catalog.contains(mapping), "catalog ordinal mapping lost {mapping}");
    }
    for mapping in [
        "1|2=>(\"clear\",\"clear\",false)",
        "3=>(\"clear\",\"unknown_after_remote_acceptance\",true)",
        "4=>(\"unknown_after_remote_acceptance\",\"resolved_by_reconcile\",true,)",
        "5=>(\"resolved_by_reconcile\",\"resolved_by_reconcile\",false)",
        "6..=8=>(\"not_applicable\",\"not_applicable\",false)",
        "ifexchange_ordinal==5{(Some(\"exact_duplicate_batch_replay\".into()),1,true)}else{(None,0,false)}",
    ] {
        assert!(
            catalog.contains(mapping),
            "catalog recovery mapping lost {mapping}"
        );
    }

    let material = compact(RUN_SUPPORT);
    for mapping in [
        "1=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::Prepare,\"prepare\",\"fresh\",\"a_prepare\",)",
        "2=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::IdempotentCommit,\"idempotent_commit\",\"fresh\",\"a_commit\",)",
        "3=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::IdempotentCommit,\"idempotent_commit\",\"same_idempotency_exact_replay\",\"a_commit\",)",
        "4=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::Reconcile,\"reconcile\",\"fresh\",\"a_reconcile\",)",
        "5=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::AuthenticatedEvents,\"authenticated_events\",\"fresh\",\"a_events\",)",
        "6=>(\"synthetic_command_b\",ExternalPoolAdapterTaskOperationKind::Prepare,\"prepare\",\"fresh\",\"b_prepare\",)",
        "7=>(\"synthetic_command_b\",ExternalPoolAdapterTaskOperationKind::CancelNoStart,\"cancel_no_start\",\"fresh\",\"b_cancel\",)",
        "8=>(\"synthetic_command_b\",ExternalPoolAdapterTaskOperationKind::Reconcile,\"reconcile\",\"fresh\",\"b_reconcile\",)",
    ] {
        assert!(material.contains(mapping), "runner ordinal mapping lost {mapping}");
    }

    let fixture = compact(FIXTURE);
    for mapping in [
        "1=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::Prepare,)",
        "2=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::IdempotentCommit,)",
        "3=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::IdempotentCommit,)",
        "4=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::Reconcile,)",
        "5=>(\"synthetic_command_a\",ExternalPoolAdapterTaskOperationKind::AuthenticatedEvents,)",
        "6=>(\"synthetic_command_b\",ExternalPoolAdapterTaskOperationKind::Prepare,)",
        "7=>(\"synthetic_command_b\",ExternalPoolAdapterTaskOperationKind::CancelNoStart,)",
        "8=>(\"synthetic_command_b\",ExternalPoolAdapterTaskOperationKind::Reconcile,)",
    ] {
        assert!(
            fixture.contains(mapping),
            "fixture request ordinal mapping lost {mapping}"
        );
    }

    let fixture_oracle = compact(FIXTURE_ORACLE);
    for mapping in [
        "1=>(\"absent\",\"prepared\",\"nonterminal\",1,None,0,(0,0),(0,0),)",
        "2=>(\"prepared\",\"committed\",\"nonterminal\",2,None,0,(0,1),(0,0),)",
        "3=>(\"committed\",\"committed\",\"nonterminal\",2,None,0,(1,1),(0,0),)",
        "4=>(\"committed\",\"running\",\"nonterminal\",2,None,0,(1,1),(0,0),)",
        "5=>(\"running\",\"terminal_after_run\",\"final\",2,None,2,(1,1),(0,2),)",
        "6=>(\"absent\",\"prepared\",\"nonterminal\",1,None,0,(0,0),(0,0),)",
        "7=>(\"prepared\",\"prepared\",\"nonterminal\",1,None,0,(0,0),(0,0),)",
        "8=>(\"prepared\",\"terminal_no_start\",\"final\",2,Some(derived_digest(TOMBSTONE_DOMAIN,&[scenario_id.as_bytes()])),0,(0,0),(0,0),)",
    ] {
        assert!(
            fixture_oracle.contains(mapping),
            "fixture oracle ordinal mapping lost {mapping}"
        );
    }
    for marker in [
        "value.replayed_events.as_slice()!=value.events.as_slice()",
        "value.event_replay_batch_count!=1",
        "value.event_replay_classification.as_deref()!=Some(\"exact_duplicate_batch_replay\")",
        "value.event_replay_root.as_deref()!=Some(inventory.as_str())",
        "first.event_sequence!=1",
        "first.previous_event_root!=cursor_before",
        "second.event_sequence!=2",
        "second.previous_event_root!=first_root",
        "second.event_root!=second_root",
        "CommitUncertaintyState::Pending(marker),4",
        "unknown_after_remote_acceptance",
        "resolved_by_reconcile",
    ] {
        assert!(
            fixture_oracle.contains(marker),
            "fixture state gate lost {marker}"
        );
    }
    let runner_oracle = compact(RUN_ORACLE);
    for marker in [
        "state.event_count=2",
        "Some(\"exact_duplicate_batch_replay\".to_owned()),1,Some(replay_root),events.clone()",
        "event_count:u64::try_from(events.len())?",
    ] {
        assert!(
            runner_oracle.contains(marker),
            "runner event replay production lost {marker}"
        );
    }
    let upstream_response = source_block(
        RUN_ORACLE,
        "struct FixtureUpstreamResponse {",
        "struct FixtureObservation {",
    );
    let semantic_observation = source_block(
        RUN_ORACLE,
        "struct FixtureObservation {",
        "struct FixtureEvent {",
    );
    assert!(!upstream_response.contains("commit_uncertainty"));
    assert!(semantic_observation.contains("commit_uncertainty_state_before"));
    assert!(semantic_observation.contains("commit_uncertainty_marker_digest"));
    assert!(SESSION_CHILD.contains("pub fn request_digest_hex(&self) -> String"));
    assert_ordered(
        RUN_ORACLE,
        &[
            "let (receipt, observed) = exchange.complete(",
            "receipt.ordinal() != material.ordinal",
            "self.apply_post_receipt_uncertainty(",
            "Ok(TaskProtocolConformanceExchangeObservation {",
        ],
    );
    for marker in [
        "(Self::Clear, 3)",
        "PostReceiptCommitUncertainty::MarkUnknown(marker)",
        "(Self::Pending(marker), 4)",
        "PostReceiptCommitUncertainty::MarkResolved(marker)",
        "apply_after_receipt",
        "Self::Pending(pending) if pending == &marker",
    ] {
        assert!(
            RUN_UNCERTAINTY.contains(marker),
            "runner uncertainty gate lost {marker}"
        );
    }
}

#[test]
fn task_protocol_conformance_session_freezes_fresh_delivery_and_cleanup() {
    assert!(SESSION_FACADE
        .contains("pub(crate) fn external_pool_adapter_task_protocol_conformance_session_roots("));
    assert!(SESSION_FACADE.contains("server_supervisor_session_policy_catalog()?"));
    assert_ordered(
        RUN_EXECUTION,
        &[
            "prepare_external_pool_adapter_ephemeral_bundle_delivery(",
            "let public_fixture_delivery_root = delivery.bundle_root_hex()",
            "external_pool_adapter_task_protocol_conformance_session_roots(",
            "let delivery_receipt = delivery.deliver(",
            "for ordinal in 1..=8",
            "delivery_receipt.shutdown(&mut session)?",
            ".wait(CHILD_EXIT_TIMEOUT)?",
            "collect_stderr()?",
            "TaskProtocolConformanceCleanupEvidence {",
            "TaskProtocolConformanceRunEvidence {",
        ],
    );
    assert!(!RUN_EXECUTION.contains(
        "public_fixture_delivery_root == runtime_compatibility_public_fixture_delivery_root"
    ));
    assert!(!RUN_EXECUTION.contains(
        "public_fixture_delivery_root != runtime_compatibility_public_fixture_delivery_root"
    ));
    assert_eq!(
        RUN_EXECUTION
            .matches("TaskProtocolConformanceRunEvidence {")
            .count(),
        1
    );
    let production = format!(
        "{SESSION_ROOTS}{SESSION_MOD}{SESSION_WIRE}{SESSION_HOST}{SESSION_CHILD}{SESSION_RECEIPT}{FIXTURE}{FIXTURE_ORACLE}{RUN_EXECUTION}{RUN_ORACLE}{RUN_UNCERTAINTY}{RUN_SUPPORT}"
    );
    for forbidden in [
        ".unwrap(",
        ".expect(",
        "panic!",
        "todo!",
        "unimplemented!",
        "unreachable!",
    ] {
        assert!(
            !production.contains(forbidden),
            "production retained {forbidden}"
        );
    }
}

fn source_block<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
    source
        .split_once(start)
        .unwrap()
        .1
        .split_once(end)
        .unwrap()
        .0
}

fn assert_ordered(source: &str, needles: &[&str]) {
    let mut cursor = 0;
    for needle in needles {
        let offset = source[cursor..]
            .find(needle)
            .unwrap_or_else(|| panic!("missing ordered source marker {needle}"));
        cursor += offset + needle.len();
    }
}

fn compact(source: &str) -> String {
    source
        .chars()
        .filter(|value| !value.is_whitespace())
        .collect()
}
