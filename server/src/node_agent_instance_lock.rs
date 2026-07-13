//! Process-lifetime lock for the node-agent state directory.
//!
//! Atomic replacement prevents torn JSON, but it cannot stop an old process
//! and its freshly updated replacement from writing complete stale snapshots
//! over each other. Acquire this guard before the first node.json read.

use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub(crate) struct NodeAgentInstanceLock {
    _file: File,
    path: PathBuf,
}

impl NodeAgentInstanceLock {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

pub(crate) fn acquire(state_path: &Path) -> Result<NodeAgentInstanceLock> {
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
    Ok(NodeAgentInstanceLock { _file: file, path })
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
        let first = acquire(&state).expect("first instance owns lock");
        assert!(acquire(&state).is_err());
        drop(first);
        acquire(&state).expect("lock is reusable after owner exits");
        let _ = std::fs::remove_dir_all(root);
    }
}
