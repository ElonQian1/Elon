use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};

use super::model::{FitStateReplay, FitStateReplayAction};
use crate::node_agent_android_inspector::adb_command::{run_adb_text, validate_device_id};
use crate::node_agent_android_live::broker::{LiveUiBroker, LiveUiSession};
use crate::node_agent_android_live::protocol::{LiveSessionView, LiveUiNode};

const TARGET_TIMEOUT: Duration = Duration::from_secs(15);
const ACTION_SETTLE: Duration = Duration::from_millis(350);

pub(crate) async fn replay_after_reinstall(
    broker: &LiveUiBroker,
    session_id: &str,
    session: &LiveUiSession,
    replay: &FitStateReplay,
    target_definition_id: &str,
    target_instance_key: Option<&str>,
) -> Result<LiveSessionView> {
    validate_device_id(&session.device_id)?;
    for step in &replay.steps {
        perform_action(session, &step.action)
            .await
            .with_context(|| {
                format!(
                    "FIT_STATE_REPLAY_FAILED: scenario={} step={}",
                    replay.scenario_id, step.name
                )
            })?;
        if !matches!(step.action, FitStateReplayAction::Wait { .. }) {
            tokio::time::sleep(ACTION_SETTLE).await;
        }
    }
    wait_for_target(
        broker,
        session_id,
        session,
        replay,
        target_definition_id,
        target_instance_key,
    )
    .await
}

async fn perform_action(session: &LiveUiSession, action: &FitStateReplayAction) -> Result<()> {
    match action {
        FitStateReplayAction::ActivateNode {
            definition_id,
            instance_key,
            occurrence,
        } => {
            let component = format!(
                "{}/com.elon.uiruntime.view.UiRuntimeControlReceiver",
                session.package_name
            );
            let mut args = vec![
                "-s".to_string(),
                session.device_id.clone(),
                "shell".to_string(),
                "am".to_string(),
                "broadcast".to_string(),
                "-n".to_string(),
                component,
                "-a".to_string(),
                "com.elon.uiruntime.ACTIVATE_NODE".to_string(),
                "--es".to_string(),
                "definition_id".to_string(),
                definition_id.clone(),
                "--ei".to_string(),
                "occurrence".to_string(),
                occurrence.to_string(),
            ];
            if let Some(instance_key) = instance_key.as_deref().filter(|value| !value.is_empty()) {
                args.extend([
                    "--es".to_string(),
                    "instance_key".to_string(),
                    instance_key.to_string(),
                ]);
            }
            let output = run_adb_text(&args, Duration::from_secs(5), 64 * 1024).await?;
            if !output.contains("result=-1") {
                bail!("ACTIVATE_NODE 未成功: {}", output.trim());
            }
        }
        FitStateReplayAction::Back => {
            let args = vec![
                "-s".to_string(),
                session.device_id.clone(),
                "shell".to_string(),
                "input".to_string(),
                "keyevent".to_string(),
                "KEYCODE_BACK".to_string(),
            ];
            run_adb_text(&args, Duration::from_secs(5), 64 * 1024).await?;
        }
        FitStateReplayAction::Wait { duration_ms } => {
            tokio::time::sleep(Duration::from_millis(*duration_ms)).await;
        }
    }
    Ok(())
}

async fn wait_for_target(
    broker: &LiveUiBroker,
    session_id: &str,
    session: &LiveUiSession,
    replay: &FitStateReplay,
    target_definition_id: &str,
    target_instance_key: Option<&str>,
) -> Result<LiveSessionView> {
    let started = Instant::now();
    loop {
        let view = session.view().await;
        let (_, nodes) = broker.tree(session_id).await?;
        match target_match_count(&nodes, target_definition_id, target_instance_key) {
            1 if view.connected => return Ok(view),
            count if count > 1 && target_instance_key.is_none() => {
                bail!(
                    "FIT_STATE_REPLAY_TARGET_AMBIGUOUS: scenario={} definitionId={} count={count}",
                    replay.scenario_id,
                    target_definition_id
                );
            }
            _ => {}
        }
        if started.elapsed() > TARGET_TIMEOUT {
            return Err(target_mismatch_error(
                replay,
                target_definition_id,
                target_instance_key,
                &view,
            ));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn target_mismatch_error(
    replay: &FitStateReplay,
    target_definition_id: &str,
    target_instance_key: Option<&str>,
    view: &LiveSessionView,
) -> anyhow::Error {
    anyhow::anyhow!(
        "FIT_STATE_REPLAY_TARGET_MISMATCH: scenario={} 重放后目标状态不匹配；definitionId={} instanceKey={:?} connected={} nodeCount={}",
        replay.scenario_id,
        target_definition_id,
        target_instance_key,
        view.connected,
        view.node_count
    )
}

fn target_match_count(
    nodes: &[LiveUiNode],
    definition_id: &str,
    instance_key: Option<&str>,
) -> usize {
    nodes
        .iter()
        .filter(|node| {
            node.definition_id == definition_id
                && instance_key.is_none_or(|key| node.instance_key.as_deref() == Some(key))
                && node.geometry.visible
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_agent_android_live::protocol::{LiveGeometry, LiveRect};

    #[test]
    fn chat_page_replay_target_is_reached_only_by_the_expected_node() {
        let nodes = vec![
            node("home.feed", None, true),
            node("chat.message.composer", Some("primary"), true),
        ];
        assert_eq!(
            target_match_count(&nodes, "chat.message.composer", Some("primary")),
            1
        );
        assert_eq!(
            target_match_count(&nodes, "chat.message.composer", Some("other")),
            0
        );
    }

    #[test]
    fn replay_target_rejects_hidden_or_ambiguous_nodes() {
        let nodes = vec![
            node("chat.message.composer", None, false),
            node("chat.message.composer", None, true),
            node("chat.message.composer", None, true),
        ];
        assert_eq!(target_match_count(&nodes, "chat.message.composer", None), 2);
    }

    #[test]
    fn replay_failure_diagnostic_never_fabricates_target_reached() {
        let replay: FitStateReplay = serde_json::from_value(serde_json::json!({
            "scenarioId":"CHAT_PAGE",
            "capturedAt":"2026-07-23T00:00:00Z",
            "expiresAt":"2026-07-24T00:00:00Z",
            "steps":[{"name":"open-chat","action":{"type":"WAIT","durationMs":100}}]
        }))
        .unwrap();
        let view = LiveSessionView {
            id: "session".into(),
            device_id: "device".into(),
            package_name: "package".into(),
            project_root: None,
            device_port: 0,
            created_at: "now".into(),
            connected: true,
            runtime_build_id: Some("build".into()),
            runtime_version: None,
            tree_revision: 1,
            node_count: 3,
            history_count: 0,
            redo_count: 0,
            source_proof: None,
            last_seen_at: None,
            last_error: None,
        };
        let diagnostic =
            target_mismatch_error(&replay, "chat.message.composer", None, &view).to_string();
        assert!(diagnostic.contains("FIT_STATE_REPLAY_TARGET_MISMATCH"));
        assert!(!diagnostic.contains("TARGET_REACHED"));
    }

    fn node(definition_id: &str, instance_key: Option<&str>, visible: bool) -> LiveUiNode {
        LiveUiNode {
            runtime_node_id: format!("runtime-{definition_id}"),
            definition_id: definition_id.to_string(),
            instance_key: instance_key.map(str::to_string),
            parent_runtime_node_id: None,
            screen_id: "screen".into(),
            kind: "node".into(),
            text: None,
            resource_id: None,
            class_name: "Node".into(),
            source: None,
            geometry: LiveGeometry {
                bounds_in_display_px: LiveRect {
                    left: 0,
                    top: 0,
                    right: 10,
                    bottom: 10,
                    width: 10,
                    height: 10,
                },
                density: 1.0,
                font_scale: 1.0,
                rotation: 0,
                visible,
            },
            properties: Default::default(),
            capabilities: Default::default(),
        }
    }
}
