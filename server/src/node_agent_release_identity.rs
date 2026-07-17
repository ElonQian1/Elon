pub(crate) fn current() -> String {
    compose(
        env!("CARGO_PKG_VERSION"),
        option_env!("ELON_NODE_AGENT_GIT_SHA"),
    )
}

fn compose(version: &str, git_sha: Option<&str>) -> String {
    let version = version.trim();
    match git_sha.map(str::trim).filter(|value| !value.is_empty()) {
        Some(git_sha) => format!("{version}+{git_sha}"),
        None => version.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::compose;

    #[test]
    fn development_build_keeps_package_version() {
        assert_eq!(compose("0.3.69", None), "0.3.69");
        assert_eq!(compose(" 0.3.69 ", Some("  ")), "0.3.69");
    }

    #[test]
    fn published_build_includes_exact_git_sha() {
        assert_eq!(
            compose("0.3.69", Some(" c97f4b6fd9c5 ")),
            "0.3.69+c97f4b6fd9c5"
        );
    }
}
