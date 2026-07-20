use std::{fs, path::Path};

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use super::{TaskJournal, TaskJournalSnapshot};
use crate::node_agent_task_journal_lock::with_task_journal_io_lock;

impl TaskJournal {
    pub(crate) fn snapshot(
        &self,
        task_id: &str,
        since: usize,
        limit: usize,
    ) -> Result<TaskJournalSnapshot> {
        self.snapshot_with_epoch(task_id, since, limit, None)
    }

    pub(crate) fn snapshot_with_epoch(
        &self,
        task_id: &str,
        since: usize,
        limit: usize,
        expected_cursor_epoch: Option<&str>,
    ) -> Result<TaskJournalSnapshot> {
        with_task_journal_io_lock(|| {
            let registry = self.load_registry()?;
            let record = registry.get(task_id).cloned();
            let event_limit = limit.clamp(1, 200);
            let cursor_epoch = self.cursor_epoch()?;

            // Read only this task and keep pagination bounded even when the
            // append-only journal contains many interleaved tasks.
            let initial_scan = self.scan_task_events(task_id, since, event_limit)?;
            let epoch_mismatch = expected_cursor_epoch
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .is_some_and(|expected| expected != cursor_epoch);
            let cursor_reset = epoch_mismatch || since > initial_scan.scanned_last_seq;
            let event_scan = if cursor_reset {
                self.scan_task_events(task_id, 0, event_limit)?
            } else {
                initial_scan
            };
            Ok(TaskJournalSnapshot {
                task_id: task_id.to_string(),
                record,
                approvals: event_scan.approvals,
                events: event_scan.events,
                last_event_seq: event_scan.last_event_seq,
                has_more: event_scan.has_more,
                cursor_epoch: cursor_epoch.clone(),
                requested_cursor_epoch: expected_cursor_epoch.map(ToOwned::to_owned),
                previous_cursor_epoch: expected_cursor_epoch.map(ToOwned::to_owned),
                cursor_reset,
                requested_cursor: since,
                old_cursor: since,
                new_cursor: event_scan.last_event_seq,
                resume_cursor: event_scan.last_event_seq,
                sidecar_update_epoch: cursor_epoch,
            })
        })
    }

    fn cursor_epoch(&self) -> Result<String> {
        let path = self.events_path();
        let source_identity = match fs::metadata(&path) {
            Ok(metadata) => event_file_identity(&path, &metadata)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => "missing".to_string(),
            Err(error) => return Err(error).with_context(|| format!("读取 {:?} 元数据", path)),
        };
        let digest =
            Sha256::digest(format!("{}\n{}", self.instance_epoch, source_identity).as_bytes());
        Ok(format!("journal-{}", hex::encode(&digest[..16])))
    }
}

#[cfg(windows)]
fn event_file_identity(path: &Path, _metadata: &fs::Metadata) -> Result<String> {
    use std::{ffi::c_void, fs::File, mem::MaybeUninit, os::windows::io::AsRawHandle};

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[repr(C)]
    struct ByHandleFileInformation {
        attributes: u32,
        creation_time: FileTime,
        last_access_time: FileTime,
        last_write_time: FileTime,
        volume_serial_number: u32,
        file_size_high: u32,
        file_size_low: u32,
        number_of_links: u32,
        file_index_high: u32,
        file_index_low: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFileInformationByHandle(
            file: *mut c_void,
            information: *mut ByHandleFileInformation,
        ) -> i32;
    }

    let file = File::open(path).with_context(|| format!("打开 {:?} 获取文件身份", path))?;
    let mut information = MaybeUninit::<ByHandleFileInformation>::uninit();
    // SAFETY: the live file handle and output pointer satisfy the Windows API.
    let succeeded =
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) != 0 };
    if !succeeded {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("读取 {:?} 文件身份", path));
    }
    // SAFETY: a successful API call initializes the complete structure.
    let information = unsafe { information.assume_init() };
    Ok(format!(
        "windows:{}:{}:{}",
        information.volume_serial_number, information.file_index_high, information.file_index_low
    ))
}

#[cfg(unix)]
fn event_file_identity(_path: &Path, metadata: &fs::Metadata) -> Result<String> {
    use std::os::unix::fs::MetadataExt;

    Ok(format!("unix:{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(not(any(windows, unix)))]
fn event_file_identity(_path: &Path, metadata: &fs::Metadata) -> Result<String> {
    let created = metadata
        .created()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    Ok(format!("portable:{created}"))
}
