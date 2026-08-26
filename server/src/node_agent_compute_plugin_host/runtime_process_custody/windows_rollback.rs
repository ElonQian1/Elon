use std::{
    mem::{size_of, zeroed, ManuallyDrop},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle},
};

use windows_sys::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0},
    System::{
        JobObjects::{
            JobObjectBasicAccountingInformation, QueryInformationJobObject, TerminateJobObject,
            JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
        },
        Threading::{TerminateProcess, WaitForSingleObject, PROCESS_INFORMATION},
    },
};

use super::{
    model::{
        ComputePluginRunnerProcessIdentity, PreparedComputePluginRunnerProcess,
        ValidatedWindowsRunnerProcessPreparation, WindowsRunnerLiveProcessCustody,
    },
    namespace_query::LoaderCurrentWindowsRunnerProcessPreparation,
    windows_job::ConfiguredRunnerJob,
};

pub(super) const ROLLBACK_EXIT_CODE: u32 = 0xE10C_7101;
pub(super) const ROLLBACK_WAIT_MS: u32 = 5_000;

pub(super) struct SuspendedProcessRollback {
    job: Option<ConfiguredRunnerJob>,
    process: Option<OwnedHandle>,
    primary_thread: Option<OwnedHandle>,
    reported_process_id: u32,
    reported_primary_thread_id: u32,
    armed: bool,
}

/// First owner created after successful `CreateProcessAsUserW`. It couples OS rollback handles to
/// loader currentness, namespace grants, content leases, and launch security before any validation
/// can fail or unwind. Drop releases the authority graph only after termination is confirmed.
pub(super) struct WindowsRunnerPostCreateCustody<'root> {
    rollback: ManuallyDrop<SuspendedProcessRollback>,
    preparation: ManuallyDrop<LoaderCurrentWindowsRunnerProcessPreparation<'root>>,
}

impl<'root> WindowsRunnerPostCreateCustody<'root> {
    pub(super) fn new(
        rollback: SuspendedProcessRollback,
        preparation: LoaderCurrentWindowsRunnerProcessPreparation<'root>,
    ) -> Self {
        Self {
            rollback: ManuallyDrop::new(rollback),
            preparation: ManuallyDrop::new(preparation),
        }
    }

    pub(super) fn rollback(&self) -> &SuspendedProcessRollback {
        &self.rollback
    }

    pub(super) fn rollback_mut(&mut self) -> &mut SuspendedProcessRollback {
        &mut self.rollback
    }

    pub(super) fn preparation(&self) -> &LoaderCurrentWindowsRunnerProcessPreparation<'root> {
        &self.preparation
    }

    pub(super) fn into_parts(
        self,
    ) -> (
        SuspendedProcessRollback,
        LoaderCurrentWindowsRunnerProcessPreparation<'root>,
    ) {
        let mut this = ManuallyDrop::new(self);
        // SAFETY: `this` will not run Drop, and each field is taken exactly once.
        unsafe {
            (
                ManuallyDrop::take(&mut this.rollback),
                ManuallyDrop::take(&mut this.preparation),
            )
        }
    }

    /// Single consuming success conversion. Until the final ManuallyDrop guard is already built,
    /// every OS handle and loader authority remains inside either `self` or a reconstituted
    /// post-create custody; no allocation, clone, or panicking extraction is performed.
    pub(super) fn into_prepared_process(
        self,
        identity: ComputePluginRunnerProcessIdentity,
    ) -> Result<PreparedComputePluginRunnerProcess<'root>, Self> {
        let (rollback, loader_current) = self.into_parts();
        let (job, process, primary_thread) = match rollback.into_handles_if_complete() {
            Ok(handles) => handles,
            Err(rollback) => return Err(Self::new(rollback, loader_current)),
        };
        let LoaderCurrentWindowsRunnerProcessPreparation {
            preparation,
            currentness: loader_currentness,
        } = loader_current;
        let ValidatedWindowsRunnerProcessPreparation {
            loader_locked,
            launch_security,
            policy: _,
            application_path: _,
            working_directory_path: _,
        } = preparation;
        Ok(PreparedComputePluginRunnerProcess {
            custody: ManuallyDrop::new(WindowsRunnerLiveProcessCustody {
                job,
                process,
                primary_thread,
                launch_security,
                loader_locked,
                identity,
                loader_currentness,
            }),
        })
    }
}

impl Drop for WindowsRunnerPostCreateCustody<'_> {
    fn drop(&mut self) {
        if self.rollback_mut().terminate_and_confirm() {
            // SAFETY: termination is confirmed. These are the sole explicit drops, ordered so OS
            // process/Job handles close before loader authority and immutable content leases.
            unsafe {
                ManuallyDrop::drop(&mut self.rollback);
                ManuallyDrop::drop(&mut self.preparation);
            }
        }
    }
}

impl SuspendedProcessRollback {
    /// Takes ownership of each distinct non-null handle returned by CreateProcessAsUserW.
    pub(super) unsafe fn from_created(
        job: ConfiguredRunnerJob,
        information: PROCESS_INFORMATION,
    ) -> Self {
        let handles_alias =
            !information.hProcess.is_null() && information.hProcess == information.hThread;
        Self {
            job: Some(job),
            process: (!information.hProcess.is_null()).then(|| unsafe {
                OwnedHandle::from_raw_handle(information.hProcess as RawHandle)
            }),
            primary_thread: (!information.hThread.is_null() && !handles_alias)
                .then(|| unsafe { OwnedHandle::from_raw_handle(information.hThread as RawHandle) }),
            reported_process_id: information.dwProcessId,
            reported_primary_thread_id: information.dwThreadId,
            armed: true,
        }
    }

    pub(super) fn has_complete_process_information(&self) -> bool {
        self.process.is_some() && self.primary_thread.is_some()
    }

    pub(super) fn job_raw(&self) -> HANDLE {
        self.job
            .as_ref()
            .expect("job custody retained")
            .raw_handle()
    }

    pub(super) fn process_raw(&self) -> HANDLE {
        owned_raw(self.process.as_ref().expect("process custody retained"))
    }

    pub(super) fn primary_thread_raw(&self) -> HANDLE {
        owned_raw(
            self.primary_thread
                .as_ref()
                .expect("primary thread custody retained"),
        )
    }

    pub(super) fn reported_process_id(&self) -> u32 {
        self.reported_process_id
    }

    pub(super) fn reported_primary_thread_id(&self) -> u32 {
        self.reported_primary_thread_id
    }

    pub(super) fn terminate_and_confirm(&mut self) -> bool {
        let Some(process) = self.process.as_ref() else {
            return false;
        };
        let confirmed = terminate_and_confirm_raw(
            self.job
                .as_ref()
                .expect("job custody retained")
                .raw_handle(),
            process,
        );
        if confirmed {
            self.armed = false;
        }
        confirmed
    }

    fn into_handles_if_complete(mut self) -> Result<(OwnedHandle, OwnedHandle, OwnedHandle), Self> {
        if !self.has_complete_process_information() || self.job.is_none() {
            return Err(self);
        }
        let parts = (
            self.job.take(),
            self.process.take(),
            self.primary_thread.take(),
        );
        match parts {
            (Some(job), Some(process), Some(primary_thread)) => {
                self.armed = false;
                Ok((job.into_handle(), process, primary_thread))
            }
            (job, process, primary_thread) => {
                self.job = job;
                self.process = process;
                self.primary_thread = primary_thread;
                Err(self)
            }
        }
    }
}

/// Attempts Job-wide termination plus a direct-process fallback. Confirmation requires both the
/// root process and Job to signal and an exact accounting query to report zero active processes.
/// Callers must retain every related owner when this returns false.
pub(super) fn terminate_and_confirm_owned(job: &OwnedHandle, process: &OwnedHandle) -> bool {
    terminate_and_confirm_raw(owned_raw(job), process)
}

fn terminate_and_confirm_raw(job: HANDLE, process: &OwnedHandle) -> bool {
    let process = owned_raw(process);
    unsafe { TerminateJobObject(job, ROLLBACK_EXIT_CODE) };
    unsafe { TerminateProcess(process, ROLLBACK_EXIT_CODE) };
    let process_signaled =
        (unsafe { WaitForSingleObject(process, ROLLBACK_WAIT_MS) }) == WAIT_OBJECT_0;
    let job_signaled = (unsafe { WaitForSingleObject(job, ROLLBACK_WAIT_MS) }) == WAIT_OBJECT_0;
    process_signaled && job_signaled && job_has_no_active_processes(job)
}

fn job_has_no_active_processes(job: HANDLE) -> bool {
    // SAFETY: zero is valid output storage for the fixed-size accounting query.
    let mut accounting = unsafe { zeroed::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() };
    let mut returned = 0_u32;
    (unsafe {
        QueryInformationJobObject(
            job,
            JobObjectBasicAccountingInformation,
            (&mut accounting as *mut JOBOBJECT_BASIC_ACCOUNTING_INFORMATION).cast(),
            size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
            &mut returned,
        )
    }) != 0
        && returned == size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32
        && accounting.ActiveProcesses == 0
}

impl Drop for SuspendedProcessRollback {
    fn drop(&mut self) {
        if self.armed && !self.terminate_and_confirm() {
            // An unconfirmed suspended child must not become an orphan merely because recovery
            // custody was dropped. Leak the exact OS handles fail-closed so the Job (if assigned),
            // process, and suspended primary thread remain held for process-level recovery. A
            // future explicit recovery service must replace this deliberate parking behavior.
            if let Some(handle) = self.primary_thread.take() {
                std::mem::forget(handle);
            }
            if let Some(handle) = self.process.take() {
                std::mem::forget(handle);
            }
            if let Some(handle) = self.job.take() {
                std::mem::forget(handle);
            }
        }
    }
}

fn owned_raw(handle: &OwnedHandle) -> HANDLE {
    handle.as_raw_handle() as HANDLE
}
