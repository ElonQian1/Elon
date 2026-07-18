use super::{git_command, git_failure_message, git_spawn_context};
use std::path::Path;
use std::process::{Command, ExitStatus, Output};

#[cfg(unix)]
fn status(code: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(code << 8)
}

#[cfg(windows)]
fn status(code: u32) -> ExitStatus {
    use std::os::windows::process::ExitStatusExt;
    ExitStatus::from_raw(code)
}

#[test]
fn failure_message_includes_command_cwd_and_streams() {
    let output = Output {
        status: status(128),
        stdout: b"stdout detail".to_vec(),
        stderr: b"stderr detail".to_vec(),
    };
    let message = git_failure_message(
        Path::new("C:/repo"),
        &["merge", "--no-edit", "feature branch"],
        &output,
    );

    assert!(message.contains("cwd=C:/repo"));
    assert!(message.contains("command=git merge --no-edit \"feature branch\""));
    assert!(message.contains("exit=128"));
    assert!(message.contains("stderr=stderr detail"));
    assert!(message.contains("stdout=stdout detail"));
}

#[test]
fn command_display_redacts_url_credentials() {
    let message = git_spawn_context(&[
        "remote",
        "set-url",
        "origin",
        "https://user:token@example.com/repo.git",
    ]);

    assert_eq!(
        message,
        "git remote set-url origin https://***@example.com/repo.git"
    );
}

#[test]
fn git_command_disables_credential_prompts() {
    let command = git_command();
    let env = command
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_string_lossy().into_owned(),
                value.map(|value| value.to_string_lossy().into_owned()),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();

    assert_eq!(env.get("GIT_TERMINAL_PROMPT"), Some(&Some("0".to_string())));
    assert_eq!(env.get("GCM_INTERACTIVE"), Some(&Some("Never".to_string())));
    assert_eq!(
        env.get("SSH_ASKPASS_REQUIRE"),
        Some(&Some("never".to_string()))
    );
}

#[test]
fn credential_challenge_fails_quickly_with_auditable_stderr() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind credential challenge server");
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(
                b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"elon-test\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    });

    let mut command = git_command();
    command
        .args(["ls-remote", &format!("http://{address}/private.git")])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().expect("spawn non-interactive git");
    let started = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if started.elapsed() > Duration::from_secs(8) {
            let _ = child.kill();
            panic!("credential challenge left background Git waiting for interaction");
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let output = child.wait_with_output().unwrap();
    assert!(!output.status.success());
    assert!(
        !output.stderr.is_empty(),
        "Git authentication failure must remain auditable on stderr"
    );
}

#[cfg(windows)]
#[test]
fn windows_git_launch_policy_has_no_console_and_closed_stdin() {
    const CHILD_ENV: &str = "ELON_TEST_BACKGROUND_GIT_CHILD";
    if std::env::var_os(CHILD_ENV).is_some() {
        let mut stdin = std::io::stdin();
        let mut content = Vec::new();
        std::io::Read::read_to_end(&mut stdin, &mut content).unwrap();
        assert!(content.is_empty(), "background Git stdin must be closed");
        assert!(
            unsafe { GetConsoleWindow() }.is_null(),
            "child inherited a console window"
        );
        return;
    }

    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "--exact",
            "git_command_error::tests::windows_git_launch_policy_has_no_console_and_closed_stdin",
            "--nocapture",
        ])
        .env(CHILD_ENV, "1");
    elon_pc_dev_runtime::configure_non_interactive_git_command(&mut child);
    let output = child.output().expect("launch background child");
    assert!(
        output.status.success(),
        "background child failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetConsoleWindow() -> *mut std::ffi::c_void;
}
