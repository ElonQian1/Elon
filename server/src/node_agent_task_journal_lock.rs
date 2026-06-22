use std::sync::{Mutex, OnceLock};

pub(crate) fn with_task_journal_io_lock<T>(operation: impl FnOnce() -> T) -> T {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let _guard = LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    operation()
}
