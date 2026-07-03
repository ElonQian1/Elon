pub(crate) fn env_flag(name: &str) -> Option<bool> {
    let value = std::env::var(name).ok()?.trim().to_ascii_lowercase();
    Some(matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

pub(crate) fn node_agent_env_file_path() -> Option<std::path::PathBuf> {
    std::env::current_exe().ok().and_then(|path| {
        path.parent()
            .map(|dir| dir.join("_internal").join("node-agent.env"))
    })
}
