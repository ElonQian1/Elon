use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static SHUTDOWN_REQUESTED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
const BACKGROUND_ERROR_MODE: u32 =
    windows_sys::Win32::System::Diagnostics::Debug::SEM_FAILCRITICALERRORS
        | windows_sys::Win32::System::Diagnostics::Debug::SEM_NOGPFAULTERRORBOX
        | windows_sys::Win32::System::Diagnostics::Debug::SEM_NOOPENFILEERRORBOX;

/// Installs process-wide Windows behavior before the node or launcher starts any child process.
/// The error mode is inherited by Git, ADB and CLI children, so a late startup failure is returned
/// to the node instead of displaying an interactive system dialog during shutdown.
pub(crate) fn initialize_process_behavior() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
        use windows_sys::Win32::System::Diagnostics::Debug::{GetErrorMode, SetErrorMode};

        SetErrorMode(GetErrorMode() | BACKGROUND_ERROR_MODE);
        let _ = SetConsoleCtrlHandler(Some(console_control_handler), 1);
    }
}

pub(crate) fn shutdown_requested() -> bool {
    if SHUTDOWN_REQUESTED.load(Ordering::Acquire) {
        return true;
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_SHUTTINGDOWN};

        if unsafe { GetSystemMetrics(SM_SHUTTINGDOWN) } != 0 {
            SHUTDOWN_REQUESTED.store(true, Ordering::Release);
            return true;
        }
    }

    false
}

pub(crate) async fn wait_for_shutdown() {
    while !shutdown_requested() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(windows)]
unsafe extern "system" fn console_control_handler(control_type: u32) -> i32 {
    if terminal_control_event(control_type) {
        SHUTDOWN_REQUESTED.store(true, Ordering::Release);
        1
    } else {
        0
    }
}

#[cfg(windows)]
fn terminal_control_event(control_type: u32) -> bool {
    use windows_sys::Win32::System::Console::{
        CTRL_CLOSE_EVENT, CTRL_LOGOFF_EVENT, CTRL_SHUTDOWN_EVENT,
    };

    matches!(
        control_type,
        CTRL_CLOSE_EVENT | CTRL_LOGOFF_EVENT | CTRL_SHUTDOWN_EVENT
    )
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn close_logoff_and_shutdown_are_terminal_but_ctrl_c_is_not_claimed() {
        use super::terminal_control_event;
        use windows_sys::Win32::System::Console::{
            CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT, CTRL_C_EVENT, CTRL_LOGOFF_EVENT,
            CTRL_SHUTDOWN_EVENT,
        };

        assert!(terminal_control_event(CTRL_CLOSE_EVENT));
        assert!(terminal_control_event(CTRL_LOGOFF_EVENT));
        assert!(terminal_control_event(CTRL_SHUTDOWN_EVENT));
        assert!(!terminal_control_event(CTRL_C_EVENT));
        assert!(!terminal_control_event(CTRL_BREAK_EVENT));
    }

    #[cfg(windows)]
    #[test]
    fn background_error_mode_suppresses_child_process_dialogs() {
        use windows_sys::Win32::System::Diagnostics::Debug::{
            SEM_FAILCRITICALERRORS, SEM_NOGPFAULTERRORBOX, SEM_NOOPENFILEERRORBOX,
        };

        assert_eq!(
            super::BACKGROUND_ERROR_MODE,
            SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX | SEM_NOOPENFILEERRORBOX
        );
    }
}
