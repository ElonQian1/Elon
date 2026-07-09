use std::path::PathBuf;
use std::sync::OnceLock;

static CACHED_ADB_PATH: OnceLock<String> = OnceLock::new();

pub(crate) fn adb_path() -> String {
    CACHED_ADB_PATH.get_or_init(detect_adb_path).clone()
}

fn detect_adb_path() -> String {
    for candidate in adb_candidates() {
        if candidate.is_file() {
            return candidate.display().to_string();
        }
    }
    adb_executable_name().to_string()
}

fn adb_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("ELON_ADB_PATH").filter(|value| !value.is_empty()) {
        candidates.push(PathBuf::from(path));
    }
    for key in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(root) = std::env::var_os(key).filter(|value| !value.is_empty()) {
            candidates.push(
                PathBuf::from(root)
                    .join("platform-tools")
                    .join(adb_executable_name()),
            );
        }
    }
    if cfg!(windows) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local)
                    .join("Android")
                    .join("Sdk")
                    .join("platform-tools")
                    .join("adb.exe"),
            );
        }
        candidates.push(PathBuf::from(r"D:\Android\sdk\platform-tools\adb.exe"));
        candidates.push(PathBuf::from(r"C:\Android\sdk\platform-tools\adb.exe"));
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            candidates.push(path.join(adb_executable_name()));
        }
    }
    candidates
}

fn adb_executable_name() -> &'static str {
    if cfg!(windows) {
        "adb.exe"
    } else {
        "adb"
    }
}
