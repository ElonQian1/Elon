use std::path::{Path, PathBuf};

const DIRECT_MODULES: &[&str] = &[
    "server", "backend", "api", "app", "cmd", "web", "frontend", "client", "android",
];
const GROUP_MODULE_ROOTS: &[&str] = &["apps", "packages", "crates", "services", "tools"];
const MAX_GROUP_CHILDREN_PER_ROOT: usize = 12;
const MAX_WORKSPACE_MODULES: usize = 32;

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceModuleCandidate {
    pub(crate) module: String,
    pub(crate) path: PathBuf,
}

pub(crate) fn workspace_module_candidates(project_root: &Path) -> Vec<WorkspaceModuleCandidate> {
    let mut modules = Vec::new();
    for module in DIRECT_MODULES {
        push_module_candidate(&mut modules, module.to_string(), project_root.join(module));
    }

    for root in GROUP_MODULE_ROOTS {
        let group_root = project_root.join(root);
        if !group_root.is_dir() {
            continue;
        }
        for name in sorted_child_dirs(&group_root)
            .into_iter()
            .take(MAX_GROUP_CHILDREN_PER_ROOT)
        {
            push_module_candidate(
                &mut modules,
                format!("{root}/{name}"),
                group_root.join(&name),
            );
            if modules.len() >= MAX_WORKSPACE_MODULES {
                return modules;
            }
        }
    }

    modules
}

fn push_module_candidate(
    modules: &mut Vec<WorkspaceModuleCandidate>,
    module: String,
    path: PathBuf,
) {
    if !path.is_dir() || modules.iter().any(|candidate| candidate.module == module) {
        return;
    }
    modules.push(WorkspaceModuleCandidate { module, path });
}

fn sorted_child_dirs(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if !file_type.is_dir() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            allowed_group_child_name(&name).then_some(name)
        })
        .collect::<Vec<_>>();
    names.sort_by_key(|name| name.to_ascii_lowercase());
    names
}

fn allowed_group_child_name(name: &str) -> bool {
    let clean = name.trim();
    !clean.is_empty()
        && !clean.starts_with('.')
        && !matches!(
            clean,
            "node_modules" | "target" | "dist" | "build" | ".git" | ".next"
        )
}

#[cfg(test)]
mod tests {
    use super::workspace_module_candidates;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn finds_direct_and_grouped_workspace_modules() {
        let root = temp_project("workspace-modules");
        fs::create_dir_all(root.join("server")).unwrap();
        fs::create_dir_all(root.join("packages").join("web")).unwrap();
        fs::create_dir_all(root.join("packages").join(".cache")).unwrap();
        fs::create_dir_all(root.join("apps").join("desktop")).unwrap();

        let modules = workspace_module_candidates(&root)
            .into_iter()
            .map(|candidate| candidate.module)
            .collect::<Vec<_>>();
        let _ = fs::remove_dir_all(root);

        assert!(modules.contains(&"server".to_string()));
        assert!(modules.contains(&"apps/desktop".to_string()));
        assert!(modules.contains(&"packages/web".to_string()));
        assert!(!modules.contains(&"packages/.cache".to_string()));
    }

    fn temp_project(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "elon-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }
}
