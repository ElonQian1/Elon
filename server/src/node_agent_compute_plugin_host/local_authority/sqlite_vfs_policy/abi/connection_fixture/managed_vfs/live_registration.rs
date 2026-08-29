//! Redacted observation of the exact live test VFS registration retained by a child fixture.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestVfsLiveRegistrationSnapshot {
    table_present: bool,
    name_present: bool,
    context_present: bool,
    registered: bool,
}

impl ManagedTestVfsLiveRegistrationSnapshot {
    pub(super) fn table_present(self) -> bool {
        self.table_present
    }

    pub(super) fn name_present(self) -> bool {
        self.name_present
    }

    pub(super) fn context_present(self) -> bool {
        self.context_present
    }

    pub(super) fn registered(self) -> bool {
        self.registered
    }
}

impl ManagedTestVfsRegistration {
    pub(super) fn live_registration_snapshot(
        &self,
    ) -> anyhow::Result<ManagedTestVfsLiveRegistrationSnapshot> {
        let table = self.table.as_deref().context("managed VFS table custody")?;
        let name = self.name.as_ref().context("managed VFS name custody")?;
        let context = self
            .context
            .as_deref()
            .context("managed VFS context custody")?;
        // SAFETY: `name` is the live NUL-terminated registration name retained by this owner.
        let lookup = unsafe { ffi::sqlite3_vfs_find(name.as_ptr()) };
        let exact_table = std::ptr::eq(lookup.cast_const(), table as *const ffi::sqlite3_vfs);
        Ok(ManagedTestVfsLiveRegistrationSnapshot {
            table_present: exact_table,
            name_present: exact_table && table.zName == name.as_ptr(),
            context_present: exact_table
                && table.pAppData == (context as *const ManagedTestVfsContext).cast_mut().cast(),
            registered: self.registered && exact_table,
        })
    }
}
