use std::path::{Path, PathBuf};

use crate::safe_path_part;

pub const NODE_DATA_ROOT_ENV: &str = "ELON_NODE_DATA_ROOT";

/// Large, reproducible PC-node data is rooted here instead of following the
/// Windows user profile. The value is intentionally pure: configuration and
/// filesystem migration belong to the node-agent layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDataPaths {
    root: PathBuf,
}

impl NodeDataPaths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn workspaces(&self) -> PathBuf {
        self.root.join("workspaces")
    }

    pub fn storage(&self) -> PathBuf {
        self.root.join("storage")
    }

    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn temp(&self) -> PathBuf {
        self.root.join("temp")
    }

    pub fn cargo_home(&self) -> PathBuf {
        self.cache().join("cargo-home")
    }

    pub fn rust_targets(&self) -> PathBuf {
        self.cache().join("rust-targets")
    }

    pub fn gradle_home(&self) -> PathBuf {
        self.cache().join("gradle-home")
    }

    pub fn npm_cache(&self) -> PathBuf {
        self.cache().join("npm")
    }

    pub fn pnpm_store(&self) -> PathBuf {
        self.cache().join("pnpm-store")
    }

    pub fn yarn_cache(&self) -> PathBuf {
        self.cache().join("yarn")
    }

    /// Worktrees of the same project share this target, while unrelated
    /// projects and explicitly different Rust toolchains never write together.
    pub fn project_rust_target(&self, project_id: &str, toolchain_key: &str) -> PathBuf {
        self.rust_targets()
            .join(safe_path_part(project_id, "project", 96))
            .join(safe_path_part(toolchain_key, "default", 80))
            .join("target")
    }

    pub fn task_temp(&self, task_id: &str) -> PathBuf {
        self.temp().join(safe_path_part(task_id, "task", 96))
    }

    pub fn managed_roots(&self) -> [PathBuf; 4] {
        [self.workspaces(), self.storage(), self.cache(), self.temp()]
    }
}

#[cfg(test)]
mod tests {
    use super::NodeDataPaths;
    use std::path::PathBuf;

    #[test]
    fn derives_stable_layout_from_single_root() {
        let paths = NodeDataPaths::new(PathBuf::from("D:/ElonNodeData"));

        assert_eq!(
            paths.workspaces(),
            PathBuf::from("D:/ElonNodeData/workspaces")
        );
        assert_eq!(paths.storage(), PathBuf::from("D:/ElonNodeData/storage"));
        assert_eq!(
            paths.gradle_home(),
            PathBuf::from("D:/ElonNodeData/cache/gradle-home")
        );
        assert_eq!(
            paths.yarn_cache(),
            PathBuf::from("D:/ElonNodeData/cache/yarn")
        );
        assert_eq!(
            paths.task_temp("task/one"),
            PathBuf::from("D:/ElonNodeData/temp/taskone")
        );
    }

    #[test]
    fn rust_targets_are_shared_by_project_and_toolchain_only() {
        let paths = NodeDataPaths::new(PathBuf::from("D:/ElonNodeData"));

        let stable = paths.project_rust_target("prj-1", "stable-msvc");
        let nightly = paths.project_rust_target("prj-1", "nightly-msvc");
        let other = paths.project_rust_target("prj-2", "stable-msvc");

        assert_ne!(stable, nightly);
        assert_ne!(stable, other);
        assert!(stable.ends_with("prj-1/stable-msvc/target"));
    }
}
