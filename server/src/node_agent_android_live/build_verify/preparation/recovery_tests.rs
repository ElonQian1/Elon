use super::*;
use std::time::Duration;

fn git(root: &std::path::Path, args: &[&str]) {
    let result = crate::git_command_error::git_command()
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

async fn terminal(
    broker: &Arc<LiveUiBroker>,
    request: &PrepareDebugRuntimeRequest,
    owner: &str,
) -> PrepareDebugRuntimeProgress {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let progress = broker
                .debug_runtime_preparations
                .poll_or_start(broker.clone(), request.clone(), 1, false, owner)
                .await
                .unwrap();
            if progress.status != "IN_PROGRESS" {
                return progress;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("preparation must reach a bounded terminal result")
}

#[tokio::test]
async fn failed_preparation_owner_can_restart_without_losing_renderer_identity() {
    // A real Git candidate with no Android project fails before ADB/build.
    // The full node uses its real bootstrap; the isolated harness substitutes
    // only the external build boundary with an explicit failure injector.
    let root = std::env::temp_dir().join(format!("prepare-recovery-{}", uuid::Uuid::new_v4()));
    let source = root.join("source");
    std::fs::create_dir_all(&source).unwrap();
    git(&source, &["init"]);
    git(
        &source,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&source, &["config", "user.name", "Recovery fixture"]);
    std::fs::write(source.join("tracked.txt"), "base").unwrap();
    git(&source, &["add", "."]);
    git(&source, &["commit", "-m", "base"]);
    let broker = Arc::new(LiveUiBroker::for_node(
        "recovery-test",
        root.join("integration"),
    ));
    let owner = broker
        .create_session(
            "ui-design-bootstrap".into(),
            "ui.design.bootstrap".into(),
            Some(source.to_string_lossy().into_owned()),
            1,
        )
        .await;
    let other = broker
        .create_session(
            "ui-design-bootstrap".into(),
            "ui.design.bootstrap".into(),
            Some(source.to_string_lossy().into_owned()),
            1,
        )
        .await;
    let request = PrepareDebugRuntimeRequest {
        device_id: "emulator-5554".into(),
        base_package_name: "com.example.recovery".into(),
        project_root: source.to_string_lossy().into_owned(),
        debug_application_id_suffix: ".uitest".into(),
        isolated_emulator_package: true,
        lkg_enabled: false,
        candidate: None,
        lease: None,
        integration_plan: None,
    };
    let failed = terminal(&broker, &request, &owner.id).await;
    assert_eq!(failed.status, "FAILED");
    let owned = broker
        .debug_runtime_preparations
        .owned_runtime_session_ids(&owner.id)
        .await;
    assert_eq!(owned.len(), 1);
    let runtime_id = owned.iter().next().unwrap();
    assert!(!broker
        .renderer_devices_owned_by_other_sessions(&owner.id)
        .await
        .contains("emulator-5554"));
    assert!(broker
        .renderer_devices_owned_by_other_sessions(&other.id)
        .await
        .contains("emulator-5554"));
    let error = tokio::time::timeout(
        Duration::from_secs(2),
        broker.debug_runtime_preparations.poll_or_start(
            broker.clone(),
            request.clone(),
            1,
            true,
            &other.id,
        ),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(error.to_string().contains("ANDROID_RENDERER_BUSY"));
    let restarted = broker
        .debug_runtime_preparations
        .poll_or_start(broker.clone(), request.clone(), 1, true, &owner.id)
        .await
        .unwrap();
    assert_ne!(restarted.operation_id, failed.operation_id);
    assert_eq!(restarted.generation, failed.generation); // clean cache remains reusable
    assert_eq!(
        broker
            .debug_runtime_preparations
            .owned_runtime_session_ids(&owner.id)
            .await,
        owned
    );
    let failed_again = terminal(&broker, &request, &owner.id).await;
    assert_eq!(failed_again.status, "FAILED");
    assert_eq!(
        broker
            .debug_runtime_preparations
            .owned_runtime_session_ids(&owner.id)
            .await
            .iter()
            .next(),
        Some(runtime_id)
    );

    std::fs::write(source.join("tracked.txt"), "fixed source").unwrap();
    git(&source, &["commit", "-am", "fixed source"]);
    let plan = prepare_integration_plan(&broker, &request).unwrap();
    let prior = broker
        .debug_integration
        .status(&plan.slot_id)
        .unwrap()
        .unwrap();
    let error = broker
        .debug_runtime_preparations
        .poll_or_start(broker.clone(), request.clone(), 1, true, &other.id)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("ANDROID_RENDERER_BUSY"));
    assert_eq!(
        broker
            .debug_integration
            .status(&plan.slot_id)
            .unwrap()
            .unwrap()
            .desired_generation,
        prior.desired_generation
    );
    let next = broker
        .debug_runtime_preparations
        .poll_or_start(broker.clone(), request.clone(), 1, true, &owner.id)
        .await
        .unwrap();
    assert!(next.generation > failed.generation);
    assert_ne!(next.source_revision, failed.source_revision);
    assert_eq!(
        terminal(&broker, &request, &owner.id).await.status,
        "FAILED"
    );
    tokio::time::timeout(
        Duration::from_secs(1),
        broker
            .debug_deployments
            .acquire("emulator-5554", "com.example.recovery.uitest"),
    )
    .await
    .expect("failure releases only its deployment guard")
    .unwrap();
    let from_runtime = broker
        .debug_runtime_preparations
        .poll_or_start(broker.clone(), request.clone(), 1, true, runtime_id)
        .await
        .unwrap();
    assert_eq!(from_runtime.status, "IN_PROGRESS");
    assert_eq!(
        terminal(&broker, &request, &owner.id).await.status,
        "FAILED"
    );
    assert_eq!(
        broker
            .debug_runtime_preparations
            .owned_runtime_session_ids(&owner.id)
            .await,
        owned
    );
    broker.remove_session(runtime_id).await.unwrap();
    let released = broker
        .debug_runtime_preparations
        .poll_or_start(broker.clone(), request.clone(), 1, true, &other.id)
        .await
        .unwrap();
    assert_eq!(released.status, "IN_PROGRESS");
    assert_eq!(
        terminal(&broker, &request, &other.id).await.status,
        "FAILED"
    );
    drop(broker);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn runtime_session_can_poll_its_own_in_progress_preparation() {
    let registry = PreparationRegistry::default();
    let state = Arc::new(RwLock::new(PreparationState {
        owner_session_id: "bootstrap".into(),
        runtime_session_id: None,
        device_id: "emulator-5554".into(),
        operation_id: "op".into(),
        status: "IN_PROGRESS".into(),
        phase: "BUILD".into(),
        source_revision: None,
        integration_revision: None,
        generation: 1,
        commits: vec![],
        evidence: vec![],
        result: None,
        error: None,
    }));
    registry.operations.lock().await.insert(
        PreparationKey {
            slot_id: "slot".into(),
            package_name: "package".into(),
            device_id: "emulator-5554".into(),
            lkg_enabled: false,
        },
        state.clone(),
    );
    PreparationReporter { state }.bind_runtime("runtime").await;
    assert!(registry.busy_device_ids_except("runtime").await.is_empty());
    assert!(registry
        .busy_device_ids_except("bootstrap")
        .await
        .is_empty());
    assert!(registry
        .busy_device_ids_except("other")
        .await
        .contains("emulator-5554"));
}
