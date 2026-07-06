// server/src/node_client_launcher/command.rs

use std::{
    ffi::OsStr,
    io,
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
const SW_SHOWNORMAL: i32 = 1;

#[cfg(windows)]
#[link(name = "shell32")]
unsafe extern "system" {
    fn ShellExecuteW(
        hwnd: *mut std::ffi::c_void,
        lpOperation: *const u16,
        lpFile: *const u16,
        lpParameters: *const u16,
        lpDirectory: *const u16,
        nShowCmd: i32,
    ) -> isize;
}

#[cfg(windows)]
pub(crate) fn open_url(url: &str) -> io::Result<()> {
    let url = url.trim();
    if !is_http_url(url) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("refusing to open non-http URL: {url}"),
        ));
    }
    shell_execute_open(url)
}

#[cfg(windows)]
pub(crate) fn ps_single_quote(value: &str) -> String {
    value.replace('\'', "''")
}

#[cfg(windows)]
fn shell_execute_open(target: &str) -> io::Result<()> {
    let operation = wide_null("open");
    let target = wide_null(target);
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            target.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    if result > 32 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "ShellExecuteW(open) failed with code {result}"
        )))
    }
}

#[cfg(windows)]
fn wide_null(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn is_http_url(value: &str) -> bool {
    let value = value.trim_start().to_ascii_lowercase();
    value.starts_with("http://") || value.starts_with("https://")
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
#[path = "command_tests.rs"]
mod tests;
