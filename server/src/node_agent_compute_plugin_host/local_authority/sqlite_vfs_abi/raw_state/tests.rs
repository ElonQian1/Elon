use std::{
    mem::MaybeUninit,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use super::super::result_codes;
use super::*;

#[derive(Debug)]
struct DropProbe(Arc<AtomicUsize>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn fresh_file() -> (
    Box<MaybeUninit<InertHandleBoundSqliteFile>>,
    *mut ffi::sqlite3_file,
) {
    let mut storage = Box::new(MaybeUninit::<InertHandleBoundSqliteFile>::uninit());
    let file = storage.as_mut_ptr().cast::<ffi::sqlite3_file>();
    // SAFETY: this is fresh, aligned storage with the exact published file layout.
    assert!(unsafe { initialize_fresh_file(file) });
    (storage, file)
}

#[test]
fn typed_install_borrow_and_take_clear_the_exact_slot_once() {
    let (_storage, file) = fresh_file();
    // SAFETY: fresh_file initialized the allocation and this test serializes all access.
    unsafe { install_state(file, 41_u64) }.expect("install exact state");
    // SAFETY: this test owns and serializes the installed allocation.
    let value = unsafe {
        with_installed_state::<u64, _>(file, |value| {
            *value += 1;
            *value
        })
    }
    .expect("borrow exact state");
    assert_eq!(value, 42);
    // SAFETY: mismatch is inspected without changing the serialized allocation.
    assert_eq!(
        unsafe { with_installed_state::<String, _>(file, |_| ()) },
        Err(RawSqliteFileStateRejection::TypeMismatch)
    );
    // SAFETY: mismatch is inspected without changing the serialized allocation.
    assert!(matches!(
        unsafe { take_installed_state::<String>(file) },
        Err(RawSqliteFileStateRejection::TypeMismatch)
    ));
    // SAFETY: no borrow overlaps this exact typed take.
    let state = unsafe { take_installed_state::<u64>(file) }.expect("take exact state");
    assert_eq!(*state, 42);
    drop(state);
    // SAFETY: the file remains initialized and serialized after take.
    assert!(matches!(
        unsafe { take_installed_state::<u64>(file) },
        Err(RawSqliteFileStateRejection::Uninstalled)
    ));
}

#[test]
fn occupied_install_returns_new_state_without_replacing_original() {
    let (_storage, file) = fresh_file();
    // SAFETY: fresh_file initialized the allocation and this test serializes all access.
    unsafe { install_state(file, 7_u32) }.expect("install original state");
    // SAFETY: the occupied check reads only initialized serialized fields.
    let (rejection, returned) = unsafe { install_state(file, String::from("returned")) }
        .expect_err("occupied slot must reject second state");
    assert_eq!(rejection, RawSqliteFileStateRejection::Occupied);
    assert_eq!(returned, "returned");
    // SAFETY: the original type and state remain installed and no borrow overlaps the take.
    let original = unsafe { take_installed_state::<u32>(file) }.expect("take original state");
    assert_eq!(*original, 7);
}

#[test]
fn inert_close_abandons_unexpected_state_once_and_clears_callbacks() {
    let (storage, file) = fresh_file();
    let drops = Arc::new(AtomicUsize::new(0));
    // SAFETY: fresh_file initialized the allocation and this test serializes all access.
    unsafe { install_state(file, DropProbe(Arc::clone(&drops))) }.expect("install drop probe");
    // SAFETY: this invokes the exact callback over the initialized allocation.
    assert_eq!(
        unsafe { super::super::io_core::close(file) },
        result_codes::CLOSE_UNAVAILABLE
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    // SAFETY: storage was initialized and remains alive for these field reads.
    let file_state = unsafe { storage.as_ref().assume_init_ref() };
    assert!(file_state.base.pMethods.is_null());
    assert!(file_state.state.is_null());
    // SAFETY: a second direct call sees an uninstalled state and cannot drop twice.
    assert_eq!(
        unsafe { super::super::io_core::close(file) },
        result_codes::CLOSE_UNAVAILABLE
    );
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn type_erased_abandonment_runs_payload_drop_without_typed_access() {
    let (_storage, file) = fresh_file();
    let drops = Arc::new(AtomicUsize::new(0));
    // SAFETY: fresh_file initialized the allocation and this test serializes all access.
    unsafe { install_state(file, DropProbe(Arc::clone(&drops))) }.expect("install drop probe");
    // SAFETY: no borrow overlaps this exact type-erased abandonment.
    assert!(unsafe { abandon_installed_state(file) }.expect("abandon installed state"));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    // SAFETY: the file remains initialized and serialized after abandonment.
    assert!(!unsafe { abandon_installed_state(file) }.expect("observe uninstalled state"));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}
