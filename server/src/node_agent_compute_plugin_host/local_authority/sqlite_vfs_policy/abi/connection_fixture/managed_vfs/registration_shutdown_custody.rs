//! Sealed registration identity and retained table/name/context custody observations.

use std::{
    ffi::CString,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use rusqlite::ffi;

use super::{ManagedTestVfsContext, ManagedTestVfsRegistration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ManagedTestVfsRegistrationDisposition {
    Registered,
    Unregistered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ManagedTestVfsRetainedPartsSnapshot {
    disposition: ManagedTestVfsRegistrationDisposition,
    table_present: bool,
    name_present: bool,
    context_present: bool,
}

impl ManagedTestVfsRetainedPartsSnapshot {
    pub(super) fn disposition(self) -> ManagedTestVfsRegistrationDisposition {
        self.disposition
    }

    pub(super) fn table_present(self) -> bool {
        self.table_present
    }

    pub(super) fn name_present(self) -> bool {
        self.name_present
    }

    pub(super) fn context_present(self) -> bool {
        self.context_present
    }
}

#[derive(Debug, Default)]
pub(super) struct ManagedTestVfsRetainedPartsWitness {
    snapshot: Mutex<Option<ManagedTestVfsRetainedPartsSnapshot>>,
}

pub(super) struct ManagedTestVfsRegistrationCustody {
    _table: Option<Box<ffi::sqlite3_vfs>>,
    _name: Option<CString>,
    _context: Option<Box<ManagedTestVfsContext>>,
    _disposition: ManagedTestVfsRegistrationDisposition,
    _witness: Arc<ManagedTestVfsRetainedPartsWitness>,
}

pub(super) struct ManagedTestRegistrationShutdownTargetWitness {
    registration_id: u64,
    occurrence: u32,
}

impl ManagedTestRegistrationShutdownTargetWitness {
    pub(super) fn registration_id(&self) -> u64 {
        self.registration_id
    }

    pub(super) fn occurrence(&self) -> u32 {
        self.occurrence
    }
}

impl ManagedTestVfsRegistration {
    pub(super) fn retained_parts_witness(&self) -> Arc<ManagedTestVfsRetainedPartsWitness> {
        Arc::clone(&self.retained_parts_witness)
    }

    pub(super) fn registration_shutdown_target_witness(
        &self,
    ) -> anyhow::Result<ManagedTestRegistrationShutdownTargetWitness> {
        if self.registration_shutdown_attempts == 0 {
            return Err(anyhow!(
                "registration shutdown target was observed before an actual attempt"
            ));
        }
        Ok(ManagedTestRegistrationShutdownTargetWitness {
            registration_id: self.id.counter_value(),
            occurrence: self.registration_shutdown_attempts,
        })
    }

    pub(super) fn retain_registered_parts(&mut self) {
        let retained = self.take_registered_parts();
        let _custody = Box::leak(Box::new(retained));
    }

    pub(super) fn take_registered_parts(&mut self) -> ManagedTestVfsRegistrationCustody {
        let disposition = if self.registered {
            ManagedTestVfsRegistrationDisposition::Registered
        } else {
            ManagedTestVfsRegistrationDisposition::Unregistered
        };
        self.registered = false;
        let table = self.table.take();
        let name = self.name.take();
        let context = self.context.take();
        let snapshot = ManagedTestVfsRetainedPartsSnapshot {
            disposition,
            table_present: table.is_some(),
            name_present: name.is_some(),
            context_present: context.is_some(),
        };
        let witness = Arc::clone(&self.retained_parts_witness);
        let custody = ManagedTestVfsRegistrationCustody {
            _table: table,
            _name: name,
            _context: context,
            _disposition: disposition,
            _witness: Arc::clone(&witness),
        };
        witness.record(snapshot);
        custody
    }
}

impl ManagedTestVfsRetainedPartsWitness {
    pub(super) fn snapshot(&self) -> Option<ManagedTestVfsRetainedPartsSnapshot> {
        *self.snapshot.lock().expect("retained VFS witness lock")
    }

    fn record(&self, snapshot: ManagedTestVfsRetainedPartsSnapshot) {
        let mut current = self.snapshot.lock().expect("retained VFS witness lock");
        assert!(
            current.replace(snapshot).is_none(),
            "retained VFS parts may be witnessed only once"
        );
    }
}
