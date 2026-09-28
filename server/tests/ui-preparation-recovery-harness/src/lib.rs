//! Executes production ownership, preparation and generation regressions without
//! compiling unrelated node test modules. External ADB/build calls are disabled;
//! this harness cannot establish native rendering or release acceptance.
#![allow(dead_code, unused_imports)]

#[path = "../../../src/git_command_error.rs"]
mod git_command_error;
mod node_agent_android_live;
#[path = "../../../src/node_agent_atomic_file.rs"]
mod node_agent_atomic_file;

mod node_agent_exec {
    pub(crate) fn hide_tokio_command_window(_: &mut tokio::process::Command) {
        panic!("harness must not launch external device/build processes");
    }
}
mod node_agent_cli_runtime_policy {
    pub(crate) fn terminate_process_tree(_: Option<u32>) -> bool {
        panic!("external build disabled in harness")
    }
}
mod node_agent_android_inspector {
    pub(crate) mod types {
        pub(crate) struct AndroidDevice {
            pub serial: String,
            pub state: String,
        }
    }
    pub(crate) mod adb_wireless {
        pub(crate) async fn list_device_inventory(
        ) -> anyhow::Result<Vec<super::types::AndroidDevice>> {
            anyhow::bail!("ADB disabled in harness")
        }
    }
    pub(crate) mod adb_capture {
        pub(crate) async fn visual_unavailable_reason(_: &str) -> anyhow::Result<Option<String>> {
            anyhow::bail!("ADB disabled in harness")
        }
    }
}
