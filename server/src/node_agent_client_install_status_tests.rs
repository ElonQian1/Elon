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
            "windowsClientDownloadUrl": "http://example.test/client.zip",
            "windowsInstallerDownloadUrl": "https://example.test/client-setup.exe",
            "windowsInstallerFileSize": 123456
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
    assert_eq!(
        status["version_manifest"]["windowsInstallerDownloadUrl"],
        "https://example.test/client-setup.exe"
    );
    assert_eq!(
        status["version_manifest"]["windowsInstallerFileSize"],
        123456
    );
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
    fs::write(start_menu.join("一龙开发平台.lnk"), "shortcut").unwrap();

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
        START_MENU_SHORTCUT_FILES.len() - 1
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
    unique_test_dir(suffix)
        .join("Programs")
        .join("一龙开发平台")
}

fn write_shortcuts(folder: &Path, names: &[&str]) {
    fs::create_dir_all(folder).unwrap();
    for name in names {
        fs::write(folder.join(name), "shortcut").unwrap();
    }
}
