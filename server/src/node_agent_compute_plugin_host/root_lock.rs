use std::{ffi::OsStr, fmt, sync::Arc};

use anyhow::{bail, Context, Result};

use crate::node_agent_managed_fs::{PinnedManagedDirectory, PinnedManagedExclusiveFileLock};

const COMPUTE_PLUGIN_ROOT_LOCK_FILE: &str = ".compute-plugin-root.lock";

/// Process-lifetime exclusion for one already pinned compute-plugin directory. The persistent
/// file is only a rendezvous point; authority comes from the retained share-none handle and its
/// pinned parent chain, never from lock-file contents or an absolute-path reopen.
#[must_use = "dropping the compute-plugin root lock releases cross-process exclusion"]
pub(super) struct ComputePluginRootLock {
    inner: Arc<ComputePluginRootLockInner>,
}

struct ComputePluginRootLockInner {
    exclusive: PinnedManagedExclusiveFileLock,
}

/// Linear lease retained by every capability that can continue touching files below the pinned
/// compute-plugin root. The lease is deliberately not `Clone`; only the owning root may mint
/// another lease, while all leases keep the exact same share-none OS handle alive through `Arc`.
#[must_use = "dropping the root-lock lease may release cross-process exclusion"]
pub(super) struct ComputePluginRootLockLease {
    inner: Arc<ComputePluginRootLockInner>,
}

impl ComputePluginRootLock {
    pub(super) fn acquire(directory: PinnedManagedDirectory) -> Result<Self> {
        if directory.filesystem_mutated() {
            bail!("COMPUTE_PLUGIN_ROOT_LOCK_DIRECTORY_MUTATED");
        }
        let exclusive = directory
            .acquire_exclusive_file_lock(OsStr::new(COMPUTE_PLUGIN_ROOT_LOCK_FILE))
            .map_err(anyhow::Error::new)
            .context("COMPUTE_PLUGIN_ROOT_LOCK_ACQUIRE")?;
        Ok(Self {
            inner: Arc::new(ComputePluginRootLockInner { exclusive }),
        })
    }

    pub(super) fn lease(&self) -> ComputePluginRootLockLease {
        ComputePluginRootLockLease {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl fmt::Debug for ComputePluginRootLock {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRootLock")
            .field("exclusive", &self.inner.exclusive)
            .finish()
    }
}

impl fmt::Debug for ComputePluginRootLockLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ComputePluginRootLockLease")
            .field("exclusive", &self.inner.exclusive)
            .finish()
    }
}
