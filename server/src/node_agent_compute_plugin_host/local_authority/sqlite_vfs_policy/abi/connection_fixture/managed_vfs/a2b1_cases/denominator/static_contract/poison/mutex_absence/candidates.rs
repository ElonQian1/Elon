use std::collections::BTreeMap;

pub(super) struct SourceView {
    pub(super) path: &'static str,
    pub(super) source: &'static str,
}

const TOKENS: &[&str] = &[
    "panic!(",
    "assert!(",
    "assert_eq!(",
    "assert_ne!(",
    "debug_assert!(",
    "debug_assert_eq!(",
    "debug_assert_ne!(",
    ".unwrap(",
    ".expect(",
    "unreachable!(",
    "todo!(",
    "unimplemented!(",
];

/// Exact lexical inventory over all frozen units. An omitted file/token pair means zero.
const REVIEWED: &[(&str, &str, usize, &str)] = &[
    ("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_abi/file_state.rs", "assert!(", 2, "cfg(test) raw-state constructors"),
    ("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs", ".expect(", 3, "one linear lease take precedes lock_routes; two sites are cfg(test)"),
    ("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner/lifecycle.rs", ".expect(", 12, "all sites are cfg(test) or cfg(all(test, windows))"),
    ("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs", "assert!(", 1, "substring of reviewed debug_assert"),
    ("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs", "debug_assert!(", 1, "used-token check excludes replacement under the same guard"),
    ("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs", ".expect(", 2, "exact_entry and remove are adjacent under the same guard"),
    ("src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs", ".expect(", 1, "exact_entry and remove are adjacent under the same guard"),
    ("src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs", "assert!(", 4, "cfg(test) merge_poison module"),
    ("src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs", "assert_eq!(", 2, "cfg(test) merge_poison module"),
    ("src/node_agent_managed_fs/sqlite_namespace_shm/coordinator.rs", ".expect(", 2, "cfg(test) merge_poison module"),
    ("src/node_agent_managed_fs/sqlite_namespace_shm/unmap.rs", ".expect(", 1, "cfg(all(test, windows)) prestate injector"),
    ("src/node_agent_managed_fs/sqlite_namespace_shm/unmap.rs", "unreachable!(", 1, "cfg(all(test, windows)) prestate injector"),
    ("src/node_agent_managed_fs/windows_sqlite_locking.rs", "assert!(", 1, "cfg(all(test, windows)) helper/test"),
    ("src/node_agent_managed_fs/windows_sqlite_locking.rs", "assert_eq!(", 5, "cfg(all(test, windows)) tests"),
    ("src/node_agent_managed_fs/windows_sqlite_locking.rs", ".expect(", 6, "cfg(all(test, windows)) helper/tests"),
];

pub(super) fn validate(sources: &[SourceView]) -> Result<(), String> {
    validate_explicit_inventory(sources)?;
    validate_index_candidates(sources)
}

fn validate_explicit_inventory(sources: &[SourceView]) -> Result<(), String> {
    let mut observed = BTreeMap::new();
    for unit in sources {
        for token in TOKENS {
            let count = unit.source.matches(token).count();
            if count != 0 {
                observed.insert((unit.path, *token), count);
            }
        }
    }
    let mut expected = BTreeMap::new();
    for (path, token, count, proof) in REVIEWED {
        if proof.is_empty() || !TOKENS.contains(token) {
            return Err(format!("invalid unwind review: {path} {token}"));
        }
        if expected.insert((*path, *token), *count).is_some() {
            return Err(format!("duplicate unwind review: {path} {token}"));
        }
    }
    if observed != expected {
        return Err(format!(
            "explicit unwind set changed: expected {expected:?}, observed {observed:?}"
        ));
    }
    Ok(())
}

fn validate_index_candidates(sources: &[SourceView]) -> Result<(), String> {
    let owner = source(
        sources,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner.rs",
    )?;
    occurrences(
        "owner invariant",
        owner,
        "validated route must remain present under exclusive owner access",
        2,
    )?;
    let lifecycle = source(sources, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/owner/lifecycle.rs")?;
    occurrences(
        "owner lifecycle invariant",
        lifecycle,
        "validated route must remain present under exclusive owner access",
        1,
    )?;
    let process = source(sources, "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/process_owner.rs")?;
    occurrences(
        "test poison module gate",
        process,
        "#[cfg(test)]\nmod tests;",
        1,
    )?;
    let state = source(
        sources,
        "src/node_agent_compute_plugin_host/local_authority/sqlite_vfs_policy/registry/state.rs",
    )?;
    occurrences("bounded owner index", state, "self.sidecar_leases[slot]", 2)?;
    occurrences("owner index source", state, ".position(Option::is_none)", 1)?;
    let locking = source(
        sources,
        "src/node_agent_managed_fs/sqlite_namespace_shm/locking.rs",
    )?;
    occurrences(
        "bounded coordinator index",
        locking,
        "exclusive_ranges[usize::from(request.first())]",
        3,
    )?;
    let types = source(
        sources,
        "src/node_agent_managed_fs/sqlite_namespace_shm/types.rs",
    )?;
    occurrences(
        "coordinator index bound",
        types,
        "if end > SHM_LOCK_COUNT",
        1,
    )
}

fn source<'a>(sources: &'a [SourceView], path: &str) -> Result<&'a str, String> {
    sources
        .iter()
        .find(|unit| unit.path == path)
        .map(|unit| unit.source)
        .ok_or_else(|| format!("reviewed source missing: {path}"))
}

fn occurrences(label: &str, source: &str, needle: &str, expected: usize) -> Result<(), String> {
    let observed = source.matches(needle).count();
    if observed != expected {
        return Err(format!(
            "candidate drift for {label}: {needle:?} expected {expected}, observed {observed}"
        ));
    }
    Ok(())
}
