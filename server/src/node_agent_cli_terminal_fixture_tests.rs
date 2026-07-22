use crate::{
    node_agent_cli_pty::{default_cols, default_rows},
    node_agent_cli_sidecar::{now_ms, CliSidecarRegistry},
    node_agent_cli_sidecar_runner::{
        codex_completion_disposition, follow_sidecar_output, run_sidecar, CliSidecarLaunchConfig,
        CliSidecarRunResult, CodexCompletionDisposition,
    },
};
use tokio::sync::watch;

#[tokio::test]
async fn real_fixture_without_final_never_becomes_done() {
    let result = run_fixture(
        "no-final",
        r#"const events=[
          {type:'turn.started'},
          {type:'item.started',item:{id:'tool',type:'command_execution'}},
          {type:'item.completed',item:{id:'tool',type:'command_execution'}}
        ]; for (const event of events) console.log(JSON.stringify(event));
        console.error('fixture stderr survives');"#,
    )
    .await;
    assert!(result.exit_ok, "the real child itself exits zero");
    assert!(result.stderr_text.contains("fixture stderr survives"));
    assert_eq!(
        codex_completion_disposition(&result.stdout_text),
        CodexCompletionDisposition::ResumeRequired
    );
}

#[tokio::test]
async fn real_fixture_with_terminal_and_final_can_be_done() {
    let result = run_fixture(
        "complete-final",
        r#"const events=[
          {type:'turn.started'},
          {type:'item.completed',item:{id:'msg',type:'agent_message',text:'verified final reply'}},
          {type:'turn.completed',usage:{output_tokens:3}}
        ]; for (const event of events) console.log(JSON.stringify(event));"#,
    )
    .await;
    assert!(result.exit_ok);
    assert_eq!(
        codex_completion_disposition(&result.stdout_text),
        CodexCompletionDisposition::Complete {
            final_reply: "verified final reply".to_string()
        }
    );
}

async fn run_fixture(name: &str, script: &str) -> CliSidecarRunResult {
    let root = std::env::temp_dir().join(format!(
        "elon-codex-terminal-fixture-{name}-{}-{}",
        std::process::id(),
        now_ms()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let registry = CliSidecarRegistry::new(root.join("sidecars"));
    let task_id = format!("task-{name}");
    let session_id = format!("sidecar-{name}");
    let output_path = registry.output_path(&task_id, &session_id);
    run_sidecar(CliSidecarLaunchConfig {
        session_id,
        task_id: task_id.clone(),
        cli_name: "codex".to_string(),
        route: "route_a_external_cli".to_string(),
        program: "node".to_string(),
        args: vec![
            "-e".to_string(),
            script.to_string(),
            "--".to_string(),
            "--json".to_string(),
        ],
        cwd: None,
        runtime_permission: Some("full_access".to_string()),
        env: Vec::new(),
        output_path: output_path.clone(),
        registry_dir: registry.dir(),
        task_journal_dir: Some(root.join("journal")),
        worker_path: None,
        worker_release: None,
        worker_sha256: None,
        codex_session_scope_key: None,
        legacy_codex_sessions_file: None,
        timeout_secs: 10,
        stdin_payload: None,
        runtime_policy: None,
        stdin_piped_empty: false,
        initial_cols: default_cols(),
        initial_rows: default_rows(),
    })
    .await
    .unwrap();
    let (_cancel_tx, mut cancel_rx) = watch::channel(false);
    let result = follow_sidecar_output(&registry, &task_id, &output_path, &mut cancel_rx, |_| {})
        .await
        .unwrap();
    let _ = std::fs::remove_dir_all(root);
    result
}
