use super::*;
use crate::store::compute_external_pool_adapter_task_protocol_conformance::run::support::exchange_material;

const RUN_NONCE: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const FIXTURE_LANE: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const FIXTURE_EXECUTOR: &str = "3333333333333333333333333333333333333333333333333333333333333333";
const REQUEST_DIGEST: &str = "4444444444444444444444444444444444444444444444444444444444444444";

#[test]
fn v272_linux_oracle_executes_exact_eight_exchange_state_machine() {
    let mut oracle = StatefulTaskProtocolOracle::new().expect("V272 oracle should initialize");
    let mut responses = Vec::new();
    let mut observations = Vec::new();

    for ordinal in 1..=8 {
        let (response, observation, post_receipt) = oracle
            .transition(&material(ordinal), REQUEST_DIGEST)
            .unwrap_or_else(|error| panic!("V272 exchange {ordinal} should transition: {error:#}"));
        oracle
            .apply_post_receipt_uncertainty(observation_scenario(ordinal), post_receipt)
            .unwrap_or_else(|error| {
                panic!("V272 exchange {ordinal} should apply receipt: {error:#}")
            });
        responses.push(response);
        observations.push(observation);
    }

    assert_eq!(responses[0].remote_state_after, "prepared");
    assert_eq!(responses[1].remote_state_after, "committed");
    assert_eq!(responses[1].oracle_start_count_after, 1);
    assert_eq!(responses[2].oracle_start_count_after, 1);
    assert_eq!(
        observations[2].commit_uncertainty_state_after,
        "unknown_after_remote_acceptance"
    );
    assert!(observations[2].commit_uncertainty_marker_digest.is_some());
    assert_eq!(responses[3].remote_state_after, "running");
    assert_eq!(
        observations[3].commit_uncertainty_state_after,
        "resolved_by_reconcile"
    );
    assert_eq!(responses[4].remote_state_after, "terminal_after_run");
    assert_eq!(responses[4].event_count, 2);
    assert!(responses[4]
        .events
        .iter()
        .eq(responses[4].replayed_events.iter()));
    assert_eq!(
        responses[4].event_replay_classification.as_deref(),
        Some("exact_duplicate_batch_replay")
    );
    assert_eq!(responses[5].remote_state_after, "prepared");
    assert_eq!(responses[6].remote_state_after, "prepared");
    assert!(responses[6].no_commit_tombstone_digest.is_none());
    assert_eq!(responses[7].remote_state_after, "terminal_no_start");
    assert!(responses[7].no_commit_tombstone_digest.is_some());
    assert_eq!(responses[7].oracle_start_count_after, 0);
    assert_eq!(responses[7].oracle_event_count_after, 0);
}

#[test]
fn v272_linux_oracle_rejects_out_of_order_and_duplicate_transitions() {
    let mut oracle = StatefulTaskProtocolOracle::new().expect("V272 oracle should initialize");

    assert!(oracle.transition(&material(2), REQUEST_DIGEST).is_err());
    transition_and_apply(&mut oracle, 1);
    assert!(oracle.transition(&material(1), REQUEST_DIGEST).is_err());
    transition_and_apply(&mut oracle, 2);

    assert!(oracle.transition(&material(8), REQUEST_DIGEST).is_err());
    transition_and_apply(&mut oracle, 6);
    assert!(oracle.transition(&material(8), REQUEST_DIGEST).is_err());
    transition_and_apply(&mut oracle, 7);
    let terminal = transition_and_apply(&mut oracle, 8);
    assert_eq!(terminal.remote_state_after, "terminal_no_start");
}

#[test]
fn v272_linux_oracle_requires_authenticated_receipt_before_reconcile() {
    let mut oracle = StatefulTaskProtocolOracle::new().expect("V272 oracle should initialize");
    transition_and_apply(&mut oracle, 1);
    transition_and_apply(&mut oracle, 2);

    let (_, replay_observation, pending_receipt) = oracle
        .transition(&material(3), REQUEST_DIGEST)
        .expect("commit replay should produce pending uncertainty");
    assert_eq!(
        replay_observation.commit_uncertainty_state_after,
        "unknown_after_remote_acceptance"
    );
    assert!(oracle.transition(&material(4), REQUEST_DIGEST).is_err());

    oracle
        .apply_post_receipt_uncertainty("synthetic_command_a", pending_receipt)
        .expect("authenticated receipt should commit uncertainty");
    let reconciled = transition_and_apply(&mut oracle, 4);
    assert_eq!(reconciled.remote_state_after, "running");
}

fn transition_and_apply(
    oracle: &mut StatefulTaskProtocolOracle,
    ordinal: u64,
) -> FixtureUpstreamResponse {
    let (response, observation, post_receipt) = oracle
        .transition(&material(ordinal), REQUEST_DIGEST)
        .unwrap_or_else(|error| panic!("V272 exchange {ordinal} should transition: {error:#}"));
    oracle
        .apply_post_receipt_uncertainty(observation_scenario(ordinal), post_receipt)
        .unwrap_or_else(|error| panic!("V272 exchange {ordinal} should apply: {error:#}"));
    assert_eq!(
        observation.adapter_observation_id,
        format!("synthetic_task_protocol_exchange_{ordinal}")
    );
    response
}

fn material(ordinal: u64) -> ExchangeMaterial {
    exchange_material(ordinal, RUN_NONCE, FIXTURE_LANE, FIXTURE_EXECUTOR)
        .unwrap_or_else(|error| panic!("V272 exchange {ordinal} material: {error:#}"))
}

fn observation_scenario(ordinal: u64) -> &'static str {
    if ordinal <= 5 {
        "synthetic_command_a"
    } else {
        "synthetic_command_b"
    }
}
