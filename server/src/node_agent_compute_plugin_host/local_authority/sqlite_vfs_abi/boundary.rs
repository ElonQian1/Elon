use std::{
    os::raw::{c_int, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
};

use rusqlite::ffi;

/// Prevents a Rust unwind from crossing a C callback boundary.
pub(super) fn catch_code(fallback: c_int, operation: impl FnOnce() -> c_int) -> c_int {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(fallback)
}

/// Variant for callbacks whose return value is not an SQLite result code.
pub(super) fn catch_value<T: Copy>(fallback: T, operation: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(operation)).unwrap_or(fallback)
}

/// # Safety
///
/// A non-null output pointer must be valid and aligned for one `c_int` write.
pub(super) unsafe fn write_int_zero(output: *mut c_int) {
    if !output.is_null() {
        // SAFETY: guaranteed by the callback contract above.
        unsafe { output.write(0) };
    }
}

/// # Safety
///
/// A non-null output pointer must be valid and aligned for one SQLite integer write.
pub(super) unsafe fn write_i64_zero(output: *mut ffi::sqlite3_int64) {
    if !output.is_null() {
        // SAFETY: guaranteed by the callback contract above.
        unsafe { output.write(0) };
    }
}

/// # Safety
///
/// A non-null output pointer must be valid and aligned for one pointer write.
pub(super) unsafe fn write_pointer_null(output: *mut *mut c_void) {
    if !output.is_null() {
        // SAFETY: guaranteed by the callback contract above.
        unsafe { output.write(ptr::null_mut()) };
    }
}

/// # Safety
///
/// A non-null output pointer must be valid and aligned for one `f64` write.
pub(super) unsafe fn write_f64_zero(output: *mut f64) {
    if !output.is_null() {
        // SAFETY: guaranteed by the callback contract above.
        unsafe { output.write(0.0) };
    }
}

/// Zeroes a callback-owned byte range without constructing a Rust slice from an invalid C input.
/// Returns false for a negative amount or for a null non-empty buffer.
///
/// # Safety
///
/// When `amount` is positive, `output` must be valid for exactly that many writable bytes.
pub(super) unsafe fn zero_bytes(output: *mut c_void, amount: c_int) -> bool {
    if amount < 0 {
        return false;
    }
    if amount == 0 {
        return true;
    }
    if output.is_null() {
        return false;
    }
    // SAFETY: guaranteed by the callback contract above after the checked conversion.
    unsafe { output.cast::<u8>().write_bytes(0, amount as usize) };
    true
}
