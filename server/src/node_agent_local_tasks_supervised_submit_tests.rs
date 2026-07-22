use std::{fs, path::Path, sync::Arc};

use axum::{body::to_bytes, body::Body, http::Request};
use homecli_proto::CliProjectContext;
use serde_json::json;
use tower::ServiceExt;

use super::*;

#[tokio::test]
async fn supervised_submit_requires_exact_grant_then_persists_dispatch_authority() {
    let root = std::env::temp_dir().join(format!(
        "elon-supervised-submit-{}",
        uuid::Uuid::new_v4().simple()
    ));
    let base = root.join("base");
    fs::create_dir_all(&base).unwrap();
    git(&base, &["init", "-b", "main"]);
    git(&base, &["config", "user.email", "tests@example.invalid"]);
    git(&base, &["config", "user.name", "Tests"]);
    git(
        &base,
        &[
            "config",
            "remote.origin.url",
            "https://example.invalid/elon.git",
        ],
    );
    fs::write(base.join("seed.txt"), "seed\n").unwrap();
    git(&base, &["add", "seed.txt"]);
    git(&base, &["commit", "-m", "seed"]);

    let owner = "owner-supervised-submit";
    let agent = "agent-supervised-submit";
    let install = "install-supervised-submit";
    let paths = elon_pc_dev_runtime::NodeDataPaths::new(root.join("node-data"));
    fs::create_dir_all(paths.workspaces()).unwrap();
    let mut runtime = crate::NodeRuntime::new(
        crate::node_agent_config::NodeConfig {
            cloud_url: "ws://127.0.0.1".into(),
            cloud_http_url: "http://127.0.0.1".into(),
            ollama_url: "http://127.0.0.1".into(),
            lm_studio_url: None,
            custom_url: None,
            price_per_1k: 0.0,
        },
        Some(crate::node_agent_config::Credentials {
            agent_id: agent.into(),
            agent_secret: "unused".into(),
            owner_user_id: owner.into(),
            user_token: None,
        }),
        crate::pc_storage_repo::StorageSettings::default(),
        crate::node_agent_data_root::NodeDataRootState::from_prepared_paths(
            paths,
            crate::node_agent_data_root::NodeDataRootSource::Persisted,
            None,
            None,
        ),
        install.into(),
    );
    runtime.local_tasks =
        crate::node_agent_local_task_store::LocalTaskStore::new(root.join("tasks.sqlite3"));
    runtime.task_journal = crate::node_agent_task_journal::TaskJournal::new(root.join("journal"));
    runtime.full_access_grants =
        crate::node_agent_full_access::FullAccessGrantState::load_from_path(
            root.join("full-access.json"),
        );

    let prompt = "CAPTURE_SUPERVISED_SUBMIT_DISPATCH_AUTHORITY";
    let conversation_id = format!("supervised-submit-{}", uuid::Uuid::new_v4().simple());
    super::tests::install_dispatch_capture(prompt);
    let body = json!({
        "project_id": "elon-self",
        "conversation_id": conversation_id,
        "workspace_path": base,
        "prompt": prompt,
        "runtime_permission": "full_access",
        "supervision": {
            "protocol": crate::node_agent_local_task_supervision::SUPERVISION_PROTOCOL,
            "supervisor": "codex_desktop",
            "task_role": "requirement",
            "acceptance_criteria": ["CLI starts"],
            "improvement_policy": "after_task_only"
        }
    });
    let runtime = Arc::new(runtime);
    let rejected = super::routes()
        .with_state(runtime.clone())
        .oneshot(post(&body))
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        git_output(&base, &["worktree", "list", "--porcelain"])
            .matches("worktree ")
            .count(),
        1
    );

    let identity =
        crate::node_agent_full_access::FullAccessGrantIdentity::new(owner, agent, install).unwrap();
    runtime
        .full_access_grants
        .grant_project(&identity, "elon-self", base.to_str().unwrap())
        .await
        .unwrap();
    let accepted = super::routes()
        .with_state(runtime.clone())
        .oneshot(post(&body))
        .await
        .unwrap();
    let accepted_status = accepted.status();
    let accepted_body = to_bytes(accepted.into_body(), usize::MAX).await.unwrap();
    assert_eq!(
        accepted_status,
        StatusCode::ACCEPTED,
        "{}",
        String::from_utf8_lossy(&accepted_body)
    );
    let response: serde_json::Value = serde_json::from_slice(&accepted_body).unwrap();
    let task_id = response["task_id"].as_str().unwrap();
    let record = runtime.local_tasks.get(task_id).unwrap().unwrap();
    let status = record.workspace_status.as_ref().unwrap();
    assert_eq!(
        status["platform_provenance"],
        "elon.conversation_worktree.v1"
    );
    assert_eq!(status["root_task_id"], task_id);
    assert_eq!(status["git_head"].as_str().unwrap().len(), 40);
    let active = status["active_workspace_path"].as_str().unwrap();
    assert_eq!(
        crate::node_agent_supervision_worktree_lease::worktree_lock_reason(
            &base,
            Path::new(active),
        )
        .unwrap()
        .as_deref(),
        Some(format!("elon-supervision:{task_id}").as_str())
    );

    let context = CliProjectContext {
        project_id: "elon-self".into(),
        conversation_id: record.conversation_id.clone(),
        runtime_permission: Some("full_access".into()),
    };
    crate::node_agent_full_access::require_route_a_full_access_grant(
        &runtime.full_access_grants,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context),
        Some(active),
        false,
        Some(&record),
    )
    .await
    .expect("persisted Submit evidence must independently authorize dispatch");

    let mut forged = record.clone();
    forged.workspace_status.as_mut().unwrap()["git_head"] =
        json!("0000000000000000000000000000000000000000");
    let forged_result = crate::node_agent_full_access::require_route_a_full_access_grant(
        &runtime.full_access_grants,
        &identity,
        "codex",
        Some("full_access"),
        Some(&context),
        Some(active),
        false,
        Some(&forged),
    )
    .await;
    assert!(
        forged_result.is_err(),
        "forged git_head was authorized; active={active}, status={:?}",
        forged.workspace_status
    );
    assert_eq!(super::tests::take_dispatch_captures(prompt).len(), 1);
    git(&base, &["worktree", "unlock", active]);
    git(&base, &["worktree", "remove", "--force", active]);
    let _ = fs::remove_dir_all(root);
}

fn post(body: &serde_json::Value) -> Request<Body> {
    Request::post("/api/local-tasks")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(body).unwrap()))
        .unwrap()
}

fn git(cwd: &Path, args: &[&str]) {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(cwd: &Path, args: &[&str]) -> String {
    let output = crate::git_command_error::git_command()
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout).into_owned()
}
