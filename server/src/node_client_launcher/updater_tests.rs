#[cfg(windows)]
#[test]
fn self_replace_script_stops_on_failure_before_restart() {
    use std::path::Path;

    let script = super::self_replace_script(
        Path::new(r"C:\ElonNode\_internal\一龙开发平台.exe.new"),
        Path::new(r"C:\ElonNode\一龙开发平台.exe"),
        Path::new(r"C:\ElonNode\卸载一龙开发平台.exe"),
        Path::new(r"C:\ElonNode\_internal\node-agent-version.json.new"),
        Path::new(r"C:\ElonNode\_internal\node-agent-version.json"),
    );

    assert!(script.contains("$ErrorActionPreference = 'Stop'"));
    assert!(script.contains(
        "Start-ElonNodeRuntimeAndWatchdogAndWait -Client $client -InstallDir $installDir"
    ));
    assert!(script.contains("Stop-ElonNodeClientProcesses -Client $client"));
    assert!(script.contains("_internal\\elon-desktop.exe"));
    assert!(script.contains("$matchesDesktopShell"));
    assert!(script.contains("$matchesCliSidecar"));
    assert!(script.contains("(-not $matchesCliSidecar)"));
    assert!(script.contains("Move-ElonNodeFileWithRetry -Source $tmpExe"));
    assert!(script.contains("Copy-ElonNodeFileWithRetry -Source $client"));
    assert!(script.contains("Start-Process -FilePath $Client -ArgumentList '--agent-runtime'"));
    assert!(script.contains("Start-Process -FilePath $Client -ArgumentList '--watchdog'"));
    assert!(script.contains("Wait-ElonNodeWatchdog"));
    assert!(script.contains("Wait-ElonNodeAdminHealth"));
    assert!(
        script
            .find("Move-ElonNodeFileWithRetry -Source $tmpExe")
            .unwrap()
            < script
                .find("Start-ElonNodeRuntimeAndWatchdogAndWait")
                .unwrap()
    );
    assert!(
        script
            .find("Move-ElonNodeFileWithRetry -Source $tmpVersion")
            .unwrap()
            < script
                .find("Start-ElonNodeRuntimeAndWatchdogAndWait")
                .unwrap()
    );
}

#[cfg(windows)]
#[test]
fn package_replace_script_updates_full_client_layout() {
    use std::path::Path;

    let script = super::package_replace_script(
        Some(1234),
        Path::new(r"C:\ElonNode\_internal\elon-node-agent-windows.zip.new"),
        Path::new(r"C:\ElonNode"),
        Path::new(r"C:\ElonNode\_internal\node-agent-version.json.new"),
        Path::new(r"C:\ElonNode\_internal\node-agent-version.json"),
    );

    assert!(script.contains("Wait-Process -Id 1234"));
    assert!(script.contains("+ '.zip'"));
    assert!(script.contains("Copy-Item -LiteralPath $zip -Destination $archivePath"));
    assert!(script.contains("Expand-Archive -LiteralPath $archivePath"));
    assert!(script.contains("Stop-ElonNodeClientProcesses -Client $client"));
    assert!(script.contains("$matchesCliSidecar"));
    assert!(script.contains("(-not $matchesCliSidecar)"));
    assert!(script.contains("Copy-ElonNodeFileWithRetry -Source $packageClient"));
    assert!(script.contains("Copy-ElonNodeFileWithRetry -Source $packageUninstall"));
    assert!(script.contains("Copy-Item -Path (Join-Path $packageInternal '*')"));
    assert!(script.contains("Move-ElonNodeFileWithRetry -Source $tmpVersion"));
    assert!(script.contains(
        "Start-ElonNodeRuntimeAndWatchdogAndWait -Client $client -InstallDir $installDir"
    ));
    assert!(script.contains("Start-Process -FilePath $Client -ArgumentList '--agent-runtime'"));
    assert!(script.contains("Start-Process -FilePath $Client -ArgumentList '--watchdog'"));
    assert!(script.contains("Wait-ElonNodeWatchdog"));
    assert!(script.contains("Wait-ElonNodeAdminHealth"));
}

#[cfg(windows)]
#[test]
fn package_replace_from_a_non_install_path_starts_the_installed_root() {
    use std::path::Path;

    let script = super::package_replace_script(
        None,
        Path::new(r"D:\build-cache\elon-node-agent-windows.zip.new"),
        Path::new(r"C:\Users\ELon\AppData\Local\ElonNode"),
        Path::new(r"D:\build-cache\node-agent-version.json.new"),
        Path::new(r"C:\Users\ELon\AppData\Local\ElonNode\_internal\node-agent-version.json"),
    );

    assert!(!script.contains("Wait-Process -Id"));
    assert!(script.contains("$client = Join-Path $installDir '一龙开发平台.exe'"));
    assert!(script.contains(
        "Start-ElonNodeRuntimeAndWatchdogAndWait -Client $client -InstallDir $installDir"
    ));
    assert!(script.contains("-ArgumentList '--agent-runtime' -WorkingDirectory $InstallDir"));
    assert!(script.contains("-ArgumentList '--watchdog' -WorkingDirectory $InstallDir"));
}

#[cfg(windows)]
#[test]
fn package_self_update_uses_extracted_repair_entrypoint() {
    use std::path::Path;

    let script = super::package_self_update_via_repair_script(
        1234,
        Path::new(r"C:\ElonNode\_internal\elon-node-agent-windows.zip.new"),
        Path::new(r"C:\ElonNode"),
        Path::new(r"C:\ElonNode\_internal\node-agent-version.json.new"),
    );

    assert!(script.contains("Wait-Process -Id $pidToWait"));
    assert!(script.contains("+ '.zip'"));
    assert!(script.contains("Copy-Item -LiteralPath $zip -Destination $archivePath"));
    assert!(script.contains("Expand-Archive -LiteralPath $archivePath"));
    assert!(script.contains("Stop-ElonNodeClientProcesses -Client $installedClient"));
    assert!(script.contains("$matchesCliSidecar"));
    assert!(script.contains("(-not $matchesCliSidecar)"));
    assert!(script
        .contains("Start-Process -FilePath $packageClient -ArgumentList '--repair-background'"));
    assert!(script.contains("Wait-Process -Id $repair.Id -Timeout 120"));
    assert!(script.contains("Wait-ElonNodeAdminHealth -Port $port -TimeoutSeconds 15"));
    assert!(script.contains("Wait-ElonNodeWatchdog -Client $installedClient"));
    assert!(script.contains("browser untouched"));
    assert!(script.contains("client-update.log"));
    assert!(script.contains("Remove-Item -LiteralPath $archivePath"));
    assert!(script.contains("Remove-Item -LiteralPath $tmpVersion"));
    assert!(!script.contains("Copy-ElonNodeFileWithRetry -Source $packageClient"));
    assert!(!script.contains(
        "Start-ElonNodeRuntimeAndWatchdogAndWait -Client $installedClient -InstallDir $installDir"
    ));
}

#[test]
fn runtime_handle_status_requires_complete_task_identity() {
    let status = serde_json::json!({
        "local_admin_token_header": "x-elon-local-admin-token",
        "active_cli_prompt_count": 2,
        "active_cli_prompt_task_ids": ["task-a", "task-b"],
        "active_task_runtime": [{"task_id": "task-a"}]
    });
    let task_ids = super::runtime_gate::runtime_handle_task_ids_from_status(&status).unwrap();
    assert_eq!(task_ids.len(), 2);
    assert!(task_ids.contains("task-a"));
    assert!(task_ids.contains("task-b"));

    let incomplete = serde_json::json!({
        "local_admin_token_header": "x-elon-local-admin-token",
        "active_cli_prompt_count": 2,
        "active_cli_prompt_task_ids": ["task-a"],
        "active_task_runtime": [{"task_id": "task-a"}]
    });
    assert!(super::runtime_gate::runtime_handle_task_ids_from_status(&incomplete).is_err());
    assert!(
        super::runtime_gate::runtime_handle_task_ids_from_status(&serde_json::json!({})).is_err()
    );
    let wrong_runtime = serde_json::json!({
        "local_admin_token_header": "x-not-elon-runtime",
        "active_cli_prompt_count": 0,
        "active_cli_prompt_task_ids": [],
        "active_task_runtime": []
    });
    assert!(super::runtime_gate::runtime_handle_task_ids_from_status(&wrong_runtime).is_err());
    let malformed_runtime_handle = serde_json::json!({
        "local_admin_token_header": "x-elon-local-admin-token",
        "active_cli_prompt_count": 0,
        "active_cli_prompt_task_ids": [],
        "active_task_runtime": [{}]
    });
    assert!(
        super::runtime_gate::runtime_handle_task_ids_from_status(&malformed_runtime_handle)
            .is_err()
    );
}

#[test]
fn runtime_handle_probe_failure_is_fail_closed() {
    let root = std::env::temp_dir().join(format!(
        "elon-update-runtime-probe-failure-{}",
        uuid::Uuid::new_v4().simple()
    ));
    std::fs::create_dir_all(root.join("_internal")).unwrap();
    std::fs::write(
        root.join("_internal").join("node-agent.env"),
        "NODE_ADMIN_PORT=1\n",
    )
    .unwrap();

    let error = super::runtime_gate::fresh_runtime_handle_task_ids(&root)
        .expect_err("an unavailable loopback runtime probe must refuse the install window");
    assert!(error.to_string().contains("拒绝进入安装窗口"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn update_download_bypasses_system_proxy_by_default() {
    let env_values = std::collections::HashMap::new();

    assert!(!super::update_uses_system_proxy(&env_values));
}

#[test]
fn update_download_can_opt_into_system_proxy() {
    let mut env_values = std::collections::HashMap::new();
    env_values.insert(
        "NODE_AGENT_UPDATE_USE_SYSTEM_PROXY".to_string(),
        "1".to_string(),
    );

    assert!(super::update_uses_system_proxy(&env_values));
}

#[test]
fn update_download_proxy_mode_system_uses_system_proxy() {
    let mut env_values = std::collections::HashMap::new();
    env_values.insert(
        "NODE_AGENT_UPDATE_PROXY_MODE".to_string(),
        "system".to_string(),
    );

    assert!(super::update_uses_system_proxy(&env_values));
}
