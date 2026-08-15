use std::{
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::entrypoint_capsule::{
    with_external_pool_adapter_entrypoint_capsule, ExternalPoolAdapterEntrypointSource,
};
use crate::compute_federation::external_pool_adapter_linux_supervisor::ExternalPoolAdapterSupervisorCapsule;

struct RetainedTestEntrypoint {
    file: File,
    sha256: String,
    size_bytes: u64,
}

impl ExternalPoolAdapterEntrypointSource for RetainedTestEntrypoint {
    fn retained_entrypoint(&self) -> Result<(&File, &str, u64)> {
        Ok((&self.file, &self.sha256, self.size_bytes))
    }
}

pub(crate) fn with_materialized_external_pool_adapter_test_capsule<T>(
    source_bytes: &[u8],
    consume: impl FnOnce(&dyn ExternalPoolAdapterSupervisorCapsule) -> Result<T>,
) -> Result<T> {
    let root = std::env::temp_dir().join(format!(
        "elon-v267-production-materializer-{}",
        Uuid::new_v4().simple()
    ));
    std::fs::create_dir(&root).context("create V267 materializer fixture directory")?;
    let source_path = root.join("adapter");
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(&source_path)
            .context("create V267 retained source fixture")?;
        file.write_all(source_bytes)
            .context("write V267 retained source fixture")?;
        file.sync_data()
            .context("sync V267 retained source fixture")?;
        let source = RetainedTestEntrypoint {
            file,
            sha256: hex::encode(Sha256::digest(source_bytes)),
            size_bytes: source_bytes.len() as u64,
        };
        let mut output = None;
        with_external_pool_adapter_entrypoint_capsule(&source, |capsule| {
            output = Some(consume(capsule)?);
            Ok(())
        })?;
        output.ok_or_else(|| anyhow::anyhow!("V267 materializer callback did not run"))
    })();

    let source_cleanup = if source_path.exists() {
        std::fs::remove_file(&source_path).context("remove V267 retained source fixture")
    } else {
        Ok(())
    };
    let root_cleanup = std::fs::remove_dir(&root).context("remove V267 fixture directory");
    match (result, source_cleanup, root_cleanup) {
        (Ok(value), Ok(()), Ok(())) => Ok(value),
        (Err(error), _, _) => Err(error),
        (Ok(_), Err(error), _) | (Ok(_), Ok(()), Err(error)) => Err(error),
    }
}
