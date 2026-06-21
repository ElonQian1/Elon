// server/src/node_client_launcher/command.rs

use std::{
    ffi::OsStr,
    process::{Child, Command, ExitStatus, Output, Stdio},
};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

pub(crate) fn silent_command<S: AsRef<OsStr>>(program: S) -> Command {
    let mut command = Command::new(program);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

pub(crate) fn spawn_hidden(command: &mut Command) -> std::io::Result<Child> {
    apply_hidden_window(command);
    command.spawn()
}

pub(crate) fn status_hidden(command: &mut Command) -> std::io::Result<ExitStatus> {
    apply_hidden_window(command);
    command.status()
}

pub(crate) fn output_hidden(command: &mut Command) -> std::io::Result<Output> {
    apply_hidden_window(command);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    command.output()
}

#[cfg(windows)]
pub(crate) fn powershell_hidden_command(script: &str) -> Command {
    let mut command = silent_command("powershell");
    command.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-WindowStyle",
        "Hidden",
        "-Command",
        script,
    ]);
    command
}

#[cfg(windows)]
pub(crate) fn cmd_hidden_command(command_line: &str) -> Command {
    let mut command = silent_command("cmd");
    command.args(["/D", "/S", "/C", command_line]);
    command
}

#[cfg(windows)]
pub(crate) fn open_url_command(url: &str) -> Command {
    // 打开网页不再绕 cmd /C start，避免 shell 窗口闪现和参数二次解析。
    let mut command = silent_command("explorer.exe");
    command.arg(url);
    command
}

#[cfg(windows)]
pub(crate) fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(windows)]
fn apply_hidden_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    // 统一给 Windows 子进程加隐藏窗口标记；新进程组能减少 shell 子进程继承控制台。
    command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(windows))]
fn apply_hidden_window(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn command_args(command: &Command) -> Vec<String> {
        command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn silent_command_uses_requested_program() {
        let command = silent_command("test-program");

        assert_eq!(command.get_program().to_string_lossy(), "test-program");
        assert!(command_args(&command).is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn powershell_command_is_hidden_and_non_interactive() {
        let command = powershell_hidden_command("Write-Output ok");
        let args = command_args(&command);

        assert_eq!(command.get_program().to_string_lossy(), "powershell");
        assert!(args
            .windows(2)
            .any(|pair| pair == ["-WindowStyle", "Hidden"]));
        assert!(args
            .windows(2)
            .any(|pair| pair == ["-Command", "Write-Output ok"]));
    }

    #[cfg(windows)]
    #[test]
    fn open_url_command_avoids_cmd_and_powershell_shells() {
        let command = open_url_command("http://127.0.0.1:7799/?a=1&b=2");
        let args = command_args(&command);

        assert_eq!(command.get_program().to_string_lossy(), "explorer.exe");
        assert_eq!(args, vec!["http://127.0.0.1:7799/?a=1&b=2"]);
    }

    #[cfg(windows)]
    #[test]
    fn cmd_command_keeps_script_as_single_argument() {
        let command = cmd_hidden_command("timeout /t 1 /nobreak >nul");
        let args = command_args(&command);

        assert_eq!(command.get_program().to_string_lossy(), "cmd");
        assert_eq!(args, vec!["/D", "/S", "/C", "timeout /t 1 /nobreak >nul"]);
    }

    #[cfg(windows)]
    #[test]
    fn ps_single_quote_doubles_embedded_quotes() {
        assert_eq!(
            ps_single_quote("C:\\Program Files\\O'Hara"),
            "C:\\Program Files\\O''Hara"
        );
    }

    #[cfg(windows)]
    #[test]
    fn hidden_creation_flag_is_create_no_window() {
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);
        assert_eq!(CREATE_NEW_PROCESS_GROUP, 0x0000_0200);
    }

    #[cfg(windows)]
    #[test]
    fn output_hidden_captures_stdout() {
        let mut command = cmd_hidden_command("echo capture-ok");
        let output = output_hidden(&mut command).unwrap();

        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("capture-ok"));
    }
}
