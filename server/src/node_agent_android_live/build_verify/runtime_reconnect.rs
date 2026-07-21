//! Safe fast path for an already installed, source-matching debug runtime.

use crate::node_agent_android_inspector::adb_capture::launch_app;
use crate::node_agent_android_live::adb_session::start_runtime_with_evidence;
use crate::node_agent_android_live::broker::{LiveUiBroker, LiveUiSession};
use crate::node_agent_android_live::protocol::LiveSessionView;

use super::preparation::PreparationReporter;
use super::wait_for_runtime;

pub(super) enum RuntimeReuse {
    Live(LiveSessionView),
    NeedsInstall(String),
}

pub(super) async fn ensure_live_without_install(
    broker: &LiveUiBroker,
    session: &LiveUiSession,
    host_port: u16,
    source_revision: Option<&str>,
    reporter: Option<&PreparationReporter>,
) -> RuntimeReuse {
    let current = session.view().await;
    let proof_matches = current.source_proof.as_ref().is_some_and(|proof| {
        source_revision.is_some_and(|revision| proof.source_revision == revision)
            && proof.runtime_build_id == current.runtime_build_id
    });
    if current.connected && current.node_count > 0 && proof_matches {
        evidence(
            reporter,
            "RUNTIME_RECONNECT",
            "ALREADY_LIVE",
            format!(
                "runtimeBuildId={} nodeCount={} sourceRevision={}",
                current.runtime_build_id.as_deref().unwrap_or("none"),
                current.node_count,
                source_revision.unwrap_or("none")
            ),
        )
        .await;
        return RuntimeReuse::Live(current);
    }

    phase(
        reporter,
        "RUNTIME_RECONNECT",
        "尝试启动已安装包并恢复 Runtime；失败后才安装 APK",
    )
    .await;
    session.reset_for_redeploy().await;
    let launch_output = match launch_app(&session.device_id, &session.package_name).await {
        Ok(output) => output,
        Err(error) => {
            let reason = format!("launch failed: {error:#}");
            evidence(reporter, "RUNTIME_RECONNECT", "FALLBACK_INSTALL", &reason).await;
            return RuntimeReuse::NeedsInstall(reason);
        }
    };
    let start = match start_runtime_with_evidence(session, host_port).await {
        Ok(start) => start,
        Err(error) => {
            let reason = format!("runtime start failed: {error:#}");
            evidence(reporter, "RUNTIME_RECONNECT", "FALLBACK_INSTALL", &reason).await;
            return RuntimeReuse::NeedsInstall(reason);
        }
    };
    match wait_for_runtime(broker, &session.id, session, None, &start).await {
        Ok(view) if view.connected && view.node_count > 0 => {
            evidence(
                reporter,
                "RUNTIME_RECONNECT",
                "RECONNECTED",
                format!(
                    "launchOutput={} runtimeBuildId={} nodeCount={}",
                    launch_output.trim(),
                    view.runtime_build_id.as_deref().unwrap_or("none"),
                    view.node_count
                ),
            )
            .await;
            RuntimeReuse::Live(view)
        }
        Ok(view) => {
            let reason = format!(
                "handshake incomplete: connected={} nodeCount={}",
                view.connected, view.node_count
            );
            evidence(reporter, "RUNTIME_RECONNECT", "FALLBACK_INSTALL", &reason).await;
            RuntimeReuse::NeedsInstall(reason)
        }
        Err(error) => {
            let reason = format!("handshake failed: {error:#}");
            evidence(reporter, "RUNTIME_RECONNECT", "FALLBACK_INSTALL", &reason).await;
            RuntimeReuse::NeedsInstall(reason)
        }
    }
}

async fn phase(reporter: Option<&PreparationReporter>, phase: &str, detail: impl AsRef<str>) {
    if let Some(reporter) = reporter {
        reporter.phase(phase, detail).await;
    }
}

async fn evidence(
    reporter: Option<&PreparationReporter>,
    phase: &str,
    status: &str,
    detail: impl AsRef<str>,
) {
    if let Some(reporter) = reporter {
        reporter.evidence(phase, status, detail).await;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reconnect_stage_is_explicitly_bounded_before_install() {
        assert_eq!(
            super::super::runtime_preparation::RUNTIME_HANDSHAKE_ATTEMPTS,
            2
        );
    }
}
