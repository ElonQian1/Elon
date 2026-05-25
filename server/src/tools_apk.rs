//! APK 产物查找辅助函数（从 tools.rs 抽出）。
//!
//! 这些函数只读取 worktree 文件系统，不修改任何状态；负责定位 Android 构建产物
//! 中最新的 APK，或按文件名查找指定 APK。

use std::path::Path;

pub const STABLE_APK_FILENAME: &str = "latest.apk";

pub fn stable_apk_url(download_base: &str) -> String {
    format!(
        "{}/{}",
        download_base.trim_end_matches('/'),
        STABLE_APK_FILENAME
    )
}

pub fn find_latest_apk(work_dir: &Path) -> Option<std::path::PathBuf> {
    let matches = collect_apks(work_dir);
    matches.into_iter().max_by_key(|p| {
        p.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    })
}

pub fn find_apk_by_filename(work_dir: &Path, filename: &str) -> Option<std::path::PathBuf> {
    collect_apks(work_dir)
        .into_iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) == Some(filename))
        .max_by_key(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        })
}

pub fn find_download_apk(work_dir: &Path, filename: &str) -> Option<std::path::PathBuf> {
    if filename == STABLE_APK_FILENAME {
        find_latest_apk(work_dir)
    } else {
        find_apk_by_filename(work_dir, filename)
    }
}

fn collect_apks(work_dir: &Path) -> Vec<std::path::PathBuf> {
    let dirs = [
        "app/build/outputs/apk",
        "android/app/build/outputs/apk",
        "build",
        "artifacts",
    ];
    let mut matches = Vec::new();
    for rel in &dirs {
        collect_apks_from_dir(&work_dir.join(rel), &mut matches);
    }
    matches
}

fn collect_apks_from_dir(dir: &Path, matches: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_apks_from_dir(&path, matches);
        } else if path.extension().and_then(|e| e.to_str()) == Some("apk") {
            matches.push(path);
        }
    }
}
