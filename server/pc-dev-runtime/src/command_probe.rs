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
    sort_command_candidates(name, &mut found);
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
            command.args(["/D", "/S", "/C", "call"]).arg(program);
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

        let codex_bin_root = PathBuf::from(&localappdata)
            .join("OpenAI")
            .join("Codex")
            .join("bin");
        if let Ok(entries) = std::fs::read_dir(codex_bin_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    push_unique_dir(&mut dirs, path);
                }
            }
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

fn sort_command_candidates(name: &str, candidates: &mut [PathBuf]) {
    if !name.eq_ignore_ascii_case("codex") {
        return;
    }
    candidates.sort_by_key(|path| codex_candidate_rank(path));
}

fn codex_candidate_rank(path: &Path) -> u8 {
    let lower = path.to_string_lossy().to_ascii_lowercase();
    let is_windows_apps = lower.contains("\\windowsapps\\");
    let is_codex_desktop_resource = is_windows_apps
        && lower.contains("\\openai.codex_")
        && lower.contains("\\app\\resources\\");
    let is_openai_codex_runtime =
        lower.contains("\\appdata\\local\\openai\\codex\\bin\\") && lower.ends_with("\\codex.exe");
    let is_script = lower.ends_with("\\codex.cmd") || lower.ends_with("\\codex.bat");

    if is_script && !is_windows_apps {
        0
    } else if is_openai_codex_runtime {
        1
    } else if !is_windows_apps {
        2
    } else if is_codex_desktop_resource {
        4
    } else {
        3
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn command_args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[cfg(windows)]
    #[test]
    fn windows_script_commands_use_call_before_script_path() {
        let command = command_from_path(Path::new(r"C:\Program Files\nodejs\npm.cmd"));
        let args = command_args(&command);

        assert_eq!(command.get_program().to_string_lossy(), "cmd");
        assert_eq!(
            args,
            vec![
                "/D".to_string(),
                "/S".to_string(),
                "/C".to_string(),
                "call".to_string(),
                r"C:\Program Files\nodejs\npm.cmd".to_string()
            ]
        );
    }

    #[cfg(windows)]
    #[test]
    fn codex_candidates_prefer_runtime_cli_over_windowsapps_desktop_resource() {
        let mut candidates = vec![
            PathBuf::from(
                r"C:\Program Files\WindowsApps\OpenAI.Codex_1.0.0.0_x64__abc\app\resources\codex.exe",
            ),
            PathBuf::from(r"C:\Users\alice\AppData\Local\OpenAI\Codex\bin\hash123\codex.exe"),
            PathBuf::from(r"C:\Users\alice\AppData\Roaming\npm\codex.cmd"),
        ];

        sort_command_candidates("codex", &mut candidates);

        assert_eq!(
            candidates[0],
            PathBuf::from(r"C:\Users\alice\AppData\Roaming\npm\codex.cmd")
        );
        assert_eq!(
            candidates[1],
            PathBuf::from(r"C:\Users\alice\AppData\Local\OpenAI\Codex\bin\hash123\codex.exe")
        );
        assert!(candidates[2]
            .to_string_lossy()
            .contains(r"WindowsApps\OpenAI.Codex"));
    }
}
