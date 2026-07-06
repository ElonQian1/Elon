use std::path::Path;
use std::process::{Command, Output};

pub(crate) fn git_command() -> Command {
    let mut command = Command::new("git");
    configure_git_command(&mut command);
    command
}

pub(crate) fn git_spawn_context(args: &[&str]) -> String {
    format!("git {}", display_args(args))
}

pub(crate) fn git_failure_message(cwd: &Path, args: &[&str], output: &Output) -> String {
    let exit = output
        .status
        .code()
        .map(|code| code.to_string())
        .unwrap_or_else(|| "terminated".to_string());
    let stdout = output_text(&output.stdout);
    let stderr = output_text(&output.stderr);

    format!(
        "git command failed (cwd={}; command=git {}; exit={}; stderr={}; stdout={})",
        cwd.display(),
        display_args(args),
        exit,
        empty_label(&stderr),
        empty_label(&stdout)
    )
}

fn display_args(args: &[&str]) -> String {
    args.iter()
        .map(|arg| quote_arg(&redact_credentials(arg)))
        .collect::<Vec<_>>()
        .join(" ")
}

fn output_text(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes).trim().to_string();
    if text.chars().count() <= 1200 {
        return text;
    }
    let mut shortened = text.chars().take(1200).collect::<String>();
    shortened.push_str("...");
    shortened
}

fn empty_label(value: &str) -> String {
    if value.is_empty() {
        "<empty>".to_string()
    } else {
        value.to_string()
    }
}

fn quote_arg(value: &str) -> String {
    if value
        .chars()
        .any(|ch| ch.is_whitespace() || matches!(ch, '"' | '\''))
    {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

fn redact_credentials(value: &str) -> String {
    let Some(scheme_pos) = value.find("://") else {
        return value.to_string();
    };
    let credentials_start = scheme_pos + 3;
    let Some(at_rel) = value[credentials_start..].find('@') else {
        return value.to_string();
    };
    let at_pos = credentials_start + at_rel;
    let slash_pos = value[credentials_start..]
        .find('/')
        .map(|offset| credentials_start + offset);
    if matches!(slash_pos, Some(pos) if pos < at_pos) {
        return value.to_string();
    }

    format!("{}***{}", &value[..credentials_start], &value[at_pos..])
}

#[cfg(windows)]
fn configure_git_command(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn configure_git_command(_command: &mut Command) {}


#[cfg(test)]
#[path = "git_command_error_tests.rs"]
mod tests;
