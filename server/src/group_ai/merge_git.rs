use std::{
    path::Path,
    process::{Command, Stdio},
};

#[derive(Debug)]
pub(super) struct CommandOutcome {
    pub success: bool,
    pub detail: String,
    pub output: Vec<String>,
}

pub(super) fn run_git(path: &Path, args: &[&str]) -> CommandOutcome {
    match Command::new("git").arg("-C").arg(path).args(args).output() {
        Ok(output) => CommandOutcome {
            success: output.status.success(),
            detail: format!("git {}", args.join(" ")),
            output: output_lines(&output.stdout, &output.stderr),
        },
        Err(error) => CommandOutcome {
            success: false,
            detail: format!("git 启动失败: {error}"),
            output: Vec::new(),
        },
    }
}

pub(super) fn git_lines(path: &Path, args: &[&str]) -> Result<Vec<String>, String> {
    let outcome = run_git(path, args);
    if outcome.success {
        Ok(outcome.output)
    } else {
        Err(outcome.detail)
    }
}

pub(super) fn run_shell(path: &Path, command: &str) -> CommandOutcome {
    let command = command.chars().take(240).collect::<String>();
    let output = if cfg!(windows) {
        Command::new("cmd")
            .arg("/C")
            .arg(&command)
            .current_dir(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    } else {
        Command::new("sh")
            .arg("-lc")
            .arg(&command)
            .current_dir(path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
    };
    match output {
        Ok(output) => CommandOutcome {
            success: output.status.success(),
            detail: format!("exit={}", output.status.code().unwrap_or(-1)),
            output: output_lines(&output.stdout, &output.stderr),
        },
        Err(error) => CommandOutcome {
            success: false,
            detail: format!("命令启动失败: {error}"),
            output: Vec::new(),
        },
    }
}

pub(super) fn cleanup_merge_state(path: &Path) {
    let _ = run_git(path, &["merge", "--abort"]);
    let _ = run_git(path, &["reset", "--merge"]);
}

pub(super) fn display(path: &Path) -> String {
    path.display().to_string()
}

fn output_lines(stdout: &[u8], stderr: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stdout)
        .lines()
        .chain(String::from_utf8_lossy(stderr).lines())
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .take(40)
        .map(|line| line.chars().take(240).collect())
        .collect()
}
