    use super::{git_failure_message, git_spawn_context};
    use std::path::Path;
    use std::process::{ExitStatus, Output};

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
