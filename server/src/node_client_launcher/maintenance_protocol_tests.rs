    use super::{
        protocol_start_requested, target_from_args, target_path, MaintenanceProtocolTarget,
    };
    use std::path::Path;

    #[test]
    fn protocol_urls_map_to_fixed_maintenance_targets() {
        assert_eq!(
            target_from_args(&["elon-node://logs".to_string()]),
            Some(MaintenanceProtocolTarget::Logs)
        );
        assert_eq!(
            target_from_args(&["elon-node://launcher-logs".to_string()]),
            Some(MaintenanceProtocolTarget::LauncherLogs)
        );
        assert_eq!(
            target_from_args(&["elon-node://maintenance/task-journal".to_string()]),
            Some(MaintenanceProtocolTarget::TaskJournal)
        );
        assert_eq!(
            target_from_args(&["elon-node://install-dir".to_string()]),
            Some(MaintenanceProtocolTarget::InstallDir)
        );
        assert_eq!(
            target_from_args(&["elon-node://c:/windows".to_string()]),
            None
        );
    }

    #[test]
    fn cli_flags_map_to_same_fixed_maintenance_targets() {
        assert_eq!(
            target_from_args(&["--open-logs".to_string()]),
            Some(MaintenanceProtocolTarget::Logs)
        );
        assert_eq!(
            target_from_args(&["--diagnostics-dir".to_string()]),
            Some(MaintenanceProtocolTarget::DiagnosticsDir)
        );
        assert!(protocol_start_requested(&["elon-node://open".to_string()]));
    }

    #[test]
    fn target_paths_stay_under_known_roots() {
        let install = Path::new(r"C:\Users\ELon\AppData\Local\ElonNode");

        assert!(
            target_path(MaintenanceProtocolTarget::LauncherLogs, install)
                .ends_with(r"_internal\logs")
        );
        assert_eq!(
            target_path(MaintenanceProtocolTarget::InstallDir, install),
            install
        );
    }

    #[cfg(windows)]
    #[test]
    fn open_path_command_uses_explorer_without_shell_wrappers() {
        let command = super::open_path_command(Path::new(r"C:\Users\ELon\AppData\Local\ElonNode"));
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        assert_eq!(command.get_program().to_string_lossy(), "explorer.exe");
        assert_eq!(args, vec![r"C:\Users\ELon\AppData\Local\ElonNode"]);
    }
