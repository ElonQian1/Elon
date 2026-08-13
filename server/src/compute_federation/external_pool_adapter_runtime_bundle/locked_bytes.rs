use std::{fs::File, io::Read, ptr::NonNull, sync::atomic};

use zeroize::Zeroize;

use super::types::ExternalPoolAdapterRuntimeBundleError;

pub(super) struct LockedSensitiveBytes {
    pointer: NonNull<u8>,
    length: usize,
}

impl LockedSensitiveBytes {
    pub(super) fn read_exact(
        file: &mut File,
        length: u64,
    ) -> Result<Self, ExternalPoolAdapterRuntimeBundleError> {
        let length = usize::try_from(length)
            .map_err(|_| ExternalPoolAdapterRuntimeBundleError::ContentDrift)?;
        if length == 0 {
            return Err(ExternalPoolAdapterRuntimeBundleError::ContentDrift);
        }
        let mut custody = Self::allocate(length)?;
        file.read_exact(custody.as_mut_slice())
            .map_err(|_| ExternalPoolAdapterRuntimeBundleError::ContentDrift)?;
        let mut extra = [0_u8; 1];
        let extra_read = file.read(&mut extra);
        extra.zeroize();
        if extra_read.map_err(|_| ExternalPoolAdapterRuntimeBundleError::ContentDrift)? != 0 {
            return Err(ExternalPoolAdapterRuntimeBundleError::ContentDrift);
        }
        Ok(custody)
    }

    pub(super) fn as_slice(&self) -> &[u8] {
        // SAFETY: allocation is retained exclusively by self for exactly length bytes.
        unsafe { std::slice::from_raw_parts(self.pointer.as_ptr(), self.length) }
    }

    fn as_mut_slice(&mut self) -> &mut [u8] {
        // SAFETY: allocation is retained exclusively by self for exactly length bytes.
        unsafe { std::slice::from_raw_parts_mut(self.pointer.as_ptr(), self.length) }
    }

    #[cfg(target_os = "linux")]
    fn allocate(length: usize) -> Result<Self, ExternalPoolAdapterRuntimeBundleError> {
        let raw = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                length,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        if raw == libc::MAP_FAILED {
            return Err(ExternalPoolAdapterRuntimeBundleError::MemoryCustodyUnavailable);
        }
        let pointer = NonNull::new(raw.cast::<u8>()).ok_or_else(|| {
            unsafe { libc::munmap(raw, length) };
            ExternalPoolAdapterRuntimeBundleError::MemoryCustodyUnavailable
        })?;
        if unsafe { libc::mlock(raw, length) } != 0 {
            unsafe { libc::munmap(raw, length) };
            return Err(ExternalPoolAdapterRuntimeBundleError::MemoryCustodyUnavailable);
        }
        if unsafe { libc::madvise(raw, length, libc::MADV_DONTDUMP) } != 0 {
            unsafe {
                libc::munlock(raw, length);
                libc::munmap(raw, length);
            }
            return Err(ExternalPoolAdapterRuntimeBundleError::MemoryCustodyUnavailable);
        }
        Ok(Self { pointer, length })
    }

    #[cfg(windows)]
    fn allocate(length: usize) -> Result<Self, ExternalPoolAdapterRuntimeBundleError> {
        use windows_sys::Win32::System::Memory::{
            VirtualAlloc, VirtualFree, VirtualLock, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE,
            PAGE_READWRITE,
        };
        let raw = unsafe {
            VirtualAlloc(
                std::ptr::null(),
                length,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        let Some(pointer) = NonNull::new(raw.cast::<u8>()) else {
            return Err(ExternalPoolAdapterRuntimeBundleError::MemoryCustodyUnavailable);
        };
        if unsafe { VirtualLock(raw, length) } == 0 {
            unsafe { VirtualFree(raw, 0, MEM_RELEASE) };
            return Err(ExternalPoolAdapterRuntimeBundleError::MemoryCustodyUnavailable);
        }
        Ok(Self { pointer, length })
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    fn allocate(_length: usize) -> Result<Self, ExternalPoolAdapterRuntimeBundleError> {
        Err(ExternalPoolAdapterRuntimeBundleError::MemoryCustodyUnavailable)
    }
}

impl Drop for LockedSensitiveBytes {
    fn drop(&mut self) {
        self.as_mut_slice().zeroize();
        atomic::compiler_fence(atomic::Ordering::SeqCst);
        self.release();
    }
}

impl LockedSensitiveBytes {
    #[cfg(target_os = "linux")]
    fn release(&mut self) {
        unsafe {
            libc::munlock(self.pointer.as_ptr().cast(), self.length);
            libc::munmap(self.pointer.as_ptr().cast(), self.length);
        }
    }

    #[cfg(windows)]
    fn release(&mut self) {
        use windows_sys::Win32::System::Memory::{VirtualFree, VirtualUnlock, MEM_RELEASE};
        unsafe {
            VirtualUnlock(self.pointer.as_ptr().cast(), self.length);
            VirtualFree(self.pointer.as_ptr().cast(), 0, MEM_RELEASE);
        }
    }

    #[cfg(not(any(target_os = "linux", windows)))]
    fn release(&mut self) {}
}
