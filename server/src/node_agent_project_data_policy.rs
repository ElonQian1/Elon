use elon_pc_dev_runtime::NodeDataPaths;
use std::path::{Path, PathBuf};

/// Existing and external projects keep the environment that already proved
/// usable. Only workspaces created below the node-managed roots opt into the
/// recommended cache layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectDataPolicy {
    InheritExisting,
    ManagedRecommended,
}

impl ProjectDataPolicy {
    pub(crate) fn uses_managed_workspace(self) -> bool {
        matches!(self, Self::ManagedRecommended)
    }
}

pub(crate) fn classify(data_paths: Option<&NodeDataPaths>, workspace: &Path) -> ProjectDataPolicy {
    let Some(data_paths) = data_paths else {
        return ProjectDataPolicy::InheritExisting;
    };
    if path_is_within(workspace, &data_paths.workspaces())
        || path_is_within(workspace, &data_paths.storage())
    {
        ProjectDataPolicy::ManagedRecommended
    } else {
        ProjectDataPolicy::InheritExisting
    }
}

fn path_is_within(candidate: &Path, root: &Path) -> bool {
    let Some(candidate) = normalized_existing(candidate) else {
        return false;
    };
    let Some(root) = normalized_existing(root) else {
        return false;
    };
    if cfg!(windows) {
        let candidate = candidate
            .to_string_lossy()
            .replace('/', "\\")
            .to_lowercase();
        let root = root.to_string_lossy().replace('/', "\\").to_lowercase();
        candidate == root || candidate.starts_with(&format!("{}\\", root.trim_end_matches('\\')))
    } else {
        candidate == root || candidate.starts_with(root)
    }
}

fn normalized_existing(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    #[test]
    fn external_project_inherits_existing_environment() {
        let root = unique_root("external");
        let data = root.join("node-data");
        let external = root.join("existing-project");
        std::fs::create_dir_all(data.join("workspaces")).unwrap();
        std::fs::create_dir_all(data.join("storage")).unwrap();
        std::fs::create_dir_all(&external).unwrap();

        assert_eq!(
            classify(Some(&NodeDataPaths::new(&data)), &external),
            ProjectDataPolicy::InheritExisting
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn platform_workspace_uses_recommended_managed_layout() {
        let root = unique_root("managed");
        let data_paths = NodeDataPaths::new(root.join("node-data"));
        let project = data_paths
            .workspaces()
            .join("user")
            .join("project")
            .join("repo");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::create_dir_all(data_paths.storage()).unwrap();

        assert_eq!(
            classify(Some(&data_paths), &project),
            ProjectDataPolicy::ManagedRecommended
        );
        let _ = std::fs::remove_dir_all(root);
    }

    fn unique_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_nanos();
        std::env::temp_dir().join(format!("elon-project-data-policy-{label}-{nanos}"))
    }
}
