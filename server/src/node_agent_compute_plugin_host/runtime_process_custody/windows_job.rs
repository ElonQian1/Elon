use std::{
    marker::PhantomData,
    mem::{size_of, zeroed},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle, RawHandle},
    ptr::{null, null_mut},
};

use anyhow::{anyhow, Error};
use windows_sys::Win32::{
    Foundation::{GetHandleInformation, HANDLE},
    System::{
        JobObjects::{
            CreateJobObjectW, JobObjectExtendedLimitInformation, QueryInformationJobObject,
            SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
            JOB_OBJECT_LIMIT_ACTIVE_PROCESS, JOB_OBJECT_LIMIT_BREAKAWAY_OK,
            JOB_OBJECT_LIMIT_JOB_MEMORY, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK,
        },
        Threading::{
            DeleteProcThreadAttributeList, InitializeProcThreadAttributeList,
            UpdateProcThreadAttribute, LPPROC_THREAD_ATTRIBUTE_LIST,
            PROC_THREAD_ATTRIBUTE_JOB_LIST, STARTUPINFOEXW, STARTUPINFOW,
        },
    },
};

use super::policy::WindowsRunnerProcessPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConfiguredRunnerJobFailurePhase {
    Creation,
    Configuration,
    AttributeList,
}

pub(super) struct ConfiguredRunnerJobFailure {
    pub(super) phase: ConfiguredRunnerJobFailurePhase,
    pub(super) error: Error,
}

/// Anonymous kill-on-close Job plus an aligned creation attribute that assigns the child before
/// CreateProcessAsUserW returns. The Job handle value remains at a stable address for the entire
/// lifetime of the opaque attribute list, as required by UpdateProcThreadAttribute.
pub(super) struct ConfiguredRunnerJob {
    job: OwnedHandle,
    attributes: AlignedProcessThreadAttributeList,
}

pub(super) struct ConfiguredRunnerStartupInfo<'job> {
    raw: STARTUPINFOEXW,
    _job: PhantomData<&'job ConfiguredRunnerJob>,
}

impl ConfiguredRunnerJob {
    pub(super) fn create(
        policy: &WindowsRunnerProcessPolicy,
    ) -> Result<Self, ConfiguredRunnerJobFailure> {
        let raw = unsafe { CreateJobObjectW(null(), null()) };
        if raw.is_null() {
            return Err(failure(
                ConfiguredRunnerJobFailurePhase::Creation,
                std::io::Error::last_os_error().into(),
            ));
        }
        // SAFETY: CreateJobObjectW returned a new owned handle, wrapped exactly once here.
        let job = unsafe { OwnedHandle::from_raw_handle(raw as RawHandle) };
        if let Err(error) = configure_and_query_back(&job, policy) {
            return Err(failure(
                ConfiguredRunnerJobFailurePhase::Configuration,
                error,
            ));
        }
        let attributes = AlignedProcessThreadAttributeList::for_job(&job)
            .map_err(|error| failure(ConfiguredRunnerJobFailurePhase::AttributeList, error))?;
        Ok(Self { job, attributes })
    }

    pub(super) fn startup_info(&self) -> ConfiguredRunnerStartupInfo<'_> {
        // SAFETY: zero is valid for every optional STARTUPINFOEXW member.
        let mut raw = unsafe { zeroed::<STARTUPINFOEXW>() };
        raw.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
        raw.lpAttributeList = self.attributes.pointer;
        ConfiguredRunnerStartupInfo {
            raw,
            _job: PhantomData,
        }
    }

    pub(super) fn into_handle(self) -> OwnedHandle {
        self.job
    }
}

impl ConfiguredRunnerStartupInfo<'_> {
    pub(super) fn as_ptr(&self) -> *const STARTUPINFOW {
        (&self.raw as *const STARTUPINFOEXW).cast()
    }
}

struct AlignedProcessThreadAttributeList {
    words: Box<[usize]>,
    job_value: Box<HANDLE>,
    pointer: LPPROC_THREAD_ATTRIBUTE_LIST,
}

impl AlignedProcessThreadAttributeList {
    fn for_job(job: &OwnedHandle) -> Result<Self, Error> {
        let mut required_bytes = 0_usize;
        unsafe {
            InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut required_bytes);
        }
        if required_bytes == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let word_bytes = size_of::<usize>();
        let word_count = required_bytes
            .checked_add(word_bytes - 1)
            .ok_or_else(|| anyhow!("COMPUTE_PLUGIN_WINDOWS_ATTRIBUTE_SIZE_OVERFLOW"))?
            / word_bytes;
        let mut words = vec![0_usize; word_count].into_boxed_slice();
        let pointer = words.as_mut_ptr().cast();
        if unsafe { InitializeProcThreadAttributeList(pointer, 1, 0, &mut required_bytes) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let job_value = Box::new(owned_raw(job));
        let list = Self {
            words,
            job_value,
            pointer,
        };
        if unsafe {
            UpdateProcThreadAttribute(
                list.pointer,
                0,
                PROC_THREAD_ATTRIBUTE_JOB_LIST as usize,
                (&*list.job_value as *const HANDLE).cast(),
                size_of::<HANDLE>(),
                null_mut(),
                null(),
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        Ok(list)
    }
}

impl Drop for AlignedProcessThreadAttributeList {
    fn drop(&mut self) {
        unsafe { DeleteProcThreadAttributeList(self.pointer) };
    }
}

fn configure_and_query_back(
    job: &OwnedHandle,
    policy: &WindowsRunnerProcessPolicy,
) -> Result<(), Error> {
    let mut handle_flags = 0_u32;
    if unsafe { GetHandleInformation(owned_raw(job), &mut handle_flags) } == 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if handle_flags != 0 {
        return Err(anyhow!("COMPUTE_PLUGIN_WINDOWS_JOB_HANDLE_FLAGS_INVALID"));
    }
    // SAFETY: zero is the documented baseline for unused JOBOBJECT limit fields.
    let mut limits = unsafe { zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
        | JOB_OBJECT_LIMIT_ACTIVE_PROCESS
        | JOB_OBJECT_LIMIT_JOB_MEMORY;
    limits.BasicLimitInformation.ActiveProcessLimit = policy.active_process_limit;
    limits.JobMemoryLimit = policy.job_memory_limit_bytes;
    if unsafe {
        SetInformationJobObject(
            owned_raw(job),
            JobObjectExtendedLimitInformation,
            (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }

    // SAFETY: the query writes exactly one JOBOBJECT_EXTENDED_LIMIT_INFORMATION record.
    let mut observed = unsafe { zeroed::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() };
    let mut returned = 0_u32;
    if unsafe {
        QueryInformationJobObject(
            owned_raw(job),
            JobObjectExtendedLimitInformation,
            (&mut observed as *mut JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
            size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            &mut returned,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    let required = limits.BasicLimitInformation.LimitFlags;
    let forbidden = JOB_OBJECT_LIMIT_BREAKAWAY_OK | JOB_OBJECT_LIMIT_SILENT_BREAKAWAY_OK;
    if returned != size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32
        || observed.BasicLimitInformation.LimitFlags & required != required
        || observed.BasicLimitInformation.LimitFlags & forbidden != 0
        || observed.BasicLimitInformation.ActiveProcessLimit != policy.active_process_limit
        || observed.JobMemoryLimit != policy.job_memory_limit_bytes
    {
        return Err(anyhow!("COMPUTE_PLUGIN_WINDOWS_JOB_QUERY_BACK_CHANGED"));
    }
    Ok(())
}

fn failure(phase: ConfiguredRunnerJobFailurePhase, error: Error) -> ConfiguredRunnerJobFailure {
    ConfiguredRunnerJobFailure { phase, error }
}

fn owned_raw(handle: &OwnedHandle) -> HANDLE {
    handle.as_raw_handle() as HANDLE
}
