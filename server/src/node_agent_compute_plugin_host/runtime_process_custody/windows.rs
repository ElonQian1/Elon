use std::{
    error::Error as StdError,
    fmt,
    mem::{zeroed, ManuallyDrop},
};

use anyhow::{anyhow, Error, Result};
use windows_sys::Win32::{
    Foundation::{FILETIME, HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::{
        JobObjects::IsProcessInJob,
        Threading::{
            CreateProcessAsUserW, GetProcessId, GetProcessTimes, GetThreadId, WaitForSingleObject,
            CREATE_NO_WINDOW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT,
            EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION,
        },
    },
};

use super::{
    encoding::{
        command_line, empty_environment_block, nul_terminated_current_directory,
        nul_terminated_path,
    },
    model::{
        ComputePluginRunnerProcessIdentity, PreparedComputePluginRunnerProcess,
        ValidatedWindowsRunnerProcessPreparation,
    },
    namespace_query::{
        LoaderCurrentWindowsRunnerProcessPreparation,
        WindowsRunnerPreCreateLoaderCurrentnessBackend,
        WindowsRunnerPreCreateLoaderCurrentnessFailureClass,
        WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody,
    },
    windows_job::{ConfiguredRunnerJob, ConfiguredRunnerJobFailurePhase},
    windows_rollback::{
        terminate_and_confirm_owned, SuspendedProcessRollback, WindowsRunnerPostCreateCustody,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WindowsRunnerProcessPrepareFailurePhase {
    InputEncoding,
    LaunchSecurity,
    JobCreation,
    JobConfiguration,
    JobAttributeList,
    LoaderCurrentness,
    ProcessCreation,
    ProcessHandleContract,
    JobMembership,
    ProcessIdentity,
    SuspendedLiveness,
}

/// Every failure retains the consumed preparation. If termination cannot be confirmed, the exact
/// process handles, loader authority, content leases, namespace grants, and recovery key are
/// parked together rather than being independently dropped; no retry permit is returned.
pub(super) struct WindowsRunnerProcessPrepareFailure<'root> {
    phase: WindowsRunnerProcessPrepareFailurePhase,
    error: Error,
    custody: WindowsRunnerProcessPrepareFailureCustody<'root>,
}

enum WindowsRunnerProcessPrepareFailureCustody<'root> {
    Validated(ValidatedWindowsRunnerProcessPreparation<'root>),
    LoaderCurrentnessUnusable {
        class: WindowsRunnerPreCreateLoaderCurrentnessFailureClass,
        custody: WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody<'root>,
    },
    LoaderCurrent(LoaderCurrentWindowsRunnerProcessPreparation<'root>),
    PostCreateUnconfirmed(ManuallyDrop<WindowsRunnerUnconfirmedProcessCustody<'root>>),
}

struct WindowsRunnerUnconfirmedProcessCustody<'root> {
    _post_create: WindowsRunnerPostCreateCustody<'root>,
    recovery_key: WindowsRunnerUnconfirmedProcessRecoveryKey,
}

struct WindowsRunnerUnconfirmedProcessRecoveryKey {
    reported_process_id: u32,
    reported_primary_thread_id: u32,
    start_material_digest: String,
}

pub(super) fn prepare_suspended_windows_runner_process<
    'root,
    B: WindowsRunnerPreCreateLoaderCurrentnessBackend,
>(
    preparation: ValidatedWindowsRunnerProcessPreparation<'root>,
    loader_currentness_backend: B,
) -> std::result::Result<
    PreparedComputePluginRunnerProcess<'root>,
    WindowsRunnerProcessPrepareFailure<'root>,
> {
    let application_path = preparation.application_path.clone();
    let working_directory_path = preparation.working_directory_path.clone();
    let application = match nul_terminated_path(&application_path) {
        Ok(value) => value,
        Err(error) => return Err(before_create_failure(InputEncoding, error, preparation)),
    };
    let mut command = match command_line(&application_path, &preparation.policy.arguments) {
        Ok(value) => value,
        Err(error) => return Err(before_create_failure(InputEncoding, error, preparation)),
    };
    let current_directory = match nul_terminated_current_directory(&working_directory_path) {
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

    let loader_current = match loader_currentness_backend.query_current_and_seal(preparation) {
        Ok(value) => value,
        Err(failure) => {
            let (class, error, custody) = failure.into_parts();
            return Err(loader_currentness_failure(class, error, custody));
        }
    };
    if let Err(error) = loader_current.validate_binding() {
        let failure = loader_current.reject_invalid_binding(error);
        let (class, error, custody) = failure.into_parts();
        return Err(loader_currentness_failure(class, error, custody));
    }
    let mut post_create = match create_suspended_process_with_custody(
        job,
        loader_current,
        &application,
        &mut command,
        &environment,
        &current_directory,
    ) {
        Ok(custody) => custody,
        Err(failure) => {
            return Err(namespace_current_failure(
                failure.phase,
                failure.error,
                failure.loader_current,
            ))
        }
    };
    if !post_create.rollback().has_complete_process_information() {
        return Err(after_create_failure(
            ProcessHandleContract,
            anyhow!("COMPUTE_PLUGIN_WINDOWS_PROCESS_HANDLE_CONTRACT_BROKEN"),
            post_create,
        ));
    }

    let mut in_job = 0;
    if unsafe {
        IsProcessInJob(
            post_create.rollback().process_raw(),
            post_create.rollback().job_raw(),
            &mut in_job,
        )
    } == 0
    {
        let error = std::io::Error::last_os_error();
        return Err(after_create_failure(
            JobMembership,
            error.into(),
            post_create,
        ));
    }
    if in_job == 0 {
        return Err(after_create_failure(
            JobMembership,
            anyhow!("COMPUTE_PLUGIN_WINDOWS_JOB_MEMBERSHIP_MISSING"),
            post_create,
        ));
    }

    let process_id = unsafe { GetProcessId(post_create.rollback().process_raw()) };
    if process_id == 0 || process_id != post_create.rollback().reported_process_id() {
        let error = if process_id == 0 {
            std::io::Error::last_os_error().into()
        } else {
            anyhow!("COMPUTE_PLUGIN_WINDOWS_PROCESS_ID_CHANGED")
        };
        return Err(after_create_failure(ProcessIdentity, error, post_create));
    }
    let thread_id = unsafe { GetThreadId(post_create.rollback().primary_thread_raw()) };
    if thread_id == 0 || thread_id != post_create.rollback().reported_primary_thread_id() {
        let error = if thread_id == 0 {
            std::io::Error::last_os_error().into()
        } else {
            anyhow!("COMPUTE_PLUGIN_WINDOWS_PRIMARY_THREAD_ID_CHANGED")
        };
        return Err(after_create_failure(ProcessIdentity, error, post_create));
    }
    let creation_filetime = match process_creation_filetime(post_create.rollback().process_raw()) {
        Ok(value) if value != 0 => value,
        Ok(_) => {
            return Err(after_create_failure(
                ProcessIdentity,
                anyhow!("COMPUTE_PLUGIN_WINDOWS_PROCESS_CREATION_TIME_MISSING"),
                post_create,
            ))
        }
        Err(error) => return Err(after_create_failure(ProcessIdentity, error, post_create)),
    };
    match unsafe { WaitForSingleObject(post_create.rollback().process_raw(), 0) } {
        WAIT_TIMEOUT => {}
        WAIT_OBJECT_0 => {
            return Err(after_create_failure(
                SuspendedLiveness,
                anyhow!("COMPUTE_PLUGIN_WINDOWS_SUSPENDED_PROCESS_NOT_LIVE"),
                post_create,
            ))
        }
        WAIT_FAILED => {
            let error = std::io::Error::last_os_error();
            return Err(after_create_failure(
                SuspendedLiveness,
                error.into(),
                post_create,
            ));
        }
        status => {
            return Err(after_create_failure(
                SuspendedLiveness,
                anyhow!("COMPUTE_PLUGIN_WINDOWS_PROCESS_WAIT_CHANGED:{status}"),
                post_create,
            ))
        }
    }

    let identity = ComputePluginRunnerProcessIdentity {
        process_id,
        primary_thread_id: thread_id,
        creation_filetime,
    };
    match post_create.into_prepared_process(identity) {
        Ok(prepared) => Ok(prepared),
        Err(post_create) => Err(after_create_failure(
            ProcessHandleContract,
            anyhow!("COMPUTE_PLUGIN_WINDOWS_SUCCESS_CUSTODY_CONVERSION_CHANGED"),
            post_create,
        )),
    }
}

struct WindowsRunnerCreateProcessFailure<'root> {
    phase: WindowsRunnerProcessPrepareFailurePhase,
    error: Error,
    loader_current: LoaderCurrentWindowsRunnerProcessPreparation<'root>,
}

/// Owns Job and loader authority while issuing CreateProcessAsUserW. On success, the returned OS
/// handles enter whole-graph post-create custody inside this frame before control returns to any
/// fallible validator; the configured Job (including its attribute storage) is retained too.
fn create_suspended_process_with_custody<'root>(
    job: ConfiguredRunnerJob,
    loader_current: LoaderCurrentWindowsRunnerProcessPreparation<'root>,
    application: &[u16],
    command: &mut [u16],
    environment: &[u16],
    current_directory: &[u16],
) -> std::result::Result<
    WindowsRunnerPostCreateCustody<'root>,
    WindowsRunnerCreateProcessFailure<'root>,
> {
    if let Err(error) = loader_current.preparation.launch_security.validate() {
        return Err(WindowsRunnerCreateProcessFailure {
            phase: LaunchSecurity,
            error,
            loader_current,
        });
    }
    let startup = job.startup_info(&loader_current.preparation.launch_security);
    let create_security = loader_current.preparation.launch_security.for_create();
    // SAFETY: CreateProcessAsUserW initializes this record on success.
    let mut process_information = unsafe { zeroed::<PROCESS_INFORMATION>() };
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
    if created == 0 {
        let error = std::io::Error::last_os_error().into();
        drop(create_security);
        drop(startup);
        return Err(WindowsRunnerCreateProcessFailure {
            phase: ProcessCreation,
            error,
            loader_current,
        });
    }
    drop(create_security);
    drop(startup);
    // SAFETY: each distinct non-null handle returned on success is newly owned. Construction is
    // infallible and happens before this frame returns or performs any post-create validation.
    let rollback = unsafe { SuspendedProcessRollback::from_created(job, process_information) };
    Ok(WindowsRunnerPostCreateCustody::new(
        rollback,
        loader_current,
    ))
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
        custody: WindowsRunnerProcessPrepareFailureCustody::Validated(preparation),
    }
}

fn loader_currentness_failure<'root>(
    class: WindowsRunnerPreCreateLoaderCurrentnessFailureClass,
    error: Error,
    custody: WindowsRunnerPreCreateLoaderCurrentnessUnusableCustody<'root>,
) -> WindowsRunnerProcessPrepareFailure<'root> {
    WindowsRunnerProcessPrepareFailure {
        phase: LoaderCurrentness,
        error,
        custody: WindowsRunnerProcessPrepareFailureCustody::LoaderCurrentnessUnusable {
            class,
            custody,
        },
    }
}

fn namespace_current_failure<'root>(
    phase: WindowsRunnerProcessPrepareFailurePhase,
    error: Error,
    preparation: LoaderCurrentWindowsRunnerProcessPreparation<'root>,
) -> WindowsRunnerProcessPrepareFailure<'root> {
    WindowsRunnerProcessPrepareFailure {
        phase,
        error,
        custody: WindowsRunnerProcessPrepareFailureCustody::LoaderCurrent(preparation),
    }
}

fn after_create_failure<'root>(
    phase: WindowsRunnerProcessPrepareFailurePhase,
    error: Error,
    mut post_create: WindowsRunnerPostCreateCustody<'root>,
) -> WindowsRunnerProcessPrepareFailure<'root> {
    let custody = if post_create.rollback_mut().terminate_and_confirm() {
        let (rollback, preparation) = post_create.into_parts();
        drop(rollback);
        WindowsRunnerProcessPrepareFailureCustody::LoaderCurrent(preparation)
    } else {
        let recovery_key = WindowsRunnerUnconfirmedProcessRecoveryKey {
            reported_process_id: post_create.rollback().reported_process_id(),
            reported_primary_thread_id: post_create.rollback().reported_primary_thread_id(),
            start_material_digest: post_create
                .preparation()
                .currentness
                .start_material_digest()
                .to_owned(),
        };
        WindowsRunnerProcessPrepareFailureCustody::PostCreateUnconfirmed(ManuallyDrop::new(
            WindowsRunnerUnconfirmedProcessCustody {
                _post_create: post_create,
                recovery_key,
            },
        ))
    };
    WindowsRunnerProcessPrepareFailure {
        phase,
        error,
        custody,
    }
}

impl Drop for PreparedComputePluginRunnerProcess<'_> {
    fn drop(&mut self) {
        let custody = &*self.custody;
        if terminate_and_confirm_owned(&custody.job, &custody.process) {
            // SAFETY: termination is confirmed and this is the sole explicit drop of the
            // ManuallyDrop owner graph. On failure the graph remains deliberately parked.
            unsafe { ManuallyDrop::drop(&mut self.custody) };
        }
    }
}

impl fmt::Debug for WindowsRunnerProcessPrepareFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (loader_currentness, outcome_uncertain, recovery_process_id) = match &self.custody {
            WindowsRunnerProcessPrepareFailureCustody::Validated(_) => ("not_queried", false, None),
            WindowsRunnerProcessPrepareFailureCustody::LoaderCurrentnessUnusable {
                class, ..
            } => match class {
                WindowsRunnerPreCreateLoaderCurrentnessFailureClass::DefinitiveRejected => {
                    ("definitive_rejected", false, None)
                }
                WindowsRunnerPreCreateLoaderCurrentnessFailureClass::OutcomeUncertain => {
                    ("outcome_uncertain", true, None)
                }
            },
            WindowsRunnerProcessPrepareFailureCustody::LoaderCurrent(_) => {
                ("retained_current", false, None)
            }
            WindowsRunnerProcessPrepareFailureCustody::PostCreateUnconfirmed(custody) => {
                let _ = (
                    custody.recovery_key.reported_primary_thread_id,
                    &custody.recovery_key.start_material_digest,
                );
                (
                    "retained_current",
                    true,
                    Some(custody.recovery_key.reported_process_id),
                )
            }
        };
        formatter
            .debug_struct("WindowsRunnerProcessPrepareFailure")
            .field("phase", &self.phase)
            .field("error", &self.error)
            .field("preparation", &"<retained-linear-custody>")
            .field("outcome_uncertain", &outcome_uncertain)
            .field("recovery_process_id", &recovery_process_id)
            .field("loader_currentness", &loader_currentness)
            .finish()
    }
}

impl fmt::Display for WindowsRunnerProcessPrepareFailure<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:#}", self.error)
    }
}

impl StdError for WindowsRunnerProcessPrepareFailure<'_> {}

use WindowsRunnerProcessPrepareFailurePhase::*;
