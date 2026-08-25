use std::{
    marker::PhantomData,
    mem::size_of,
    os::windows::io::{AsRawHandle, OwnedHandle},
    slice,
};

use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use windows_sys::Win32::{
    Foundation::{GetHandleInformation, HANDLE},
    Security::{
        AclSizeInformation, GetAclInformation, GetSecurityDescriptorControl,
        GetSecurityDescriptorDacl, GetSecurityDescriptorLength, GetTokenInformation,
        IsTokenRestricted, IsValidSecurityDescriptor, TokenIsAppContainer, TokenPrimary, TokenType,
        ACL_SIZE_INFORMATION, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR_RELATIVE, SE_SELF_RELATIVE,
    },
};

use crate::node_agent_compute_plugin_host::manifest_validation::is_sha256;

/// Creation-time authority for a least-rights primary token and empty-DACL child handles.
///
/// There is intentionally no constructor in this source slice. A future producer must derive a
/// restricted/AppContainer primary token from the exact work-admission grant, close every adjust
/// handle, reopen only the rights required by CreateProcessAsUserW, and seal query-back material.
/// The aligned self-relative descriptors must each contain a present, non-NULL, empty DACL.
pub(super) struct SealedWindowsRunnerLaunchSecurity {
    primary_token: OwnedHandle,
    process_descriptor: AlignedSelfRelativeSecurityDescriptor,
    thread_descriptor: AlignedSelfRelativeSecurityDescriptor,
    token_profile_digest: String,
    restricted_token_expected: bool,
    app_container_expected: bool,
}

/// `usize` storage gives Win32 security descriptors explicit pointer alignment while `byte_len`
/// excludes allocator padding from validation and hashing.
struct AlignedSelfRelativeSecurityDescriptor {
    words: Box<[usize]>,
    byte_len: usize,
    digest: String,
}

pub(super) struct WindowsRunnerCreateSecurity<'owner> {
    pub(super) primary_token: HANDLE,
    pub(super) process_attributes: SECURITY_ATTRIBUTES,
    pub(super) thread_attributes: SECURITY_ATTRIBUTES,
    _owner: PhantomData<&'owner SealedWindowsRunnerLaunchSecurity>,
}

impl SealedWindowsRunnerLaunchSecurity {
    pub(super) fn validate(&self) -> Result<()> {
        if !is_sha256(&self.token_profile_digest)
            || (!self.restricted_token_expected && !self.app_container_expected)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_LAUNCH_SECURITY_BINDING_INVALID");
        }
        let token = self.primary_token.as_raw_handle() as HANDLE;
        let mut handle_flags = 0_u32;
        if unsafe { GetHandleInformation(token, &mut handle_flags) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if handle_flags != 0 {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRIMARY_TOKEN_HANDLE_FLAGS_INVALID");
        }
        if query_token_u32(token, TokenType)? != TokenPrimary as u32 {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRIMARY_TOKEN_TYPE_CHANGED");
        }
        let restricted = unsafe { IsTokenRestricted(token) } != 0;
        let app_container = query_token_u32(token, TokenIsAppContainer)? != 0;
        if restricted != self.restricted_token_expected
            || app_container != self.app_container_expected
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRIMARY_TOKEN_PROFILE_CHANGED");
        }
        self.process_descriptor.validate_empty_dacl()?;
        self.thread_descriptor.validate_empty_dacl()?;
        Ok(())
    }

    pub(super) fn for_create(&self) -> WindowsRunnerCreateSecurity<'_> {
        WindowsRunnerCreateSecurity {
            primary_token: self.primary_token.as_raw_handle() as HANDLE,
            process_attributes: self.process_descriptor.security_attributes(),
            thread_attributes: self.thread_descriptor.security_attributes(),
            _owner: PhantomData,
        }
    }

    pub(super) fn token_profile_digest(&self) -> &str {
        &self.token_profile_digest
    }

    pub(super) fn process_descriptor_digest(&self) -> &str {
        &self.process_descriptor.digest
    }

    pub(super) fn thread_descriptor_digest(&self) -> &str {
        &self.thread_descriptor.digest
    }

    pub(super) const fn restricted_token_expected(&self) -> bool {
        self.restricted_token_expected
    }

    pub(super) const fn app_container_expected(&self) -> bool {
        self.app_container_expected
    }
}

impl AlignedSelfRelativeSecurityDescriptor {
    fn bytes(&self) -> Result<&[u8]> {
        let capacity = self
            .words
            .len()
            .checked_mul(size_of::<usize>())
            .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_SD_SIZE_OVERFLOW"))?;
        if self.byte_len < size_of::<SECURITY_DESCRIPTOR_RELATIVE>() || self.byte_len > capacity {
            bail!("COMPUTE_PLUGIN_WINDOWS_SD_LENGTH_INVALID");
        }
        // SAFETY: `words` owns at least `capacity` initialized bytes and `byte_len` was bounded.
        Ok(unsafe { slice::from_raw_parts(self.words.as_ptr().cast::<u8>(), self.byte_len) })
    }

    fn validate_empty_dacl(&self) -> Result<()> {
        let bytes = self.bytes()?;
        if !is_sha256(&self.digest) || hex::encode(Sha256::digest(bytes)) != self.digest {
            bail!("COMPUTE_PLUGIN_WINDOWS_SD_DIGEST_CHANGED");
        }
        let descriptor = self.words.as_ptr().cast_mut().cast();
        if unsafe { IsValidSecurityDescriptor(descriptor) } == 0 {
            bail!("COMPUTE_PLUGIN_WINDOWS_SD_INVALID");
        }
        if usize::try_from(unsafe { GetSecurityDescriptorLength(descriptor) })? != self.byte_len {
            bail!("COMPUTE_PLUGIN_WINDOWS_SD_LENGTH_CHANGED");
        }
        let mut control = 0_u16;
        let mut revision = 0_u32;
        if unsafe { GetSecurityDescriptorControl(descriptor, &mut control, &mut revision) } == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        if control & SE_SELF_RELATIVE == 0 || revision == 0 {
            bail!("COMPUTE_PLUGIN_WINDOWS_SD_NOT_SELF_RELATIVE");
        }
        let mut dacl_present = 0;
        let mut dacl = std::ptr::null_mut();
        let mut dacl_defaulted = 0;
        if unsafe {
            GetSecurityDescriptorDacl(
                descriptor,
                &mut dacl_present,
                &mut dacl,
                &mut dacl_defaulted,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        if dacl_present == 0 || dacl.is_null() || dacl_defaulted != 0 {
            bail!("COMPUTE_PLUGIN_WINDOWS_SD_DACL_NOT_EXPLICIT");
        }
        let mut acl_size = ACL_SIZE_INFORMATION {
            AceCount: 0,
            AclBytesInUse: 0,
            AclBytesFree: 0,
        };
        if unsafe {
            GetAclInformation(
                dacl,
                (&mut acl_size as *mut ACL_SIZE_INFORMATION).cast(),
                size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        if acl_size.AceCount != 0 {
            bail!("COMPUTE_PLUGIN_WINDOWS_SD_DACL_NOT_EMPTY");
        }
        Ok(())
    }

    fn security_attributes(&self) -> SECURITY_ATTRIBUTES {
        SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.words.as_ptr().cast_mut().cast(),
            bInheritHandle: 0,
        }
    }
}

fn query_token_u32(token: HANDLE, class: i32) -> Result<u32> {
    let mut value = 0_u32;
    let mut returned = 0_u32;
    if unsafe {
        GetTokenInformation(
            token,
            class,
            (&mut value as *mut u32).cast(),
            size_of::<u32>() as u32,
            &mut returned,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    if returned != size_of::<u32>() as u32 {
        bail!("COMPUTE_PLUGIN_WINDOWS_TOKEN_QUERY_SIZE_CHANGED");
    }
    Ok(value)
}

impl std::fmt::Debug for SealedWindowsRunnerLaunchSecurity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SealedWindowsRunnerLaunchSecurity")
            .field("primary_token", &"<least-rights-owned-handle>")
            .field("process_descriptor", &"<empty-dacl-self-relative>")
            .field("thread_descriptor", &"<empty-dacl-self-relative>")
            .field("token_profile_digest", &"<redacted>")
            .finish()
    }
}
