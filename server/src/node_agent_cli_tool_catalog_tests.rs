use super::*;
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn catalog_marks_only_core_small_tool_for_auto_install() {
    let auto_tools = codex_tool_catalog()
        .iter()
        .filter(|spec| spec.install_policy == InstallPolicy::AutoSmall)
        .map(|spec| spec.id)
        .collect::<Vec<_>>();

    assert_eq!(auto_tools, vec!["rg"]);
}

#[test]
fn core_tool_prefers_codex_program_sibling() {
    let root = unique_temp_dir("codex-tool-sibling");
    fs::create_dir_all(&root).unwrap();
    let codex = root.join(executable_name("codex"));
    let rg = root.join(executable_name("rg"));
    fs::write(&codex, b"").unwrap();
    fs::write(&rg, b"").unwrap();

    let resolved = resolve_codex_tools(Some(&codex));
    let first = resolved.first().expect("rg should resolve");

    assert_eq!(first.spec.id, "rg");
    assert_eq!(first.path, rg);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn optional_missing_tools_do_not_add_env_vars() {
    let root = unique_temp_dir("codex-no-tools");
    fs::create_dir_all(&root).unwrap();
    let codex = root.join(executable_name("codex"));
    fs::write(&codex, b"").unwrap();

    let tools = resolve_codex_tools_with_candidates(Some(&codex), &|_| Vec::new(), false);
    let envs = codex_child_env_overrides_from_tools(tools, Some(OsString::new()));

    assert!(!envs.iter().any(|(key, _)| key == "ELON_CODEX_JQ_PATH"));
    assert!(!envs.iter().any(|(key, _)| key == "ELON_CODEX_7Z_PATH"));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn path_merge_deduplicates_existing_entries() {
    let root = unique_temp_dir("codex-tool-path");
    let first = root.join("first");
    let second = root.join("second");
    fs::create_dir_all(&first).unwrap();
    fs::create_dir_all(&second).unwrap();
    let current = env::join_paths([second.clone()]).unwrap();

    let merged = prepend_dirs_to_path(vec![first.clone(), second.clone()], Some(current))
        .expect("merged PATH");
    let parts = env::split_paths(&OsString::from(merged)).collect::<Vec<_>>();

    assert_eq!(parts[0], first);
    assert_eq!(
        parts
            .iter()
            .filter(|path| path_key(path) == path_key(&second))
            .count(),
        1
    );
    let _ = fs::remove_dir_all(root);
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    env::temp_dir().join(format!("elon-{label}-{}-{nanos}", std::process::id()))
}
