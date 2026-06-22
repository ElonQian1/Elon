// server/src/node_agent_client_install_status.rs

use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

const LEGACY_TOP_LEVEL_FILES: &[&str] = &[
    "安装一龙PC节点.cmd",
    "启动一龙节点.cmd",
    "卸载一龙PC节点.cmd",
    "install-elon-node.ps1",
    "start-node-agent.ps1",
    "tray-launcher.ps1",
    "uninstall-elon-node.ps1",
    "elon-node-agent.exe",
    "elon-node-client.exe",
    "node-agent-version.json",
    "node-agent.env",
    "node-agent.env.example",
    "README.txt",
];

pub(crate) fn status_payload() -> Value {
    #[cfg(windows)]
    {
        let install_dir = std::env::var("LOCALAPPDATA")
            .ok()
            .map(|value| PathBuf::from(value).join("ElonNode"));
        match install_dir {
            Some(install_dir) => {
                status_for_install_dir(&install_dir, std::env::current_exe().ok().as_deref())
            }
            None => json!({
                "supported": true,
                "installed": false,
                "layout_status": "unknown",
                "error": "LOCALAPPDATA is not configured",
            }),
        }
    }
    #[cfg(not(windows))]
    {
        json!({
            "supported": false,
            "installed": false,
            "layout_status": "unsupported",
            "reason": "Windows client install status is only available on Windows."
        })
    }
}

pub(crate) fn status_for_install_dir(install_dir: &Path, current_exe: Option<&Path>) -> Value {
    let client_exe = install_dir.join(crate::node_client_launcher::CLIENT_EXE_NAME);
    let uninstall_exe = install_dir.join(crate::node_client_launcher::UNINSTALL_EXE_NAME);
    let internal_dir = install_dir.join(crate::node_client_launcher::INTERNAL_DIR_NAME);
    let version_file = internal_dir.join("node-agent-version.json");
    let layout = root_layout_status(install_dir, &client_exe, &uninstall_exe, &internal_dir);
    let manifest = version_manifest_summary(&version_file);
    let installed = client_exe.exists() && uninstall_exe.exists();
    let running_from_install_dir = current_exe
        .map(|path| path.starts_with(install_dir))
        .unwrap_or(false);
    let git_sha = manifest
        .get("gitSha")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let package_version = manifest
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    json!({
        "supported": true,
        "install_dir": path_to_string(install_dir),
        "client_exe": path_to_string(&client_exe),
        "uninstall_exe": path_to_string(&uninstall_exe),
        "internal_dir": path_to_string(&internal_dir),
        "version_file": path_to_string(&version_file),
        "installed": installed,
        "running_from_install_dir": running_from_install_dir,
        "installed_git_sha": git_sha,
        "installed_package_version": package_version,
        "version_manifest": manifest,
        "layout_status": layout["status"].clone(),
        "layout": layout,
        "files": {
            "client_exe": file_meta(&client_exe),
            "uninstall_exe": file_meta(&uninstall_exe),
            "internal_dir": file_meta(&internal_dir),
            "version_file": file_meta(&version_file),
        }
    })
}

fn root_layout_status(
    install_dir: &Path,
    client_exe: &Path,
    uninstall_exe: &Path,
    internal_dir: &Path,
) -> Value {
    let expected = [
        crate::node_client_launcher::CLIENT_EXE_NAME,
        crate::node_client_launcher::UNINSTALL_EXE_NAME,
        crate::node_client_launcher::INTERNAL_DIR_NAME,
    ];
    let mut entries = Vec::new();
    let mut legacy = Vec::new();
    let mut unexpected = Vec::new();
    if let Ok(read_dir) = fs::read_dir(install_dir) {
        for entry in read_dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(name.clone());
            if LEGACY_TOP_LEVEL_FILES.iter().any(|item| *item == name) {
                legacy.push(name.clone());
            }
            if !expected.iter().any(|item| *item == name) {
                unexpected.push(name);
            }
        }
    }
    entries.sort();
    legacy.sort();
    unexpected.sort();

    let missing = [
        (crate::node_client_launcher::CLIENT_EXE_NAME, client_exe),
        (
            crate::node_client_launcher::UNINSTALL_EXE_NAME,
            uninstall_exe,
        ),
        (crate::node_client_launcher::INTERNAL_DIR_NAME, internal_dir),
    ]
    .into_iter()
    .filter_map(|(name, path)| (!path.exists()).then_some(name))
    .collect::<Vec<_>>();

    let status = if !install_dir.exists() || !missing.is_empty() {
        "incomplete"
    } else if !legacy.is_empty() {
        "legacy_files_present"
    } else if !unexpected.is_empty() {
        "unexpected_entries"
    } else {
        "clean"
    };

    json!({
        "status": status,
        "entries": entries,
        "missing_entries": missing,
        "legacy_top_level_files": legacy,
        "unexpected_top_level_entries": unexpected,
        "expected_top_level_entries": expected,
    })
}

fn version_manifest_summary(path: &Path) -> Value {
    let Some(value) = safe_json_file(path) else {
        return json!({
            "exists": false,
            "path": path_to_string(path),
        });
    };
    json!({
        "exists": true,
        "path": path_to_string(path),
        "version": value.get("version").and_then(Value::as_str),
        "gitSha": value.get("gitSha").and_then(Value::as_str),
        "updated_at": value.get("updated_at").and_then(Value::as_str),
        "downloadUrl": value.get("downloadUrl").and_then(Value::as_str),
        "linuxDownloadUrl": value.get("linuxDownloadUrl").and_then(Value::as_str),
        "windowsClientDownloadUrl": value.get("windowsClientDownloadUrl").and_then(Value::as_str),
        "fileSize": value.get("fileSize").and_then(Value::as_u64),
        "windowsClientFileSize": value.get("windowsClientFileSize").and_then(Value::as_u64),
    })
}

fn safe_json_file(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn file_meta(path: &Path) -> Value {
    match fs::metadata(path) {
        Ok(meta) => json!({
            "path": path_to_string(path),
            "exists": true,
            "is_dir": meta.is_dir(),
            "len": meta.len(),
        }),
        Err(_) => json!({
            "path": path_to_string(path),
            "exists": false,
        }),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::status_for_install_dir;
    use crate::node_client_launcher::{CLIENT_EXE_NAME, INTERNAL_DIR_NAME, UNINSTALL_EXE_NAME};
    use serde_json::json;
    use std::{fs, path::PathBuf};

    #[test]
    fn clean_layout_reports_manifest_sha() {
        let dir = unique_test_dir("clean");
        let internal = dir.join(INTERNAL_DIR_NAME);
        fs::create_dir_all(&internal).unwrap();
        fs::write(dir.join(CLIENT_EXE_NAME), "client").unwrap();
        fs::write(dir.join(UNINSTALL_EXE_NAME), "uninstall").unwrap();
        fs::write(
            internal.join("node-agent-version.json"),
            serde_json::to_string(&json!({
                "version": "0.3.68",
                "gitSha": "abc123",
                "windowsClientDownloadUrl": "http://example.test/client.zip"
            }))
            .unwrap(),
        )
        .unwrap();

        let status = status_for_install_dir(&dir, Some(&dir.join(CLIENT_EXE_NAME)));
        assert_eq!(status["installed"], true);
        assert_eq!(status["layout_status"], "clean");
        assert_eq!(status["installed_git_sha"], "abc123");
        assert_eq!(status["running_from_install_dir"], true);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn legacy_files_are_reported_separately_from_missing_entries() {
        let dir = unique_test_dir("legacy");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(CLIENT_EXE_NAME), "client").unwrap();
        fs::write(dir.join("安装一龙PC节点.cmd"), "legacy").unwrap();

        let status = status_for_install_dir(&dir, None);
        assert_eq!(status["installed"], false);
        assert_eq!(status["layout_status"], "incomplete");
        assert!(status["layout"]["legacy_top_level_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("安装一龙PC节点.cmd")));
        assert!(status["layout"]["missing_entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some(UNINSTALL_EXE_NAME)));

        let _ = fs::remove_dir_all(dir);
    }

    fn unique_test_dir(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-client-install-status-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }
}
