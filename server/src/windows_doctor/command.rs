use serde_json::{json, Value};
use std::process::Command;

pub(super) const MAX_COMMAND_OUTPUT: usize = 12_000;

pub(super) fn powershell_check(script: &str) -> Value {
    let full_script = format!(
        "$ErrorActionPreference='SilentlyContinue'; [Console]::OutputEncoding=[Text.UTF8Encoding]::UTF8; {script}"
    );
    run_command(
        "powershell",
        &[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &full_script,
        ],
        MAX_COMMAND_OUTPUT,
    )
}

pub(super) fn run_command<S: AsRef<str>>(program: &str, args: &[S], max_output: usize) -> Value {
    let args_for_json = args
        .iter()
        .map(|arg| arg.as_ref().to_string())
        .collect::<Vec<_>>();
    let mut command = Command::new(program);
    command.args(args.iter().map(|arg| arg.as_ref()));
    hide_command_window(&mut command);
    match command.output() {
        Ok(output) => {
            let stdout = truncate_lossy(&output.stdout, max_output);
            let stderr = truncate_lossy(&output.stderr, max_output / 2);
            json!({
                "program": program,
                "args": args_for_json,
                "success": output.status.success(),
                "code": output.status.code(),
                "stdout": stdout,
                "stderr": stderr,
            })
        }
        Err(error) => json!({
            "program": program,
            "args": args_for_json,
            "success": false,
            "error": error.to_string(),
        }),
    }
}

fn truncate_lossy(bytes: &[u8], max_chars: usize) -> String {
    let value = String::from_utf8_lossy(bytes);
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }
    let head = value.chars().take(max_chars).collect::<String>();
    format!("{head}\n... truncated {count} chars ...")
}

fn hide_command_window(_command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        _command.creation_flags(CREATE_NO_WINDOW);
    }
}
