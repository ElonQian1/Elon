use chrono::{Duration, Utc};

use super::super::live_artifacts::build_verify_request;
use super::super::model::{
    FitRunDocument, FitStateReplay, FitStateReplayAction, FitStateReplayStep,
};
use super::fixtures::{cleanup, run};

#[test]
fn chat_page_trace_persists_and_is_selected_for_patch_free_reinstall() {
    let (root, mut run) = run(false);
    run.environment.scenario = Some("CHAT_PAGE".into());
    run.environment.state_replay = Some(chat_page_replay(Duration::minutes(10)));

    let persisted = serde_json::to_vec(&run).unwrap();
    let restored: FitRunDocument = serde_json::from_slice(&persisted).unwrap();
    let request = build_verify_request(&restored).expect("fresh CHAT_PAGE trace");
    let replay = request
        .state_replay
        .expect("non-root page must carry its persisted trace into build verification");

    assert_eq!(replay.scenario_id, "CHAT_PAGE");
    assert!(matches!(
        replay.steps[0].action,
        FitStateReplayAction::ActivateNode { .. }
    ));
    cleanup(root);
}

#[test]
fn non_root_page_without_trace_fails_closed() {
    let (root, mut run) = run(false);
    run.environment.scenario = Some("CHAT_PAGE".into());
    let error = build_verify_request(&run).unwrap_err().to_string();
    assert!(error.contains("FIT_STATE_REPLAY_MISSING"));
    cleanup(root);
}

#[test]
fn expired_trace_fails_closed_with_explicit_diagnostic() {
    let (root, mut run) = run(false);
    run.environment.scenario = Some("CHAT_PAGE".into());
    let mut replay = chat_page_replay(Duration::minutes(10));
    let captured_at = Utc::now() - Duration::minutes(20);
    replay.captured_at = captured_at.to_rfc3339();
    replay.expires_at = (captured_at + Duration::minutes(10)).to_rfc3339();
    run.environment.state_replay = Some(replay);
    let error = build_verify_request(&run).unwrap_err().to_string();
    assert!(error.contains("FIT_STATE_REPLAY_EXPIRED"));
    cleanup(root);
}

#[test]
fn home_and_absent_scenarios_keep_the_compatible_no_replay_path() {
    for scenario in [None, Some("HOME_PAGE".to_string())] {
        let (root, mut run) = run(false);
        run.environment.scenario = scenario;
        assert!(build_verify_request(&run).unwrap().state_replay.is_none());
        cleanup(root);
    }
}

fn chat_page_replay(valid_for: Duration) -> FitStateReplay {
    let captured_at = Utc::now();
    FitStateReplay {
        schema_version: 1,
        scenario_id: "CHAT_PAGE".into(),
        captured_at: captured_at.to_rfc3339(),
        expires_at: (captured_at + valid_for).to_rfc3339(),
        steps: vec![
            FitStateReplayStep {
                name: "open-chat-tab".into(),
                action: FitStateReplayAction::ActivateNode {
                    definition_id: "home.navigation.chat".into(),
                    instance_key: None,
                    occurrence: 0,
                },
            },
            FitStateReplayStep {
                name: "wait-chat-content".into(),
                action: FitStateReplayAction::Wait { duration_ms: 500 },
            },
        ],
    }
}
