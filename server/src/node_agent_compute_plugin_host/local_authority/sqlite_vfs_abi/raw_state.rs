//! Unique ownership protocol for Rust state stored behind `sqlite3_file.state`.
//!
//! The production inert `xOpen` only initializes fresh storage and never installs state. These
//! private primitives reserve the future transition: one exact I/O table, closure-scoped typed
//! borrows, a consuming typed take, and a type-erased fail-closed abandonment path.

use std::{
    any::TypeId,
    os::raw::c_void,
    ptr::{self, NonNull},
};

use rusqlite::ffi;

use super::{types::InertHandleBoundSqliteFile, INERT_IO_METHODS};

#[cfg(all(test, windows))]
mod close_control;
#[cfg(all(test, windows))]
mod close_witness;

#[cfg(all(test, windows))]
pub(in crate::node_agent_compute_plugin_host::local_authority) use close_witness::{
    HandleBoundSqliteAbiRawCloseWitness, HandleBoundSqliteAbiRawCloseWitnessSnapshot,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RawSqliteFileStateRejection {
    NullFile,
    Occupied,
    Uninstalled,
    ForeignMethods,
    StateMissing,
    TypeMismatch,
    #[cfg(all(test, windows))]
    TestStateTakeRejected,
}

struct RawSqliteFileStateEnvelope {
    type_id: TypeId,
    payload: Option<NonNull<c_void>>,
    drop_payload: unsafe fn(NonNull<c_void>),
}

impl RawSqliteFileStateEnvelope {
    fn new<State: 'static>(state: State) -> Box<Self> {
        let payload = NonNull::from(Box::leak(Box::new(state))).cast();
        Box::new(Self {
            type_id: TypeId::of::<State>(),
            payload: Some(payload),
            drop_payload: drop_typed_payload::<State>,
        })
    }

    fn is<State: 'static>(&self) -> bool {
        self.type_id == TypeId::of::<State>()
    }

    unsafe fn with_typed<State: 'static, Output>(
        &mut self,
        operation: impl FnOnce(&mut State) -> Output,
    ) -> Output {
        let payload = self
            .payload
            .expect("live raw SQLite state envelope must retain its payload")
            .cast::<State>();
        // SAFETY: the TypeId gate is checked before this method is called, and the callback
        // boundary guarantees exclusive access to this sqlite3_file for the operation scope.
        operation(unsafe {
            payload
                .as_ptr()
                .as_mut()
                .expect("boxed payload is non-null")
        })
    }

    unsafe fn take_typed<State: 'static>(mut envelope: Box<Self>) -> Box<State> {
        let payload = envelope
            .payload
            .take()
            .expect("live raw SQLite state envelope must retain its payload")
            .cast::<State>();
        drop(envelope);
        // SAFETY: ownership of the exact pointer was removed from the envelope after the TypeId
        // gate, and no other Box can be reconstructed by this protocol.
        unsafe { Box::from_raw(payload.as_ptr()) }
    }
}

impl Drop for RawSqliteFileStateEnvelope {
    fn drop(&mut self) {
        if let Some(payload) = self.payload.take() {
            // SAFETY: the function pointer was paired with this payload in `new`.
            unsafe { (self.drop_payload)(payload) };
        }
    }
}

unsafe fn drop_typed_payload<State>(payload: NonNull<c_void>) {
    // SAFETY: callers use only the drop function paired by `RawSqliteFileStateEnvelope::new`.
    drop(unsafe { Box::from_raw(payload.cast::<State>().as_ptr()) });
}

/// Initializes the fresh, potentially uninitialized storage supplied to `xOpen`.
///
/// This is deliberately separate from close/take. Calling it for an initialized file would erase
/// custody without running Drop and violates the safety contract.
///
/// # Safety
///
/// A non-null pointer must identify a fresh, aligned `szOsFile` allocation supplied to this VFS.
pub(super) unsafe fn initialize_fresh_file(file: *mut ffi::sqlite3_file) -> bool {
    let Some(file) = NonNull::new(file.cast::<InertHandleBoundSqliteFile>()) else {
        return false;
    };
    // SAFETY: the callback contract grants the full fresh allocation. Writes do not read its
    // uninitialized prior contents.
    unsafe {
        ptr::addr_of_mut!((*file.as_ptr()).base.pMethods).write(ptr::null());
        ptr::addr_of_mut!((*file.as_ptr()).state).write(ptr::null_mut());
        #[cfg(all(test, windows))]
        ptr::addr_of_mut!((*file.as_ptr()).raw_close_control).write(ptr::null_mut());
    }
    true
}

/// Installs one state value only into storage already initialized by `initialize_fresh_file`.
/// A rejection returns the input state unchanged.
///
/// # Safety
///
/// `file` must remain a live allocation owned by SQLite, initialized by this module, and not be
/// accessed concurrently for the duration of the call.
pub(super) unsafe fn install_state<State: 'static>(
    file: *mut ffi::sqlite3_file,
    state: State,
) -> Result<(), (RawSqliteFileStateRejection, State)> {
    let Some(file) = NonNull::new(file.cast::<InertHandleBoundSqliteFile>()) else {
        return Err((RawSqliteFileStateRejection::NullFile, state));
    };
    // SAFETY: guaranteed by the initialized-file contract above.
    let occupied = unsafe {
        !ptr::addr_of!((*file.as_ptr()).base.pMethods)
            .read()
            .is_null()
            || !ptr::addr_of!((*file.as_ptr()).state).read().is_null()
    };
    #[cfg(all(test, windows))]
    // A retained exact-allocation close control is terminal custody, not reusable fresh storage.
    let occupied = occupied
        || unsafe {
            !ptr::addr_of!((*file.as_ptr()).raw_close_control)
                .read()
                .is_null()
        };
    if occupied {
        return Err((RawSqliteFileStateRejection::Occupied, state));
    }

    let envelope = Box::into_raw(RawSqliteFileStateEnvelope::new(state)).cast::<c_void>();
    // Install state before methods so no callback path can observe a missing payload. Neither
    // pointer write can fail; the inverse take order first disables methods and then clears state.
    // SAFETY: guaranteed by the initialized-file contract above.
    unsafe {
        ptr::addr_of_mut!((*file.as_ptr()).state).write(envelope);
        ptr::addr_of_mut!((*file.as_ptr()).base.pMethods).write(&INERT_IO_METHODS);
    }
    Ok(())
}

/// Runs one closure-scoped mutable borrow after exact methods and runtime type checks.
///
/// # Safety
///
/// The file must have been installed by this module and SQLite/caller must serialize callback
/// access so no other mutable or consuming access overlaps the closure.
pub(super) unsafe fn with_installed_state<State: 'static, Output>(
    file: *mut ffi::sqlite3_file,
    operation: impl FnOnce(&mut State) -> Output,
) -> Result<Output, RawSqliteFileStateRejection> {
    let envelope = unsafe { installed_envelope(file)? };
    if !envelope.is::<State>() {
        return Err(RawSqliteFileStateRejection::TypeMismatch);
    }
    // SAFETY: exact method and TypeId gates passed, with exclusivity required by this function.
    Ok(unsafe { envelope.with_typed(operation) })
}

/// Disables callbacks, clears the raw slot, and returns the uniquely owned typed state.
///
/// # Safety
///
/// The file must have been installed by this module, and no callback/borrow may overlap the take.
pub(super) unsafe fn take_installed_state<State: 'static>(
    file: *mut ffi::sqlite3_file,
) -> Result<Box<State>, RawSqliteFileStateRejection> {
    #[cfg(all(test, windows))]
    // SAFETY: forwarded from this function's serialized live-allocation contract. Recording before
    // validation makes a second xClose attempt durable even after the first call cleared raw slots.
    unsafe {
        record_test_state_take_attempt(file);
    }
    #[cfg(all(test, windows))]
    // SAFETY: this is the same serialized allocation. The exact test control rejects before
    // either raw slot changes, so the retained Connection remains the typed state owner.
    if unsafe { test_raw_state_take_is_rejected(file) } {
        return Err(RawSqliteFileStateRejection::TestStateTakeRejected);
    }
    let envelope = unsafe { installed_envelope(file)? };
    if !envelope.is::<State>() {
        return Err(RawSqliteFileStateRejection::TypeMismatch);
    }
    let file = NonNull::new(file.cast::<InertHandleBoundSqliteFile>())
        .expect("installed envelope requires a non-null file");
    // Disable the callback table before removing state ownership.
    // SAFETY: exact installation and exclusive take contracts passed above.
    let raw = unsafe {
        ptr::addr_of_mut!((*file.as_ptr()).base.pMethods).write(ptr::null());
        #[cfg(all(test, windows))]
        record_test_methods_clear(file.as_ptr().cast());
        ptr::addr_of_mut!((*file.as_ptr()).state).replace(ptr::null_mut())
    };
    // SAFETY: the raw pointer came from Box::into_raw in `install_state` and was cleared first.
    let envelope = unsafe { Box::from_raw(raw.cast::<RawSqliteFileStateEnvelope>()) };
    // SAFETY: the TypeId gate above proves the payload type.
    let state = unsafe { RawSqliteFileStateEnvelope::take_typed(envelope) };
    #[cfg(all(test, windows))]
    // SAFETY: the same exact allocation remains live and serialized after typed take succeeds.
    unsafe {
        record_test_state_take_success(file.as_ptr().cast());
    }
    Ok(state)
}

/// Captures a cloneable, redacted witness while the exact test VFS allocation is still live.
/// The clone owns only atomic counters and remains readable after SQLite releases the allocation.
///
/// # Safety
///
/// `file` must identify a live allocation initialized by this ABI module, and no callback may
/// overlap this observation.
#[cfg(all(test, windows))]
pub(in crate::node_agent_compute_plugin_host::local_authority) unsafe fn observe_test_vfs_file_raw_close_witness(
    file: *mut ffi::sqlite3_file,
) -> Option<HandleBoundSqliteAbiRawCloseWitness> {
    // Validate that this is the module's live installation before creating a sidecar. The borrow
    // ends before touching the independent control slot.
    unsafe { installed_envelope(file) }.ok()?;
    // SAFETY: the caller supplies the live serialized allocation described above.
    Some(unsafe { ensure_test_raw_close_control(file) }?.witness())
}

/// Arms an exact-allocation rejection before typed take or either raw-slot mutation.
/// The returned witness remains readable while the owning test Connection is retained.
///
/// # Safety
///
/// `file` must identify a live installed allocation initialized by this ABI module, and no callback
/// may overlap arming.
#[cfg(all(test, windows))]
pub(in crate::node_agent_compute_plugin_host::local_authority) unsafe fn arm_test_vfs_file_raw_state_take_rejection(
    file: *mut ffi::sqlite3_file,
) -> Option<HandleBoundSqliteAbiRawCloseWitness> {
    unsafe { installed_envelope(file) }.ok()?;
    // SAFETY: validation above and the caller's serialization contract cover the exact sidecar.
    unsafe { ensure_test_raw_close_control(file) }?.arm_state_take_rejection(file)
}

/// Records entry into the real raw xClose callback without changing its result or custody path.
#[cfg(all(test, windows))]
pub(super) unsafe fn record_test_raw_close_entry(file: *mut ffi::sqlite3_file) {
    // SAFETY: the xClose callback supplies exclusive access to its exact live allocation.
    if let Some(control) = unsafe { test_raw_close_control(file) } {
        control.record_raw_close_entry(file);
    }
}

#[cfg(all(test, windows))]
unsafe fn record_test_state_take_attempt(file: *mut ffi::sqlite3_file) {
    if let Some(control) = unsafe { test_raw_close_control(file) } {
        control.record_state_take_attempt(file);
    }
}

#[cfg(all(test, windows))]
unsafe fn record_test_methods_clear(file: *mut ffi::sqlite3_file) {
    if let Some(control) = unsafe { test_raw_close_control(file) } {
        control.record_methods_clear(file);
    }
}

#[cfg(all(test, windows))]
unsafe fn record_test_state_take_success(file: *mut ffi::sqlite3_file) {
    if let Some(control) = unsafe { test_raw_close_control(file) } {
        control.record_state_take_success(file);
    }
}

#[cfg(all(test, windows))]
unsafe fn test_raw_state_take_is_rejected(file: *mut ffi::sqlite3_file) -> bool {
    unsafe { test_raw_close_control(file) }.is_some_and(|control| control.rejects_state_take(file))
}

#[cfg(all(test, windows))]
pub(super) unsafe fn record_test_state_close_attempt(file: *mut ffi::sqlite3_file) {
    if let Some(control) = unsafe { test_raw_close_control(file) } {
        control.record_state_close_attempt(file);
    }
}

/// Releases the allocation-owned sidecar on an ordinary terminal path. An armed rejection never
/// reaches this path because it preserves both installed raw slots for process-lifetime custody.
#[cfg(all(test, windows))]
pub(super) unsafe fn release_test_raw_close_control_if_unretained(file: *mut ffi::sqlite3_file) {
    let Some(file) = NonNull::new(file.cast::<InertHandleBoundSqliteFile>()) else {
        return;
    };
    // SAFETY: callers serialize access to the exact initialized test allocation.
    let raw = unsafe { ptr::addr_of!((*file.as_ptr()).raw_close_control).read() };
    let Some(control) =
        NonNull::new(raw.cast::<close_control::HandleBoundSqliteAbiRawCloseControl>())
    else {
        return;
    };
    // A mismatched pointer cannot be safely reclaimed through this allocation.
    if !unsafe { control.as_ref() }.matches_allocation(file.as_ptr().cast()) {
        return;
    }
    // Clear the allocation slot before releasing its uniquely owned Box.
    unsafe {
        ptr::addr_of_mut!((*file.as_ptr()).raw_close_control).write(ptr::null_mut());
        drop(Box::from_raw(control.as_ptr()));
    }
}

/// Fail-closes an unexpectedly installed state without pretending physical xClose succeeded.
/// Payload Drop is expected to quarantine managed custody; the callback still returns an error.
///
/// # Safety
///
/// The file must be either uninstalled or installed by this module, with no overlapping access.
pub(super) unsafe fn abandon_installed_state(
    file: *mut ffi::sqlite3_file,
) -> Result<bool, RawSqliteFileStateRejection> {
    let file = NonNull::new(file.cast::<InertHandleBoundSqliteFile>())
        .ok_or(RawSqliteFileStateRejection::NullFile)?;
    // SAFETY: guaranteed by the initialized-file contract above.
    let (methods, state) = unsafe {
        (
            ptr::addr_of!((*file.as_ptr()).base.pMethods).read(),
            ptr::addr_of!((*file.as_ptr()).state).read(),
        )
    };
    if methods.is_null() && state.is_null() {
        #[cfg(all(test, windows))]
        // SAFETY: this is the same serialized initialized allocation. Retained custody is preserved.
        unsafe {
            release_test_raw_close_control_if_unretained(file.as_ptr().cast());
        }
        return Ok(false);
    }
    validate_installed(methods, state)?;
    #[cfg(all(test, windows))]
    // SAFETY: validation and the caller contract prove this exact allocation is serialized.
    if let Some(control) = unsafe { test_raw_close_control(file.as_ptr().cast()) } {
        control.record_state_abandon(file.as_ptr().cast());
    }
    // SAFETY: validation proves this module's exact table and a non-null envelope.
    unsafe {
        ptr::addr_of_mut!((*file.as_ptr()).base.pMethods).write(ptr::null());
        ptr::addr_of_mut!((*file.as_ptr()).state).write(ptr::null_mut());
        drop(Box::from_raw(state.cast::<RawSqliteFileStateEnvelope>()));
    }
    #[cfg(all(test, windows))]
    // SAFETY: raw state has reached an ordinary terminal abandonment path.
    unsafe {
        release_test_raw_close_control_if_unretained(file.as_ptr().cast());
    }
    Ok(true)
}

#[cfg(all(test, windows))]
unsafe fn ensure_test_raw_close_control<'file>(
    file: *mut ffi::sqlite3_file,
) -> Option<&'file mut close_control::HandleBoundSqliteAbiRawCloseControl> {
    let file = NonNull::new(file.cast::<InertHandleBoundSqliteFile>())?;
    // SAFETY: observation/arming owns serialized access to the initialized test allocation.
    let slot = unsafe { ptr::addr_of_mut!((*file.as_ptr()).raw_close_control) };
    let mut raw = unsafe { slot.read() };
    if raw.is_null() {
        raw = Box::into_raw(Box::new(
            close_control::HandleBoundSqliteAbiRawCloseControl::new(file.as_ptr().cast()),
        ))
        .cast();
        unsafe { slot.write(raw) };
    }
    let control = unsafe {
        raw.cast::<close_control::HandleBoundSqliteAbiRawCloseControl>()
            .as_mut()?
    };
    control
        .matches_allocation(file.as_ptr().cast())
        .then_some(control)
}

#[cfg(all(test, windows))]
unsafe fn test_raw_close_control<'file>(
    file: *mut ffi::sqlite3_file,
) -> Option<&'file mut close_control::HandleBoundSqliteAbiRawCloseControl> {
    let file = NonNull::new(file.cast::<InertHandleBoundSqliteFile>())?;
    // SAFETY: callers serialize access to this module's initialized test allocation.
    let raw = unsafe { ptr::addr_of!((*file.as_ptr()).raw_close_control).read() };
    let control = unsafe {
        raw.cast::<close_control::HandleBoundSqliteAbiRawCloseControl>()
            .as_mut()?
    };
    control
        .matches_allocation(file.as_ptr().cast())
        .then_some(control)
}

/// Confirms both the exact installed method table and the type-erased payload identity without
/// borrowing the payload itself. The caller must serialize this read with callback access.
#[cfg(test)]
pub(super) unsafe fn test_vfs_file_has_exact_installed_state<State: 'static>(
    file: *mut ffi::sqlite3_file,
) -> bool {
    match unsafe { installed_envelope(file) } {
        Ok(envelope) => envelope.is::<State>(),
        Err(_) => false,
    }
}

unsafe fn installed_envelope<'file>(
    file: *mut ffi::sqlite3_file,
) -> Result<&'file mut RawSqliteFileStateEnvelope, RawSqliteFileStateRejection> {
    let file = NonNull::new(file.cast::<InertHandleBoundSqliteFile>())
        .ok_or(RawSqliteFileStateRejection::NullFile)?;
    // SAFETY: guaranteed by the initialized-file contract of public callers.
    let (methods, state) = unsafe {
        (
            ptr::addr_of!((*file.as_ptr()).base.pMethods).read(),
            ptr::addr_of!((*file.as_ptr()).state).read(),
        )
    };
    validate_installed(methods, state)?;
    // SAFETY: the exact-method validation and module contract prove an envelope from install.
    Ok(unsafe {
        state
            .cast::<RawSqliteFileStateEnvelope>()
            .as_mut()
            .expect("validated state pointer is non-null")
    })
}

fn validate_installed(
    methods: *const ffi::sqlite3_io_methods,
    state: *mut c_void,
) -> Result<(), RawSqliteFileStateRejection> {
    if methods.is_null() {
        return Err(if state.is_null() {
            RawSqliteFileStateRejection::Uninstalled
        } else {
            RawSqliteFileStateRejection::ForeignMethods
        });
    }
    if !ptr::eq(methods, &INERT_IO_METHODS) {
        return Err(RawSqliteFileStateRejection::ForeignMethods);
    }
    if state.is_null() {
        return Err(RawSqliteFileStateRejection::StateMissing);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
