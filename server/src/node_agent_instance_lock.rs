//! Process-lifetime lock for the node-agent state directory.
//!
//! Atomic replacement prevents torn JSON, but it cannot stop an old process
//! and its freshly updated replacement from writing complete stale snapshots
//! over each other. Acquire this guard before the first node.json read.

use std::{
    fmt,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};

use anyhow::{Context, Result};

pub(crate) struct NodeAgentInstanceLock {
    inner: Arc<NodeAgentInstanceLockInner>,
}

struct NodeAgentInstanceLockInner {
    _file: File,
    path: PathBuf,
}

/// Opaque process-local identity for one exact instance-lock handle. Keeping this value does not
/// retain the lock; it only lets later linear custody prove that its lease descends from the same
/// configured lock acquired at startup.
#[derive(Clone)]
pub(crate) struct NodeAgentInstanceLockBinding {
    inner: Weak<NodeAgentInstanceLockInner>,
}

/// Process-local proof that the real node instance lock is still retained. A weak witness cannot
/// keep the lock alive and becomes unusable as soon as the owning guard is dropped.
#[derive(Clone)]
pub(crate) struct NodeAgentInstanceLockWitness {
    inner: Weak<NodeAgentInstanceLockInner>,
}

/// Operation-scoped retention of the same file handle that owns the node state-directory lock.
/// Bootstrap stores only a weak witness. This lease is a necessary process-liveness prerequisite,
/// but it does not make a separately configured compute-plugin root exclusive.
pub(crate) struct NodeAgentInstanceLockLease {
    inner: Arc<NodeAgentInstanceLockInner>,
}

impl NodeAgentInstanceLock {
    pub(crate) fn path(&self) -> &Path {
        &self.inner.path
    }

    pub(crate) fn liveness_witness(&self) -> NodeAgentInstanceLockWitness {
        NodeAgentInstanceLockWitness {
            inner: Arc::downgrade(&self.inner),
        }
    }
}

impl NodeAgentInstanceLockWitness {
    pub(crate) fn is_live(&self) -> bool {
        self.inner.strong_count() > 0
    }

    pub(crate) fn try_acquire_lease(&self) -> Option<NodeAgentInstanceLockLease> {
        self.inner
            .upgrade()
            .map(|inner| NodeAgentInstanceLockLease { inner })
    }

    pub(crate) fn try_acquire_bound_lease(
        &self,
    ) -> Option<(NodeAgentInstanceLockBinding, NodeAgentInstanceLockLease)> {
        let inner = self.inner.upgrade()?;
        let binding = NodeAgentInstanceLockBinding {
            inner: Arc::downgrade(&inner),
        };
        Some((binding, NodeAgentInstanceLockLease { inner }))
    }
}

impl NodeAgentInstanceLockBinding {
    pub(crate) fn matches_witness(&self, witness: &NodeAgentInstanceLockWitness) -> bool {
        self.inner.ptr_eq(&witness.inner)
    }

    pub(crate) fn matches_lease(&self, lease: &NodeAgentInstanceLockLease) -> bool {
        self.inner
            .upgrade()
            .is_some_and(|inner| Arc::ptr_eq(&inner, &lease.inner))
    }
}

impl PartialEq for NodeAgentInstanceLockBinding {
    fn eq(&self, other: &Self) -> bool {
        self.inner.ptr_eq(&other.inner)
    }
}

impl Eq for NodeAgentInstanceLockBinding {}

impl fmt::Debug for NodeAgentInstanceLockWitness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NodeAgentInstanceLockWitness")
            .field("liveness", &"<process-local>")
            .finish()
    }
}

/// Production acquisition is intentionally bound to the one configured node-state location.
/// Callers cannot mint an authority-compatible witness for an arbitrary path.
pub(crate) fn acquire_configured() -> Result<NodeAgentInstanceLock> {
    acquire_at(&crate::node_agent_config::state_path())
}

fn acquire_at(state_path: &Path) -> Result<NodeAgentInstanceLock> {
    let state_dir = state_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("节点状态路径没有父目录: {}", state_path.display()))?;
    std::fs::create_dir_all(state_dir)
        .with_context(|| format!("无法创建节点状态目录 {}", state_dir.display()))?;
    let path = state_dir.join("node-agent.instance.lock");
    let file = open_exclusive(&path).with_context(|| {
        format!(
            "另一份一龙 PC 节点进程正在使用状态目录；为防止覆盖 node.json，当前进程拒绝启动: {}",
            path.display()
        )
    })?;
    Ok(NodeAgentInstanceLock {
        inner: Arc::new(NodeAgentInstanceLockInner { _file: file, path }),
    })
}

#[cfg(windows)]
fn open_exclusive(path: &Path) -> std::io::Result<File> {
    use std::os::windows::fs::OpenOptionsExt;

    OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .share_mode(0)
        .open(path)
}

#[cfg(not(windows))]
fn open_exclusive(path: &Path) -> std::io::Result<File> {
    use std::os::fd::AsRawFd;

    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;
    if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(file)
}

#[cfg(not(windows))]
unsafe extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_guard_can_own_a_state_directory() {
        let root =
            std::env::temp_dir().join(format!("elon-node-instance-lock-{}", uuid::Uuid::new_v4()));
        let state = root.join("node.json");
        let first = acquire_at(&state).expect("first instance owns lock");
        assert!(acquire_at(&state).is_err());
        drop(first);
        acquire_at(&state).expect("lock is reusable after owner exits");
        let _ = std::fs::remove_dir_all(root);
    }
}
