//! Pure validation policy shared by small contract tests and future node/Desktop adapters.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceClass {
    Light,
    Heavy,
}

/// The PowerShell orchestrator consumes this same checked-in policy. Keeping the
/// mirror behind an include makes a missing/renamed production contract a compile error.
pub const PRODUCTION_POLICY_JSON: &str = include_str!("../../../scripts/validation/policy.json");

pub fn normalize_command(args: &[impl AsRef<str>]) -> String {
    args.iter()
        .map(|arg| arg.as_ref().trim().replace('\\', "/"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn classify_cargo(args: &[impl AsRef<str>]) -> ResourceClass {
    let values = args.iter().map(AsRef::as_ref).collect::<Vec<_>>();
    let verb = values.first().copied().unwrap_or_default();
    if matches!(verb, "test" | "build" | "bench" | "install" | "rustc")
        || values
            .iter()
            .any(|arg| matches!(*arg, "--release" | "--all-targets"))
    {
        ResourceClass::Heavy
    } else {
        ResourceClass::Light
    }
}

pub fn agent_validation_disables_incremental(domain: &str, release: bool) -> bool {
    release || domain == "validation" || domain.contains("agent-validation")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn policy_values(field: &str) -> BTreeSet<String> {
        serde_json::from_str::<serde_json::Value>(PRODUCTION_POLICY_JSON)
            .expect("production validation policy must be valid JSON")
            .get(field)
            .and_then(serde_json::Value::as_array)
            .unwrap_or_else(|| {
                panic!("production validation policy field {field} must be an array")
            })
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("policy values must be strings")
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn normalization_is_worktree_path_separator_independent() {
        assert_eq!(
            normalize_command(&["check", "server\\Cargo.toml"]),
            "check\nserver/Cargo.toml"
        );
    }

    #[test]
    fn checks_are_light_but_linking_and_tests_are_heavy() {
        assert_eq!(classify_cargo(&["check"]), ResourceClass::Light);
        assert_eq!(classify_cargo(&["clippy"]), ResourceClass::Light);
        assert_eq!(classify_cargo(&["test", "filter"]), ResourceClass::Heavy);
        assert_eq!(
            classify_cargo(&["check", "--all-targets"]),
            ResourceClass::Heavy
        );
    }

    #[test]
    fn only_agent_validation_or_release_disables_incremental() {
        assert!(!agent_validation_disables_incremental(
            "dev-windows-msvc",
            false
        ));
        assert!(agent_validation_disables_incremental(
            "agent-validation",
            false
        ));
        assert!(agent_validation_disables_incremental(
            "dev-windows-msvc",
            true
        ));
    }

    #[test]
    fn rust_contract_matches_the_production_consumed_policy() {
        let rust_heavy_verbs =
            BTreeSet::from_iter(["test", "build", "bench", "install", "rustc"].map(str::to_owned));
        let rust_heavy_flags =
            BTreeSet::from_iter(["--release", "--all-targets"].map(str::to_owned));
        let rust_agent_domains =
            BTreeSet::from_iter(["validation", "agent-validation"].map(str::to_owned));
        assert_eq!(policy_values("heavy_verbs"), rust_heavy_verbs);
        assert_eq!(policy_values("heavy_flags"), rust_heavy_flags);
        assert_eq!(
            policy_values("agent_validation_domains"),
            rust_agent_domains
        );
        for verb in &rust_heavy_verbs {
            assert_eq!(classify_cargo(&[verb.as_str()]), ResourceClass::Heavy);
        }
        for flag in &rust_heavy_flags {
            assert_eq!(
                classify_cargo(&["check", flag.as_str()]),
                ResourceClass::Heavy
            );
        }
        for domain in &rust_agent_domains {
            assert!(agent_validation_disables_incremental(
                domain.as_str(),
                false
            ));
        }
    }
}
