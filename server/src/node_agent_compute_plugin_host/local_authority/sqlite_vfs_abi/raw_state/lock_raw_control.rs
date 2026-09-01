//! Closed, memory-safe Windows-test controller for q11 raw Lock representations.
//!
//! This controller never accepts pointers or slot values from a caller. It starts from one exact
//! installed file, then performs one reviewed mutation selected by the closed enum. Representations
//! that prevent SQLite close intentionally retain their original fixture custody until the child
//! process exits. The two invalid-pointer safety premises are absent by construction.

use std::{
    any::TypeId,
    os::raw::c_void,
    ptr::{self, NonNull},
};

use rusqlite::ffi;

use super::{drop_typed_payload, RawSqliteFileStateEnvelope};
use crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi::{
    file_state::HandleBoundSqliteFileState,
    raw_lock_observation::HandleBoundSqliteAbiRawLockRejectionCaseV1,
    types::InertHandleBoundSqliteFile, INERT_IO_METHODS,
};

struct OtherTypePayloadMissing;
struct OtherTypePayloadDropCompleted;
struct OtherTypePayloadDropUnwind;

impl Drop for OtherTypePayloadDropUnwind {
    fn drop(&mut self) {
        panic!("controlled q11 raw Lock payload Drop unwind");
    }
}

/// Three-bit slot tag: methods-present, state-present, exact-methods.
///
/// # Safety
///
/// `file` must identify this ABI module's live, initialized and serialized allocation.
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) unsafe fn slot_tag(
    file: *mut ffi::sqlite3_file,
) -> Result<u64, &'static str> {
    let file = NonNull::new(file.cast::<InertHandleBoundSqliteFile>())
        .ok_or("raw Lock slot snapshot file was null")?;
    // SAFETY: forwarded from the exact live-allocation contract above.
    let (methods, state) = unsafe {
        (
            ptr::addr_of!((*file.as_ptr()).base.pMethods).read(),
            ptr::addr_of!((*file.as_ptr()).state).read(),
        )
    };
    Ok(u64::from(!methods.is_null())
        | (u64::from(!state.is_null()) << 1)
        | (u64::from(ptr::eq(methods, &INERT_IO_METHODS)) << 2))
}

/// Returns `(prepared_slot_tag, retained_fixture_tag)` after exactly one closed mutation.
/// Retention bits are: detached original envelope, detached HandleBound file, and terminal
/// raw/adapter representation requiring process-exit isolation.
///
/// # Safety
///
/// The caller must have independently proved exact methods, expected state type and live payload,
/// and must serialize this mutation with every SQLite callback.
pub(in crate::node_agent_compute_plugin_host::local_authority::sqlite_vfs_abi) unsafe fn prepare(
    file: *mut ffi::sqlite3_file,
    case_v1: HandleBoundSqliteAbiRawLockRejectionCaseV1,
) -> Result<(u64, u64), &'static str> {
    let file = NonNull::new(file.cast::<InertHandleBoundSqliteFile>())
        .ok_or("raw Lock controller source file was null")?;
    let envelope = unsafe { exact_expected_envelope(file)? };
    if !envelope.has_payload() {
        return Err("raw Lock controller source payload was missing");
    }

    let retained = match case_v1 {
        HandleBoundSqliteAbiRawLockRejectionCaseV1::NullFileDirect => 0,
        HandleBoundSqliteAbiRawLockRejectionCaseV1::UninstalledDirect => {
            unsafe {
                detach_original_envelope(file);
                write_methods(file, ptr::null());
            }
            0b101
        }
        HandleBoundSqliteAbiRawLockRejectionCaseV1::MethodsNullStatePresentDirect => {
            unsafe { write_methods(file, ptr::null()) };
            0b100
        }
        HandleBoundSqliteAbiRawLockRejectionCaseV1::ForeignMethodsStateNullDirect => {
            unsafe {
                detach_original_envelope(file);
                write_methods(file, &FOREIGN_IO_METHODS);
            }
            0b101
        }
        HandleBoundSqliteAbiRawLockRejectionCaseV1::ForeignMethodsStatePresentDirect => {
            unsafe { write_methods(file, &FOREIGN_IO_METHODS) };
            0b100
        }
        HandleBoundSqliteAbiRawLockRejectionCaseV1::ExactMethodsStateNullDirect => {
            unsafe { detach_original_envelope(file) };
            0b101
        }
        HandleBoundSqliteAbiRawLockRejectionCaseV1::OtherTypePayloadMissingDropCompleted => {
            unsafe {
                replace_envelope(
                    file,
                    empty_envelope::<OtherTypePayloadMissing>(),
                );
            }
            0b101
        }
        HandleBoundSqliteAbiRawLockRejectionCaseV1::OtherTypePayloadPresentDropCompleted => {
            unsafe {
                replace_envelope(
                    file,
                    RawSqliteFileStateEnvelope::new(OtherTypePayloadDropCompleted),
                );
            }
            0b101
        }
        HandleBoundSqliteAbiRawLockRejectionCaseV1::OtherTypePayloadPresentDropUnwindCaught => {
            unsafe {
                replace_envelope(
                    file,
                    RawSqliteFileStateEnvelope::new(OtherTypePayloadDropUnwind),
                );
            }
            0b101
        }
        HandleBoundSqliteAbiRawLockRejectionCaseV1::ExpectedTypePayloadMissingDropCompleted => {
            unsafe {
                replace_envelope(file, empty_envelope::<HandleBoundSqliteFileState>());
            }
            0b101
        }
        HandleBoundSqliteAbiRawLockRejectionCaseV1::HandleBoundFileMissingDirect => {
            // SAFETY: exact_expected_envelope checked the TypeId and live payload. The test-only
            // state method removes and deliberately retains the concrete file until process exit.
            let detached = unsafe {
                envelope.with_typed::<HandleBoundSqliteFileState, _>(|state| {
                    state.detach_file_for_raw_lock_rejection()
                })
            };
            if !detached {
                return Err("raw Lock controller HandleBound file was already missing");
            }
            0b110
        }
    };
    let slots = unsafe { slot_tag(file.as_ptr().cast()) }?;
    Ok((slots, retained))
}

unsafe fn exact_expected_envelope<'file>(
    file: NonNull<InertHandleBoundSqliteFile>,
) -> Result<&'file mut RawSqliteFileStateEnvelope, &'static str> {
    // SAFETY: the controller caller owns serialized access to this exact initialized allocation.
    let (methods, state) = unsafe {
        (
            ptr::addr_of!((*file.as_ptr()).base.pMethods).read(),
            ptr::addr_of!((*file.as_ptr()).state).read(),
        )
    };
    if !ptr::eq(methods, &INERT_IO_METHODS) || state.is_null() {
        return Err("raw Lock controller source slots were not exact and installed");
    }
    // SAFETY: exact installation was independently validated before controller entry.
    let envelope = unsafe { state.cast::<RawSqliteFileStateEnvelope>().as_mut() }
        .ok_or("raw Lock controller source envelope was null")?;
    if !envelope.is::<HandleBoundSqliteFileState>() {
        return Err("raw Lock controller source envelope type was not HandleBound state");
    }
    Ok(envelope)
}

fn empty_envelope<State: 'static>() -> Box<RawSqliteFileStateEnvelope> {
    Box::new(RawSqliteFileStateEnvelope {
        type_id: TypeId::of::<State>(),
        payload: None,
        drop_payload: drop_typed_payload::<State>,
    })
}

unsafe fn detach_original_envelope(file: NonNull<InertHandleBoundSqliteFile>) {
    // The exact original envelope is deliberately left allocated. Losing its owning pointer is a
    // closed child-only retention mechanism, not a production cleanup claim.
    unsafe {
        let _retained = ptr::addr_of_mut!((*file.as_ptr()).state).replace(ptr::null_mut());
    }
}

unsafe fn replace_envelope(
    file: NonNull<InertHandleBoundSqliteFile>,
    envelope: Box<RawSqliteFileStateEnvelope>,
) {
    unsafe { detach_original_envelope(file) };
    let raw = Box::into_raw(envelope).cast::<c_void>();
    unsafe { ptr::addr_of_mut!((*file.as_ptr()).state).write(raw) };
}

unsafe fn write_methods(
    file: NonNull<InertHandleBoundSqliteFile>,
    methods: *const ffi::sqlite3_io_methods,
) {
    unsafe { ptr::addr_of_mut!((*file.as_ptr()).base.pMethods).write(methods) };
}

// A valid, immutable method-table object whose address is intentionally not the exact table.
// The production raw gate compares the pointer and never dispatches through this table.
static FOREIGN_IO_METHODS: ffi::sqlite3_io_methods = ffi::sqlite3_io_methods {
    iVersion: 0,
    xClose: None,
    xRead: None,
    xWrite: None,
    xTruncate: None,
    xSync: None,
    xFileSize: None,
    xLock: None,
    xUnlock: None,
    xCheckReservedLock: None,
    xFileControl: None,
    xSectorSize: None,
    xDeviceCharacteristics: None,
    xShmMap: None,
    xShmLock: None,
    xShmBarrier: None,
    xShmUnmap: None,
    xFetch: None,
    xUnfetch: None,
};
