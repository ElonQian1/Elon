use std::{env, path::Path};

#[path = "node_agent_cli_tool_catalog.rs"]
mod tool_catalog;

pub(crate) fn codex_child_env_overrides(codex_program: &str) -> Vec<(String, String)> {
    tool_catalog::codex_child_env_overrides(Path::new(codex_program), env::var_os("PATH"))
}
