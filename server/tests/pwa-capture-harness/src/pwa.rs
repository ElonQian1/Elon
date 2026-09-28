//! Production PWA tests with real isolated browsers; no Android or node restart.
#![allow(dead_code, unused_imports)]
#![recursion_limit = "256"]

mod node_agent_cli_redaction {
    pub(crate) use elon_pwa_capture_harness::redact_text;
}

#[path = "../../../src/node_agent_pwa_runtime/mod.rs"]
mod node_agent_pwa_runtime;

mod capture_contract;
#[path = "../../../src/node_agent_android_live/semantic_parity/capture_result.rs"]
mod semantic_capture_result;

mod node_agent_exec {
    pub(crate) fn hide_tokio_command_window(command: &mut tokio::process::Command) {
        #[cfg(windows)]
        command.creation_flags(0x0800_0000);
    }
}
