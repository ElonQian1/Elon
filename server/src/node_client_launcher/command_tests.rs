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

    #[test]
    #[cfg(windows)]
    fn http_url_validator_rejects_local_paths_and_empty_targets() {
        assert!(is_http_url("http://127.0.0.1:7799/?a=1&b=2"));
        assert!(is_http_url(" https://example.com/pc"));
        assert!(!is_http_url(""));
        assert!(!is_http_url(r"C:\Users\Administrator\Documents"));
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
