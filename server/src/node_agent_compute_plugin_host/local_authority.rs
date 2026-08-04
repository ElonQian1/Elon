use std::{path::PathBuf, time::Duration};

use anyhow::{bail, Context, Result};
use rusqlite::{Connection, Transaction, TransactionBehavior};

use super::local_authority_schema;

mod fetch_claim_revocation;
mod initialization;
mod keyring_integrity;
mod keyring_snapshot;
mod keyring_store;
mod plan_application;
mod plan_application_persistence;
mod plan_application_projection;
mod plan_application_replay_children;
mod plan_application_writes;

pub(crate) use initialization::{
    ComputePluginAuthorityInitialization, ComputePluginAuthorityInitializationOutcome,
};
pub(crate) use keyring_store::{
    ComputePluginKeyringInstallDisposition, ComputePluginKeyringInstallResult,
};
pub(crate) use plan_application::{
    ComputePluginCandidateHandle, ComputePluginPlanApplicationDisposition,
    ComputePluginPlanApplicationReceipt, ComputePluginPlanApplicationResult,
};

const COMPUTE_PLUGIN_STATE_FILE: &str = "compute-plugin-state.sqlite3";
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Path-based facade for the plugin control-state authority. It is intentionally not wired into
/// the Host yet; opening the database must happen only after the NodeAgent instance lock is held.
#[derive(Debug, Clone)]
pub(crate) struct ComputePluginLocalAuthority {
    path: PathBuf,
}

impl Default for ComputePluginLocalAuthority {
    fn default() -> Self {
        Self::new(crate::node_agent_config::state_path().with_file_name(COMPUTE_PLUGIN_STATE_FILE))
    }
}

impl ComputePluginLocalAuthority {
    pub(crate) fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub(crate) fn path(&self) -> &std::path::Path {
        &self.path
    }

    /// Opens and validates schema only; this does not initialize authority facts or enable sharing.
    pub(crate) fn ensure_schema(&self) -> Result<()> {
        self.connect().map(drop)
    }

    /// Only nested authority kernels may use the generic transaction seam. Host and downloader
    /// code receive purpose-specific operations so they cannot partially update authority tables.
    fn with_immediate<T>(
        &self,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        let mut connection = self.connect()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("COMPUTE_PLUGIN_AUTHORITY_BEGIN_IMMEDIATE")?;
        let value = operation(&transaction)?;
        transaction
            .commit()
            .context("COMPUTE_PLUGIN_AUTHORITY_COMMIT")?;
        Ok(value)
    }

    fn connect(&self) -> Result<Connection> {
        let parent = self.path.parent().ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_AUTHORITY_PATH: database has no parent directory")
        })?;
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "COMPUTE_PLUGIN_AUTHORITY_DIRECTORY: cannot create {}",
                parent.display()
            )
        })?;
        let mut connection = Connection::open(&self.path).with_context(|| {
            format!(
                "COMPUTE_PLUGIN_AUTHORITY_OPEN: cannot open {}",
                self.path.display()
            )
        })?;
        connection
            .busy_timeout(BUSY_TIMEOUT)
            .context("COMPUTE_PLUGIN_AUTHORITY_BUSY_TIMEOUT")?;
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .context("COMPUTE_PLUGIN_AUTHORITY_WAL")?;
        connection
            .pragma_update(None, "synchronous", "FULL")
            .context("COMPUTE_PLUGIN_AUTHORITY_SYNCHRONOUS")?;
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEYS")?;
        let foreign_keys = connection
            .pragma_query_value(None, "foreign_keys", |row| row.get::<_, i64>(0))
            .context("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEYS_READ")?;
        if foreign_keys != 1 {
            bail!("COMPUTE_PLUGIN_AUTHORITY_FOREIGN_KEYS_DISABLED");
        }
        connection
            .pragma_update(None, "trusted_schema", "OFF")
            .context("COMPUTE_PLUGIN_AUTHORITY_TRUSTED_SCHEMA")?;
        local_authority_schema::ensure_schema(&mut connection)?;
        Ok(connection)
    }
}
