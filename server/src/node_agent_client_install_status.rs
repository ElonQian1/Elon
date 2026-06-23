// server/src/node_agent_client_install_status.rs

use serde_json::{json, Value};
#[cfg(any(windows, test))]
use std::{
    fs,
    path::{Path, PathBuf},
};

const CLIENT_EXE_NAME: &str = "一龙PC节点.exe";
const UNINSTALL_EXE_NAME: &str = "卸载一龙PC节点.exe";
const INTERNAL_DIR_NAME: &str = "_internal";
const START_MENU_FOLDER_NAME: &str = "一龙PC节点";
const START_MENU_ENTRY_NAMES: &[&str] = &[
    "一龙PC节点",
    "打开运行日志",
    "导出诊断",
    "检查更新",
    "卸载一龙PC节点",
];
#[cfg(any(windows, test))]
const START_MENU_SHORTCUT_FILES: &[&str] = &[
    "一龙PC节点.lnk",
    "打开运行日志.lnk",
    "导出诊断.lnk",
    "检查更新.lnk",
    "卸载一龙PC节点.lnk",
];

#[cfg(any(windows, test))]
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
            .map(|value| std::path::PathBuf::from(value).join("ElonNode"));
        match install_dir {
            Some(install_dir) => {
                status_for_install_dir(&install_dir, std::env::current_exe().ok().as_deref())
            }
            None => json!({
                "supported": true,
                "installed": false,
                "layout_status": "unknown",
                "product_status": unsupported_product_status("LOCALAPPDATA is not configured"),
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
            "product_status": {
                "status": "unsupported",
                "summary": "当前平台不是 Windows，无法检查一龙 PC 节点客户端安装布局。",
                "primary_entry_name": CLIENT_EXE_NAME,
                "uninstall_entry_name": UNINSTALL_EXE_NAME,
                "root_layout_expectation": root_layout_expectation(),
                "start_menu_folder_name": "一龙PC节点",
                "start_menu_entries": start_menu_entries(),
            },
            "reason": "Windows client install status is only available on Windows."
        })
    }
}

#[cfg(any(windows, test))]
pub(crate) fn status_for_install_dir(install_dir: &Path, current_exe: Option<&Path>) -> Value {
    let start_menu_folder = default_start_menu_folder();
    status_for_install_dir_with_start_menu(install_dir, current_exe, start_menu_folder.as_deref())
}

#[cfg(any(windows, test))]
fn status_for_install_dir_with_start_menu(
    install_dir: &Path,
    current_exe: Option<&Path>,
    start_menu_folder: Option<&Path>,
) -> Value {
    let client_exe = install_dir.join(CLIENT_EXE_NAME);
    let uninstall_exe = install_dir.join(UNINSTALL_EXE_NAME);
    let internal_dir = install_dir.join(INTERNAL_DIR_NAME);
    let version_file = internal_dir.join("node-agent-version.json");
    let layout = root_layout_status(install_dir, &client_exe, &uninstall_exe, &internal_dir);
    let start_menu = start_menu_status(start_menu_folder);
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
    let product_status = product_status(installed, running_from_install_dir, &layout, &start_menu);

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
        "start_menu": start_menu,
        "product_status": product_status,
        "files": {
            "client_exe": file_meta(&client_exe),
            "uninstall_exe": file_meta(&uninstall_exe),
            "internal_dir": file_meta(&internal_dir),
            "version_file": file_meta(&version_file),
        }
    })
}

#[cfg(any(windows, test))]
fn product_status(
    installed: bool,
    running_from_install_dir: bool,
    layout: &Value,
    start_menu: &Value,
) -> Value {
    let layout_status = layout["status"].as_str().unwrap_or("unknown");
    let start_menu_status = start_menu
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let missing_entry_count = array_len(layout.get("missing_entries"));
    let legacy_file_count = array_len(layout.get("legacy_top_level_files"));
    let unexpected_entry_count = array_len(layout.get("unexpected_top_level_entries"));
    let missing_start_menu_entry_count = array_len(start_menu.get("missing_entries"));
    let (status, summary) = if !installed {
        (
            "needs_repair",
            format!("安装不完整；请重新运行 {CLIENT_EXE_NAME}，客户端会自动修复安装目录。"),
        )
    } else if matches!(start_menu_status, "missing" | "incomplete") {
        (
            "repair_recommended",
            format!(
                "客户端可用，但开始菜单维护入口不完整；重新运行 {CLIENT_EXE_NAME} 会自动修复。"
            ),
        )
    } else if layout_status == "clean" && running_from_install_dir {
        (
            "ready",
            format!("正常；日常只运行 {CLIENT_EXE_NAME}，卸载只运行 {UNINSTALL_EXE_NAME}。"),
        )
    } else if layout_status == "clean" {
        (
            "ready_external_launch",
            format!("安装目录正常；日常入口是 {CLIENT_EXE_NAME}，当前进程不是从安装目录启动。"),
        )
    } else {
        (
            "cleanup_recommended",
            format!("客户端可用，但安装目录仍有旧文件或额外文件；重新运行 {CLIENT_EXE_NAME} 会自动收敛布局。"),
        )
    };

    let mut recommended_actions = Vec::new();
    if !installed {
        recommended_actions.push(format!("重新运行 {CLIENT_EXE_NAME} 修复安装。"));
    } else if layout_status != "clean" {
        recommended_actions.push(format!("重新运行 {CLIENT_EXE_NAME} 收敛安装目录。"));
    }
    if installed && matches!(start_menu_status, "missing" | "incomplete") {
        recommended_actions.push(format!("重新运行 {CLIENT_EXE_NAME} 修复开始菜单维护入口。"));
    }
    if installed && !running_from_install_dir {
        recommended_actions.push(format!("以后从安装目录运行 {CLIENT_EXE_NAME}。"));
    }
    if recommended_actions.is_empty() {
        recommended_actions.push("无需处理；保持只运行主程序和卸载程序。".to_string());
    }

    json!({
        "status": status,
        "summary": summary,
        "primary_entry_name": CLIENT_EXE_NAME,
        "uninstall_entry_name": UNINSTALL_EXE_NAME,
        "root_layout_expectation": root_layout_expectation(),
        "start_menu_folder_name": "一龙PC节点",
        "start_menu_entries": start_menu_entries(),
        "missing_entry_count": missing_entry_count,
        "legacy_file_count": legacy_file_count,
        "unexpected_entry_count": unexpected_entry_count,
        "start_menu_status": start_menu_status,
        "missing_start_menu_entry_count": missing_start_menu_entry_count,
        "recommended_actions": recommended_actions,
    })
}

#[cfg(any(windows, test))]
fn array_len(value: Option<&Value>) -> usize {
    value.and_then(Value::as_array).map(Vec::len).unwrap_or(0)
}

#[cfg(windows)]
fn unsupported_product_status(reason: &str) -> Value {
    json!({
        "status": "needs_repair",
        "summary": format!("无法确认安装目录：{reason}。请重新运行 {CLIENT_EXE_NAME} 修复。"),
        "primary_entry_name": CLIENT_EXE_NAME,
        "uninstall_entry_name": UNINSTALL_EXE_NAME,
        "root_layout_expectation": root_layout_expectation(),
        "start_menu_folder_name": "一龙PC节点",
        "start_menu_entries": start_menu_entries(),
    })
}

fn root_layout_expectation() -> String {
    format!("{CLIENT_EXE_NAME}、{UNINSTALL_EXE_NAME}、{INTERNAL_DIR_NAME}")
}

fn start_menu_entries() -> Vec<&'static str> {
    START_MENU_ENTRY_NAMES.to_vec()
}

#[cfg(any(windows, test))]
fn root_layout_status(
    install_dir: &Path,
    client_exe: &Path,
    uninstall_exe: &Path,
    internal_dir: &Path,
) -> Value {
    let expected = [CLIENT_EXE_NAME, UNINSTALL_EXE_NAME, INTERNAL_DIR_NAME];
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
        (CLIENT_EXE_NAME, client_exe),
        (UNINSTALL_EXE_NAME, uninstall_exe),
        (INTERNAL_DIR_NAME, internal_dir),
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

#[cfg(any(windows, test))]
fn default_start_menu_folder() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|appdata| {
        PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join(START_MENU_FOLDER_NAME)
    })
}

#[cfg(any(windows, test))]
fn start_menu_status(folder: Option<&Path>) -> Value {
    let Some(folder) = folder else {
        return json!({
            "status": "unknown",
            "folder": Value::Null,
            "folder_name": START_MENU_FOLDER_NAME,
            "entry_names": START_MENU_ENTRY_NAMES,
            "expected_entries": START_MENU_SHORTCUT_FILES,
            "entries": [],
            "missing_entries": [],
        });
    };

    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(folder) {
        for entry in read_dir.flatten() {
            entries.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    entries.sort();

    let missing = START_MENU_SHORTCUT_FILES
        .iter()
        .filter_map(|name| (!folder.join(name).exists()).then_some(*name))
        .collect::<Vec<_>>();
    let status = if !folder.exists() {
        "missing"
    } else if !missing.is_empty() {
        "incomplete"
    } else {
        "clean"
    };

    json!({
        "status": status,
        "folder": path_to_string(folder),
        "folder_name": START_MENU_FOLDER_NAME,
        "entry_names": START_MENU_ENTRY_NAMES,
        "expected_entries": START_MENU_SHORTCUT_FILES,
        "entries": entries,
        "missing_entries": missing,
    })
}

#[cfg(any(windows, test))]
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

#[cfg(any(windows, test))]
fn safe_json_file(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

#[cfg(any(windows, test))]
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

#[cfg(any(windows, test))]
fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::{
        status_for_install_dir, status_for_install_dir_with_start_menu, CLIENT_EXE_NAME,
        INTERNAL_DIR_NAME, START_MENU_SHORTCUT_FILES, UNINSTALL_EXE_NAME,
    };
    use serde_json::json;
    use std::{
        fs,
        path::{Path, PathBuf},
    };

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

        let start_menu = start_menu_dir("clean-menu");
        write_shortcuts(&start_menu, START_MENU_SHORTCUT_FILES);
        let status = status_for_install_dir_with_start_menu(
            &dir,
            Some(&dir.join(CLIENT_EXE_NAME)),
            Some(&start_menu),
        );
        assert_eq!(status["installed"], true);
        assert_eq!(status["layout_status"], "clean");
        assert_eq!(status["start_menu"]["status"], "clean");
        assert_eq!(status["installed_git_sha"], "abc123");
        assert_eq!(status["running_from_install_dir"], true);
        assert_eq!(status["product_status"]["status"], "ready");
        assert_eq!(status["product_status"]["missing_entry_count"], 0);
        assert_eq!(
            status["product_status"]["missing_start_menu_entry_count"],
            0
        );
        assert_eq!(status["product_status"]["legacy_file_count"], 0);
        assert_eq!(status["product_status"]["unexpected_entry_count"], 0);
        assert!(status["product_status"]["summary"]
            .as_str()
            .unwrap()
            .contains(CLIENT_EXE_NAME));
        assert_eq!(
            status["product_status"]["primary_entry_name"],
            CLIENT_EXE_NAME
        );
        assert!(status["product_status"]["start_menu_entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("导出诊断")));

        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(start_menu);
    }

    #[test]
    fn complete_layout_with_legacy_files_is_actionable_not_confusing() {
        let dir = unique_test_dir("complete-legacy");
        let internal = dir.join(INTERNAL_DIR_NAME);
        fs::create_dir_all(&internal).unwrap();
        fs::write(dir.join(CLIENT_EXE_NAME), "client").unwrap();
        fs::write(dir.join(UNINSTALL_EXE_NAME), "uninstall").unwrap();
        fs::write(dir.join("启动一龙节点.cmd"), "legacy").unwrap();

        let status =
            status_for_install_dir_with_start_menu(&dir, Some(&dir.join(CLIENT_EXE_NAME)), None);
        assert_eq!(status["installed"], true);
        assert_eq!(status["layout_status"], "legacy_files_present");
        assert_eq!(status["product_status"]["status"], "cleanup_recommended");
        assert_eq!(status["product_status"]["legacy_file_count"], 1);
        assert!(status["product_status"]["recommended_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str().unwrap_or_default().contains("收敛安装目录")));
        assert!(status["product_status"]["summary"]
            .as_str()
            .unwrap()
            .contains("重新运行"));

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
        assert_eq!(status["product_status"]["status"], "needs_repair");
        assert_eq!(status["product_status"]["missing_entry_count"], 2);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_start_menu_shortcuts_are_actionable() {
        let dir = unique_test_dir("missing-start-menu");
        let internal = dir.join(INTERNAL_DIR_NAME);
        let start_menu = start_menu_dir("partial-menu");
        fs::create_dir_all(&internal).unwrap();
        fs::create_dir_all(&start_menu).unwrap();
        fs::write(dir.join(CLIENT_EXE_NAME), "client").unwrap();
        fs::write(dir.join(UNINSTALL_EXE_NAME), "uninstall").unwrap();
        fs::write(start_menu.join("一龙PC节点.lnk"), "shortcut").unwrap();

        let status = status_for_install_dir_with_start_menu(
            &dir,
            Some(&dir.join(CLIENT_EXE_NAME)),
            Some(&start_menu),
        );

        assert_eq!(status["installed"], true);
        assert_eq!(status["layout_status"], "clean");
        assert_eq!(status["start_menu"]["status"], "incomplete");
        assert_eq!(status["product_status"]["status"], "repair_recommended");
        assert_eq!(
            status["product_status"]["missing_start_menu_entry_count"],
            4
        );
        assert!(status["start_menu"]["missing_entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item.as_str() == Some("打开运行日志.lnk")));
        assert!(status["product_status"]["recommended_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item
                .as_str()
                .unwrap_or_default()
                .contains("开始菜单维护入口")));

        let _ = fs::remove_dir_all(dir);
        let _ = fs::remove_dir_all(start_menu);
    }

    fn unique_test_dir(suffix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-client-install-status-test-{}-{}",
            std::process::id(),
            suffix
        ))
    }

    fn start_menu_dir(suffix: &str) -> PathBuf {
        unique_test_dir(suffix).join("Programs").join("一龙PC节点")
    }

    fn write_shortcuts(folder: &Path, names: &[&str]) {
        fs::create_dir_all(folder).unwrap();
        for name in names {
            fs::write(folder.join(name), "shortcut").unwrap();
        }
    }
}
