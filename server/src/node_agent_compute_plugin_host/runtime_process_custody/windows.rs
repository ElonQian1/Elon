use std::{
    error::Error as StdError,
    fmt,
    mem::zeroed,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle},
};

use anyhow::{anyhow, Error, Result};
use windows_sys::Win32::{
    Foundation::{FILETIME, HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::{
        JobObjects::{IsProcessInJob, TerminateJobObject},
        Threading::{
            CreateProcessAsUserW, GetProcessId, GetProcessTimes, GetThreadId, TerminateProcess,
            WaitForSingleObject, CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
            EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION,
        },
    },
};

use super::{
    encoding::{command_line, empty_environment_block, nul_terminated_path},
    model::{
        ComputePluginRunnerProcessIdentity, PreparedComputePluginRunnerProcess,
        ValidatedWindowsRunnerProcessPreparation,
    },
    windows_job::{ConfiguredRunnerJob, ConfiguredRunnerJobFailurePhase},
};

const ROLLBACK_EXIT_CODE: u32 = 0xE10C_7101;
const ROLLBACK_WAIT_MS: u32 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsRunnerProcessPrepareFailurePhase {
    InputEncoding,
    LaunchSecurity,
    JobCreation,
    JobConfiguration,
    JobAttributeList,
    ProcessCreation,
    ProcessHandleContract,
    JobMembership,
    ProcessIdentity,
    SuspendedLiveness,
}

/// Every failure retains the consumed preparation. If termination cannot be confirmed, it also
/// retains all live OS handles until this error is dropped; it never returns a retry permit.
pub(super) struct WindowsRunnerProcessPrepareFailure<'root> {
    phase: WindowsRunnerProcessPrepareFailurePhase,
    error: Error,
    uncertain_process: Option<SuspendedProcessRollback>,
    _preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
}

pub(super) fn prepare_suspended_windows_runner_process<'root>(
    preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
) -> std::result::Result<
    PreparedComputePluginRunnerProcess<'root>,
    WindowsRunnerProcessPrepareFailure<'root>,
> {
    let application = match nul_terminated_path(&preparation.image.absolute_path) {
        Ok(value) => value,
        Err(error) => return Err(before_create_failure(InputEncoding, error, preparation)),
    };
    let mut command = match command_line(
        &preparation.image.absolute_path,
        &preparation.policy.arguments,
    ) {
        Ok(value) => value,
        Err(error) => return Err(before_create_failure(InputEncoding, error, preparation)),
    };
    let current_directory = match nul_terminated_path(&preparation.image.working_directory_path) {
        Ok(value) => value,
        Err(error) => return Err(before_create_failure(InputEncoding, error, preparation)),
    };
    let environment = empty_environment_block();
    let launch_security_validation = preparation.launch_security.validate();
    if let Err(error) = launch_security_validation {
        return Err(before_create_failure(LaunchSecurity, error, preparation));
    }

    let job = match ConfiguredRunnerJob::create(&preparation.policy) {
        Ok(value) => value,
        Err(failure) => {
            let phase = match failure.phase {
                ConfiguredRunnerJobFailurePhase::Creation => JobCreation,
                ConfiguredRunnerJobFailurePhase::Configuration => JobConfiguration,
                ConfiguredRunnerJobFailurePhase::AttributeList => JobAttributeList,
            };
            return Err(before_create_failure(phase, failure.error, preparation));
        }
    };

    let startup = job.startup_info();
    // SAFETY: CreateProcessAsUserW initializes this record on success.
    let mut process_information = unsafe { zeroed::<PROCESS_INFORMATION>() };
    let create_security = preparation.launch_security.for_create();
    let created = unsafe {
        CreateProcessAsUserW(
            create_security.primary_token,
            application.as_ptr(),
            command.as_mut_ptr(),
            &create_security.process_attributes,
            &create_security.thread_attributes,
            0,
            CREATE_SUSPENDED
                | CREATE_UNICODE_ENVIRONMENT
                | CREATE_NO_WINDOW
                | EXTENDED_STARTUPINFO_PRESENT,
            environment.as_ptr().cast(),
            current_directory.as_ptr(),
            startup.as_ptr(),
            &mut process_information,
        )
    };
    let create_error = (created == 0).then(std::io::Error::last_os_error);
    drop(create_security);
    drop(startup);
    if let Some(error) = create_error {
        return Err(before_create_failure(
            ProcessCreation,
            error.into(),
            preparation,
        ));
    }
    let job = job.into_handle();
    // SAFETY: every distinct non-null handle returned by successful CreateProcessAsUserW is newly
    // owned. The
    // rollback guard wraps each such handle exactly once, including the defensive contract-broken
    // branch, so an unconfirmed child never becomes a scalar-only error.
    let mut rollback = unsafe { SuspendedProcessRollback::from_created(job, process_information) };
    if !rollback.has_complete_process_information() {
        return Err(after_create_failure(
            ProcessHandleContract,
            anyhow!("COMPUTE_PLUGIN_WINDOWS_PROCESS_HANDLE_CONTRACT_BROKEN"),
            preparation,
            rollback,
        ));
    }

    let mut in_job = 0;
    if unsafe { IsProcessInJob(rollback.process_raw(), rollback.job_raw(), &mut in_job) } == 0 {
        let error = std::io::Error::last_os_error();
        return Err(after_create_failure(
            JobMembership,
            error.into(),
            preparation,
            rollback,
        ));
    }
    if in_job == 0 {
        return Err(after_create_failure(
            JobMembership,
            anyhow!("COMPUTE_PLUGIN_WINDOWS_JOB_MEMBERSHIP_MISSING"),
            preparation,
            rollback,
        ));
    }

    let process_id = unsafe { GetProcessId(rollback.process_raw()) };
    if process_id == 0 || process_id != process_information.dwProcessId {
        let error = if process_id == 0 {
            std::io::Error::last_os_error().into()
        } else {
            anyhow!("COMPUTE_PLUGIN_WINDOWS_PROCESS_ID_CHANGED")
        };
        return Err(after_create_failure(
            ProcessIdentity,
            error,
            preparation,
            rollback,
        ));
    }
    let thread_id = unsafe { GetThreadId(rollback.primary_thread_raw()) };
    if thread_id == 0 || thread_id != process_information.dwThreadId {
        let error = if thread_id == 0 {
            std::io::Error::last_os_error().into()
        } else {
            anyhow!("COMPUTE_PLUGIN_WINDOWS_PRIMARY_THREAD_ID_CHANGED")
        };
        return Err(after_create_failure(
            ProcessIdentity,
            error,
            preparation,
            rollback,
        ));
    }
    let creation_filetime = match process_creation_filetime(rollback.process_raw()) {
        Ok(value) if value != 0 => value,
        Ok(_) => {
            return Err(after_create_failure(
                ProcessIdentity,
                anyhow!("COMPUTE_PLUGIN_WINDOWS_PROCESS_CREATION_TIME_MISSING"),
                preparation,
                rollback,
            ))
        }
        Err(error) => {
            return Err(after_create_failure(
                ProcessIdentity,
                error,
                preparation,
                rollback,
            ))
        }
    };
    match unsafe { WaitForSingleObject(rollback.process_raw(), 0) } {
        WAIT_TIMEOUT => {}
        WAIT_OBJECT_0 => {
            return Err(after_create_failure(
                SuspendedLiveness,
                anyhow!("COMPUTE_PLUGIN_WINDOWS_SUSPENDED_PROCESS_NOT_LIVE"),
                preparation,
                rollback,
            ))
        }
        WAIT_FAILED => {
            let error = std::io::Error::last_os_error();
            return Err(after_create_failure(
                SuspendedLiveness,
                error.into(),
                preparation,
                rollback,
            ));
        }
        status => {
            return Err(after_create_failure(
                SuspendedLiveness,
                anyhow!("COMPUTE_PLUGIN_WINDOWS_PROCESS_WAIT_CHANGED:{status}"),
                preparation,
                rollback,
            ))
        }
    }

    let (job, process, primary_thread) = rollback.into_handles();
    let ValidatedWindowsRunnerProcessPreparation {
        admitted,
        image,
        launch_security,
        policy,
    } = preparation;
    Ok(PreparedComputePluginRunnerProcess {
        job,
        process,
        primary_thread,
        launch_security,
        runner_image: image,
        admitted,
        identity: ComputePluginRunnerProcessIdentity {
            process_id,
            primary_thread_id: thread_id,
            creation_filetime,
        },
        start_material_digest: policy.start_material_digest,
    })
}

fn process_creation_filetime(process: HANDLE) -> Result<u64> {
    // SAFETY: zero is valid storage for the four FILETIME outputs.
    let mut creation = unsafe { zeroed::<FILETIME>() };
    let mut exit = unsafe { zeroed::<FILETIME>() };
    let mut kernel = unsafe { zeroed::<FILETIME>() };
    let mut user = unsafe { zeroed::<FILETIME>() };
    if unsafe { GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok((u64::from(creation.dwHighDateTime) << 32) | u64::from(creation.dwLowDateTime))
}

fn before_create_failure<'root>(
    phase: WindowsRunnerProcessPrepareFailurePhase,
    error: Error,
    preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
) -> WindowsRunnerProcessPrepareFailure<'root> {
    WindowsRunnerProcessPrepareFailure {
        phase,
        error,
        uncertain_process: None,
        _preparation: preparation,
    }
}

fn after_create_failure<'root>(
    phase: WindowsRunnerProcessPrepareFailurePhase,
    error: Error,
    preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
    mut rollback: SuspendedProcessRollback,
) -> WindowsRunnerProcessPrepareFailure<'root> {
    let uncertain_process = if rollback.terminate_and_confirm() {
        None
    } else {
        Some(rollback)
    };
    WindowsRunnerProcessPrepareFailure {
        phase,
        error,
        uncertain_process,
        _preparation: preparation,
    }
}

struct SuspendedProcessRollback {
    job: Option<OwnedHandle>,
    process: Option<OwnedHandle>,
    primary_thread: Option<OwnedHandle>,
    armed: bool,
}

impl SuspendedProcessRollback {
    /// Takes ownership of each distinct non-null handle returned by CreateProcessAsUserW.
    unsafe fn from_created(job: OwnedHandle, information: PROCESS_INFORMATION) -> Self {
        let handles_alias =
            !information.hProcess.is_null() && information.hProcess == information.hThread;
        Self {
            job: Some(job),
            process: (!information.hProcess.is_null()).then(|| unsafe {
                OwnedHandle::from_raw_handle(information.hProcess as RawHandle)
            }),
            primary_thread: (!information.hThread.is_null() && !handles_alias)
                .then(|| unsafe { OwnedHandle::from_raw_handle(information.hThread as RawHandle) }),
            armed: true,
        }
    }

    fn has_complete_process_information(&self) -> bool {
        self.process.is_some() && self.primary_thread.is_some()
    }

    fn job_raw(&self) -> HANDLE {
        owned_raw(self.job.as_ref().expect("job custody retained"))
    }

    fn process_raw(&self) -> HANDLE {
        owned_raw(self.process.as_ref().expect("process custody retained"))
    }

    fn primary_thread_raw(&self) -> HANDLE {
        owned_raw(
            self.primary_thread
                .as_ref()
                .expect("primary thread custody retained"),
        )
    }

    fn terminate_and_confirm(&mut self) -> bool {
        let process = self.process.as_ref().map(owned_raw);
        if let Some(process) = process {
            if unsafe { WaitForSingleObject(process, 0) } == WAIT_OBJECT_0 {
                self.armed = false;
                return true;
            }
        }
        unsafe { TerminateJobObject(self.job_raw(), ROLLBACK_EXIT_CODE) };
        let Some(process) = process else {
            return false;
        };
        unsafe { TerminateProcess(process, ROLLBACK_EXIT_CODE) };
        let confirmed = unsafe { WaitForSingleObject(process, ROLLBACK_WAIT_MS) } == WAIT_OBJECT_0;
        if confirmed {
            self.armed = false;
        }
        confirmed
    }

    fn into_handles(mut self) -> (OwnedHandle, OwnedHandle, OwnedHandle) {
        self.armed = false;
        (
            self.job.take().expect("job custody retained"),
            self.process.take().expect("process custody retained"),
            self.primary_thread
                .take()
                .expect("primary thread custody retained"),
        )
    }
}

impl Drop for SuspendedProcessRollback {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.terminate_and_confirm();
        }
    }
}

impl Drop for PreparedComputePluginRunnerProcess<'_> {
    fn drop(&mut self) {
        unsafe { TerminateJobObject(owned_raw(&self.job), ROLLBACK_EXIT_CODE) };
        unsafe { WaitForSingleObject(owned_raw(&self.process), ROLLBACK_WAIT_MS) };
    }
}

impl fmt::Debug for WindowsRunnerProcessPrepareFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("WindowsRunnerProcessPrepareFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("preparation", &"<retained-linear-custody>")
            .field("outcome_uncertain", &self.uncertain_process.is_some())
            .finish()
    }
}

impl fmt::Display for WindowsRunnerProcessPrepareFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for WindowsRunnerProcessPrepareFailure<'_> {}

fn owned_raw(handle: &OwnedHandle) -> HANDLE {
    handle.as_raw_handle() as HANDLE
}

use WindowsRunnerProcessPrepareFailurePhase::*;
