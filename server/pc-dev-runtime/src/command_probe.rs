use std::{
    io,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

#[cfg(windows)]
use std::ffi::OsStr;

pub fn command_candidates(name: &str) -> Vec<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return Vec::new();
    }

    let direct = PathBuf::from(name);
    if has_path_separator(name) || direct.is_absolute() {
        return direct.exists().then_some(direct).into_iter().collect();
    }

    let mut found = Vec::new();
    for dir in command_search_dirs() {
        for file_name in command_file_names(name) {
            let path = dir.join(file_name);
            if path.exists() && !contains_path(&found, &path) {
                found.push(path);
            }
        }
    }
    found
}

pub fn command_path(name: &str) -> Option<PathBuf> {
    command_candidates(name).into_iter().next()
}

pub fn command_output(name: &str, args: &[&str], cwd: Option<&Path>) -> io::Result<Output> {
    let program = command_path(name).unwrap_or_else(|| PathBuf::from(name));
    let mut command = command_from_path(&program);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.stdin(Stdio::null());
    command.output()
}

pub fn command_from_path(program: &Path) -> Command {
    #[cfg(windows)]
    {
        if is_windows_script(program) {
            let mut command = Command::new("cmd");
            command.args(["/D", "/S", "/C"]).arg(program);
            apply_hidden_window(&mut command);
            return command;
        }
    }

    let mut command = Command::new(program);
    apply_hidden_window(&mut command);
    command
}

pub fn apply_hidden_window(_command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        _command.creation_flags(CREATE_NO_WINDOW);
    }
}

fn command_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            push_unique_dir(&mut dirs, dir);
        }
    }

    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        let localappdata = std::env::var("LOCALAPPDATA").unwrap_or_default();
        let userprofile = std::env::var("USERPROFILE").unwrap_or_default();
        let program_files =
            std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".to_string());
        let program_files_x86 = std::env::var("ProgramFiles(x86)")
            .unwrap_or_else(|_| "C:\\Program Files (x86)".to_string());

        for dir in [
            PathBuf::from(&appdata).join("npm"),
            PathBuf::from(&localappdata).join("Yarn").join("bin"),
            PathBuf::from(&appdata).join("pnpm"),
            PathBuf::from(&userprofile).join(".volta").join("bin"),
            PathBuf::from(&appdata).join("nvm"),
            PathBuf::from(&userprofile).join("scoop").join("shims"),
            PathBuf::from(&program_files).join("GitHub CLI"),
            PathBuf::from(&program_files_x86).join("GitHub CLI"),
            PathBuf::from(&program_files).join("Git").join("cmd"),
            PathBuf::from(&program_files).join("Git").join("bin"),
            PathBuf::from(&program_files).join("nodejs"),
            PathBuf::from(&program_files).join("Ollama"),
        ] {
            push_unique_dir(&mut dirs, dir);
        }
    }

    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        for dir in [
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from(&home).join(".npm-global").join("bin"),
            PathBuf::from(&home).join(".local").join("bin"),
            PathBuf::from(&home).join(".yarn").join("bin"),
            PathBuf::from(&home).join(".volta").join("bin"),
        ] {
            push_unique_dir(&mut dirs, dir);
        }
    }

    dirs
}

fn command_file_names(name: &str) -> Vec<String> {
    let path = Path::new(name);
    if path.extension().is_some() {
        return vec![name.to_string()];
    }

    #[cfg(windows)]
    {
        [".cmd", ".exe", ".bat", ".com", ""]
            .into_iter()
            .map(|ext| format!("{name}{ext}"))
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![name.to_string()]
    }
}

fn push_unique_dir(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if dir.as_os_str().is_empty() || contains_path(dirs, &dir) {
        return;
    }
    dirs.push(dir);
}

fn contains_path(paths: &[PathBuf], candidate: &Path) -> bool {
    let candidate_key = path_key(candidate);
    paths.iter().any(|path| path_key(path) == candidate_key)
}

fn path_key(path: &Path) -> String {
    let value = path.to_string_lossy().to_string();
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

fn has_path_separator(value: &str) -> bool {
    value.contains('/') || value.contains('\\')
}

#[cfg(windows)]
fn is_windows_script(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|ext| matches!(ext.to_ascii_lowercase().as_str(), "cmd" | "bat"))
        .unwrap_or(false)
}
