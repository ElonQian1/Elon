use std::{
    convert::Infallible,
    marker::PhantomData,
    mem::size_of,
    os::windows::io::{AsRawHandle, OwnedHandle},
    slice,
};

use anyhow::{bail, Result};
use serde_json::json;
use sha2::{Digest, Sha256};
use windows_sys::Win32::{
    Foundation::{GetHandleInformation, HANDLE},
    Security::{
        AclSizeInformation, GetAclInformation, GetSecurityDescriptorControl,
        GetSecurityDescriptorDacl, GetSecurityDescriptorLength, GetTokenInformation,
        GetUserObjectSecurity, IsTokenRestricted, IsValidSecurityDescriptor, TokenIsAppContainer,
        TokenPrimary, TokenSessionId, TokenStatistics, TokenType, ACL_SIZE_INFORMATION,
        DACL_SECURITY_INFORMATION, GROUP_SECURITY_INFORMATION, LABEL_SECURITY_INFORMATION,
        OWNER_SECURITY_INFORMATION, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR_RELATIVE,
        SE_SELF_RELATIVE, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY, TOKEN_STATISTICS,
    },
    System::StationsAndDesktops::{
        CloseDesktop, CloseWindowStation, GetUserObjectInformationW, HDESK, HWINSTA, UOI_NAME,
        UOI_TYPE,
    },
};

use crate::node_agent_compute_plugin_host::{
    manifest_validation::is_sha256, signed_artifact_verification::jcs_sha256_hex,
};

/// Creation-time authority for a least-rights primary token and empty-DACL child handles.
///
/// There is intentionally no constructor in this source slice. A future producer must derive a
/// restricted/AppContainer primary token from the exact work-admission grant, close every adjust
/// handle, reopen only the rights required by CreateProcessAsUserW, and seal query-back material.
/// The aligned self-relative descriptors must each contain a present, non-NULL, empty DACL. That
/// DACL alone does not exclude a same-owner principal with WRITE_DAC; owner SID, mandatory label,
/// service/account isolation, and post-create query-back are separate required proofs.
pub(super) struct SealedWindowsRunnerLaunchSecurity {
    primary_token: OwnedHandle,
    process_descriptor: AlignedSelfRelativeSecurityDescriptor,
    thread_descriptor: AlignedSelfRelativeSecurityDescriptor,
    token_profile_digest: String,
    restricted_token_expected: bool,
    app_container_expected: bool,
    token_user_sid_digest: String,
    process_owner_sid_digest: String,
    thread_owner_sid_digest: String,
    process_mandatory_label_digest: String,
    thread_mandatory_label_digest: String,
    object_isolation_profile_digest: String,
    token_granted_access_mask: u32,
    target_object_access_check_set_digest: String,
    caller_token_privilege_lineage_digest: String,
    primary_token_session_id: u32,
    primary_token_logon_session_identity_digest: String,
    private_desktop_isolation_digest: String,
    private_desktop: SealedWindowsRunnerPrivateDesktopCustody,
    _object_owner_label_isolation_producer_unavailable: Infallible,
    _target_token_accesscheck_producer_unavailable: Infallible,
    _caller_privilege_lineage_producer_unavailable: Infallible,
    _private_desktop_isolation_producer_unavailable: Infallible,
}

/// Exact named window-station/desktop owners consumed by `STARTUPINFO.lpDesktop`. Desktop is
/// declared first so it closes before its containing window station after child termination.
struct SealedWindowsRunnerPrivateDesktopCustody {
    _desktop: OwnedPrivateDesktop,
    _window_station: OwnedPrivateWindowStation,
    qualified_name: Box<[u16]>,
    window_station_query_receipt: SealedWindowsUserObjectQueryReceipt,
    desktop_query_receipt: SealedWindowsUserObjectQueryReceipt,
    desktop_parent_binding_receipt: SealedWindowsDesktopParentBindingReceipt,
    token_session_namespace_binding_receipt: SealedWindowsDesktopTokenSessionBindingReceipt,
    window_station_identity_digest: String,
    desktop_identity_digest: String,
}

/// Retained exact query-back material for one live user object. A future producer must capture
/// name, type, and owner/group/DACL/mandatory-label security from the same retained handle.
struct SealedWindowsUserObjectQueryReceipt {
    name_utf16: Box<[u16]>,
    object_type_utf16: Box<[u16]>,
    security_descriptor: Box<[u8]>,
    name_query_digest: String,
    type_query_digest: String,
    security_query_digest: String,
    receipt_digest: String,
    _authenticated_user_object_query_producer_unavailable: Infallible,
}

/// Authenticated creation/open relation proving that the retained HDESK is a child of the exact
/// retained HWINSTA. Independent UOI_NAME queries cannot establish this parent relationship.
struct SealedWindowsDesktopParentBindingReceipt {
    window_station_handle_value: usize,
    desktop_handle_value: usize,
    window_station_identity_digest: String,
    desktop_identity_digest: String,
    qualified_name_digest: String,
    creation_request_digest: String,
    authenticated_response_digest: String,
    receipt_digest: String,
    authenticated_response: Vec<u8>,
    _authenticated_desktop_parent_backend_unavailable: Infallible,
}

/// Authenticated creation/open evidence tying the exact private user objects to the live primary
/// token's Windows session and logon namespace. Names and security descriptors alone are not
/// globally unique across sessions, so this receipt is part of pre-create currentness.
struct SealedWindowsDesktopTokenSessionBindingReceipt {
    primary_token_handle_value: usize,
    primary_token_session_id: u32,
    primary_token_logon_session_identity_digest: String,
    window_station_handle_value: usize,
    desktop_handle_value: usize,
    window_station_identity_digest: String,
    desktop_identity_digest: String,
    target_object_access_check_set_digest: String,
    creation_and_access_check_request_digest: String,
    authenticated_response_digest: String,
    receipt_digest: String,
    authenticated_response: Vec<u8>,
    _authenticated_token_session_namespace_backend_unavailable: Infallible,
}

struct OwnedPrivateDesktop(HDESK);
struct OwnedPrivateWindowStation(HWINSTA);

impl Drop for OwnedPrivateDesktop {
    fn drop(&mut self) {
        unsafe { CloseDesktop(self.0) };
    }
}

impl Drop for OwnedPrivateWindowStation {
    fn drop(&mut self) {
        unsafe { CloseWindowStation(self.0) };
    }
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
            || !is_sha256(&self.token_user_sid_digest)
            || !is_sha256(&self.process_owner_sid_digest)
            || !is_sha256(&self.thread_owner_sid_digest)
            || !is_sha256(&self.process_mandatory_label_digest)
            || !is_sha256(&self.thread_mandatory_label_digest)
            || !is_sha256(&self.object_isolation_profile_digest)
            || !is_sha256(&self.target_object_access_check_set_digest)
            || !is_sha256(&self.caller_token_privilege_lineage_digest)
            || !is_sha256(&self.primary_token_logon_session_identity_digest)
            || !is_sha256(&self.private_desktop_isolation_digest)
            || self.token_granted_access_mask
                != (TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY)
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
        let live_session_id = query_token_u32(token, TokenSessionId)?;
        let live_statistics = query_token_statistics(token)?;
        let live_logon_session_identity_digest =
            token_logon_session_identity_digest(&live_statistics)?;
        if restricted != self.restricted_token_expected
            || app_container != self.app_container_expected
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRIMARY_TOKEN_PROFILE_CHANGED");
        }
        if live_session_id != self.primary_token_session_id
            || live_logon_session_identity_digest
                != self.primary_token_logon_session_identity_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRIMARY_TOKEN_SESSION_CHANGED");
        }
        self.process_descriptor.validate_empty_dacl()?;
        self.thread_descriptor.validate_empty_dacl()?;
        self.private_desktop.validate(
            token,
            live_session_id,
            &live_logon_session_identity_digest,
            &self.target_object_access_check_set_digest,
            &self.private_desktop_isolation_digest,
        )?;
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

    pub(super) fn token_user_sid_digest(&self) -> &str {
        &self.token_user_sid_digest
    }

    pub(super) fn process_owner_sid_digest(&self) -> &str {
        &self.process_owner_sid_digest
    }

    pub(super) fn thread_owner_sid_digest(&self) -> &str {
        &self.thread_owner_sid_digest
    }

    pub(super) fn process_mandatory_label_digest(&self) -> &str {
        &self.process_mandatory_label_digest
    }

    pub(super) fn thread_mandatory_label_digest(&self) -> &str {
        &self.thread_mandatory_label_digest
    }

    pub(super) fn object_isolation_profile_digest(&self) -> &str {
        &self.object_isolation_profile_digest
    }

    pub(super) fn token_granted_access_mask(&self) -> u32 {
        self.token_granted_access_mask
    }

    pub(super) fn target_object_access_check_set_digest(&self) -> &str {
        &self.target_object_access_check_set_digest
    }

    pub(super) fn caller_token_privilege_lineage_digest(&self) -> &str {
        &self.caller_token_privilege_lineage_digest
    }

    pub(super) fn private_desktop_isolation_digest(&self) -> &str {
        &self.private_desktop_isolation_digest
    }

    pub(super) fn private_desktop_name_ptr(&self) -> *mut u16 {
        self.private_desktop.qualified_name.as_ptr().cast_mut()
    }
}

impl SealedWindowsRunnerPrivateDesktopCustody {
    fn validate(
        &self,
        primary_token: HANDLE,
        primary_token_session_id: u32,
        primary_token_logon_session_identity_digest: &str,
        target_object_access_check_set_digest: &str,
        expected_isolation_digest: &str,
    ) -> Result<()> {
        let name = self.qualified_name.as_ref();
        if self._desktop.0.is_null()
            || self._window_station.0.is_null()
            || name.len() < 4
            || name.len() > 260
            || name.last() != Some(&0)
            || name[..name.len() - 1].contains(&0)
            || name[..name.len() - 1]
                .iter()
                .filter(|unit| **unit == u16::from(b'\\'))
                .count()
                != 1
            || !is_sha256(&self.window_station_identity_digest)
            || !is_sha256(&self.desktop_identity_digest)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRIVATE_DESKTOP_BINDING_INVALID");
        }
        let station_identity = self.window_station_query_receipt.validate(
            self._window_station.0,
            "window_station",
            "WindowStation",
        )?;
        let desktop_identity =
            self.desktop_query_receipt
                .validate(self._desktop.0, "desktop", "Desktop")?;
        if station_identity != self.window_station_identity_digest
            || desktop_identity != self.desktop_identity_digest
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRIVATE_DESKTOP_IDENTITY_CHANGED");
        }
        let station_name = without_terminal_nul(
            &self.window_station_query_receipt.name_utf16,
            "COMPUTE_PLUGIN_WINDOWS_WINDOW_STATION_NAME_INVALID",
        )?;
        let desktop_name = without_terminal_nul(
            &self.desktop_query_receipt.name_utf16,
            "COMPUTE_PLUGIN_WINDOWS_DESKTOP_NAME_INVALID",
        )?;
        let mut expected_qualified_name =
            Vec::with_capacity(station_name.len() + 1 + desktop_name.len() + 1);
        expected_qualified_name.extend_from_slice(station_name);
        expected_qualified_name.push(u16::from(b'\\'));
        expected_qualified_name.extend_from_slice(desktop_name);
        expected_qualified_name.push(0);
        if name != expected_qualified_name.as_slice() {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRIVATE_DESKTOP_NAME_CHANGED");
        }
        let qualified_name_digest = sha256_wide(name);
        let parent_binding_digest = self.desktop_parent_binding_receipt.validate(
            self._window_station.0,
            self._desktop.0,
            &station_identity,
            &desktop_identity,
            &qualified_name_digest,
        )?;
        let token_session_namespace_binding_digest =
            self.token_session_namespace_binding_receipt.validate(
                primary_token,
                primary_token_session_id,
                primary_token_logon_session_identity_digest,
                self._window_station.0,
                self._desktop.0,
                &station_identity,
                &desktop_identity,
                target_object_access_check_set_digest,
            )?;
        let isolation_digest = jcs_sha256_hex(&json!({
            "schema": "elon.compute_plugin.windows_private_desktop_isolation.v1",
            "window_station_identity_digest": station_identity,
            "desktop_identity_digest": desktop_identity,
            "qualified_name_digest": qualified_name_digest,
            "desktop_parent_binding_receipt_digest": parent_binding_digest,
            "primary_token_session_id": primary_token_session_id,
            "primary_token_logon_session_identity_digest": primary_token_logon_session_identity_digest,
            "target_object_access_check_set_digest": target_object_access_check_set_digest,
            "token_session_namespace_binding_receipt_digest": token_session_namespace_binding_digest,
        }))?;
        if isolation_digest != expected_isolation_digest {
            bail!("COMPUTE_PLUGIN_WINDOWS_PRIVATE_DESKTOP_ISOLATION_CHANGED");
        }
        Ok(())
    }
}

impl SealedWindowsDesktopTokenSessionBindingReceipt {
    #[allow(clippy::too_many_arguments)]
    fn validate(
        &self,
        primary_token: HANDLE,
        primary_token_session_id: u32,
        primary_token_logon_session_identity_digest: &str,
        window_station: HWINSTA,
        desktop: HDESK,
        window_station_identity_digest: &str,
        desktop_identity_digest: &str,
        target_object_access_check_set_digest: &str,
    ) -> Result<String> {
        let authenticated_response_digest =
            hex::encode(Sha256::digest(&self.authenticated_response));
        if self.primary_token_handle_value != primary_token as usize
            || self.primary_token_session_id != primary_token_session_id
            || self.primary_token_logon_session_identity_digest
                != primary_token_logon_session_identity_digest
            || self.window_station_handle_value != window_station as usize
            || self.desktop_handle_value != desktop as usize
            || self.window_station_identity_digest != window_station_identity_digest
            || self.desktop_identity_digest != desktop_identity_digest
            || self.target_object_access_check_set_digest != target_object_access_check_set_digest
            || self.authenticated_response.is_empty()
            || self.authenticated_response_digest != authenticated_response_digest
            || [
                &self.primary_token_logon_session_identity_digest,
                &self.window_station_identity_digest,
                &self.desktop_identity_digest,
                &self.target_object_access_check_set_digest,
                &self.creation_and_access_check_request_digest,
                &self.authenticated_response_digest,
                &self.receipt_digest,
            ]
            .iter()
            .any(|digest| !is_sha256(digest))
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_DESKTOP_TOKEN_SESSION_BINDING_CHANGED");
        }
        let receipt_digest = jcs_sha256_hex(&json!({
            "schema": "elon.compute_plugin.windows_desktop_token_session_binding.v1",
            "primary_token_handle_value": self.primary_token_handle_value,
            "primary_token_session_id": self.primary_token_session_id,
            "primary_token_logon_session_identity_digest": self.primary_token_logon_session_identity_digest,
            "window_station_handle_value": self.window_station_handle_value,
            "desktop_handle_value": self.desktop_handle_value,
            "window_station_identity_digest": self.window_station_identity_digest,
            "desktop_identity_digest": self.desktop_identity_digest,
            "target_object_access_check_set_digest": self.target_object_access_check_set_digest,
            "creation_and_access_check_request_digest": self.creation_and_access_check_request_digest,
            "authenticated_response_digest": self.authenticated_response_digest,
        }))?;
        if receipt_digest != self.receipt_digest {
            bail!("COMPUTE_PLUGIN_WINDOWS_DESKTOP_TOKEN_SESSION_RECEIPT_CHANGED");
        }
        Ok(receipt_digest)
    }
}

impl SealedWindowsDesktopParentBindingReceipt {
    fn validate(
        &self,
        window_station: HWINSTA,
        desktop: HDESK,
        station_identity: &str,
        desktop_identity: &str,
        qualified_name_digest: &str,
    ) -> Result<String> {
        let authenticated_response_digest =
            hex::encode(Sha256::digest(&self.authenticated_response));
        if self.window_station_handle_value != window_station as usize
            || self.desktop_handle_value != desktop as usize
            || self.window_station_identity_digest != station_identity
            || self.desktop_identity_digest != desktop_identity
            || self.qualified_name_digest != qualified_name_digest
            || self.authenticated_response.is_empty()
            || self.authenticated_response_digest != authenticated_response_digest
            || [
                &self.window_station_identity_digest,
                &self.desktop_identity_digest,
                &self.qualified_name_digest,
                &self.creation_request_digest,
                &self.authenticated_response_digest,
                &self.receipt_digest,
            ]
            .iter()
            .any(|digest| !is_sha256(digest))
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_DESKTOP_PARENT_BINDING_CHANGED");
        }
        let receipt_digest = jcs_sha256_hex(&json!({
            "schema": "elon.compute_plugin.windows_desktop_parent_binding.v1",
            "window_station_handle_value": self.window_station_handle_value,
            "desktop_handle_value": self.desktop_handle_value,
            "window_station_identity_digest": self.window_station_identity_digest,
            "desktop_identity_digest": self.desktop_identity_digest,
            "qualified_name_digest": self.qualified_name_digest,
            "creation_request_digest": self.creation_request_digest,
            "authenticated_response_digest": self.authenticated_response_digest,
        }))?;
        if receipt_digest != self.receipt_digest {
            bail!("COMPUTE_PLUGIN_WINDOWS_DESKTOP_PARENT_RECEIPT_CHANGED");
        }
        Ok(receipt_digest)
    }
}

impl SealedWindowsUserObjectQueryReceipt {
    fn validate(&self, handle: HANDLE, object_kind: &str, expected_type: &str) -> Result<String> {
        let live_name = query_user_object_wide(handle, UOI_NAME)?;
        let live_type = query_user_object_wide(handle, UOI_TYPE)?;
        let live_security = query_user_object_security(handle)?;
        let expected_type = wide_z(expected_type);
        if self.name_utf16.as_ref() != live_name.as_slice()
            || self.object_type_utf16.as_ref() != live_type.as_slice()
            || self.object_type_utf16.as_ref() != expected_type.as_slice()
            || self.security_descriptor.as_ref() != live_security.as_slice()
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_USER_OBJECT_QUERY_BACK_CHANGED");
        }
        let name_query_digest = sha256_wide(&live_name);
        let type_query_digest = sha256_wide(&live_type);
        let security_query_digest = hex::encode(Sha256::digest(&live_security));
        if name_query_digest != self.name_query_digest
            || type_query_digest != self.type_query_digest
            || security_query_digest != self.security_query_digest
            || !is_sha256(&self.receipt_digest)
        {
            bail!("COMPUTE_PLUGIN_WINDOWS_USER_OBJECT_QUERY_DIGEST_CHANGED");
        }
        let receipt_digest = jcs_sha256_hex(&json!({
            "schema": "elon.compute_plugin.windows_user_object_query_receipt.v1",
            "object_kind": object_kind,
            "name_query_digest": name_query_digest,
            "type_query_digest": type_query_digest,
            "security_query_digest": security_query_digest,
        }))?;
        if receipt_digest != self.receipt_digest {
            bail!("COMPUTE_PLUGIN_WINDOWS_USER_OBJECT_RECEIPT_CHANGED");
        }
        Ok(receipt_digest)
    }
}

fn query_user_object_wide(handle: HANDLE, index: i32) -> Result<Vec<u16>> {
    let mut required_bytes = 0_u32;
    unsafe {
        GetUserObjectInformationW(handle, index, std::ptr::null_mut(), 0, &mut required_bytes);
    }
    if required_bytes < 2 || required_bytes % 2 != 0 {
        bail!("COMPUTE_PLUGIN_WINDOWS_USER_OBJECT_QUERY_SIZE_INVALID");
    }
    let mut value = vec![0_u16; usize::try_from(required_bytes / 2)?];
    let mut returned_bytes = required_bytes;
    if unsafe {
        GetUserObjectInformationW(
            handle,
            index,
            value.as_mut_ptr().cast(),
            required_bytes,
            &mut returned_bytes,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    if returned_bytes != required_bytes
        || value.last() != Some(&0)
        || value[..value.len() - 1].contains(&0)
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_USER_OBJECT_QUERY_RESULT_INVALID");
    }
    Ok(value)
}

fn query_user_object_security(handle: HANDLE) -> Result<Vec<u8>> {
    let requested = OWNER_SECURITY_INFORMATION
        | GROUP_SECURITY_INFORMATION
        | DACL_SECURITY_INFORMATION
        | LABEL_SECURITY_INFORMATION;
    let mut required_bytes = 0_u32;
    unsafe {
        GetUserObjectSecurity(
            handle,
            &requested,
            std::ptr::null_mut(),
            0,
            &mut required_bytes,
        );
    }
    if required_bytes < size_of::<SECURITY_DESCRIPTOR_RELATIVE>() as u32 {
        bail!("COMPUTE_PLUGIN_WINDOWS_USER_OBJECT_SD_SIZE_INVALID");
    }
    let word_bytes = size_of::<usize>();
    let word_count = usize::try_from(required_bytes)?
        .checked_add(word_bytes - 1)
        .ok_or_else(|| anyhow::anyhow!("COMPUTE_PLUGIN_WINDOWS_USER_OBJECT_SD_SIZE_OVERFLOW"))?
        / word_bytes;
    let mut words = vec![0_usize; word_count];
    let mut returned_bytes = required_bytes;
    if unsafe {
        GetUserObjectSecurity(
            handle,
            &requested,
            words.as_mut_ptr().cast(),
            required_bytes,
            &mut returned_bytes,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    if returned_bytes != required_bytes
        || unsafe { IsValidSecurityDescriptor(words.as_mut_ptr().cast()) } == 0
        || unsafe { GetSecurityDescriptorLength(words.as_mut_ptr().cast()) } != required_bytes
    {
        bail!("COMPUTE_PLUGIN_WINDOWS_USER_OBJECT_SD_CHANGED");
    }
    // SAFETY: `words` owns initialized storage of at least `required_bytes` for this query.
    Ok(unsafe {
        slice::from_raw_parts(
            words.as_ptr().cast::<u8>(),
            usize::try_from(required_bytes)?,
        )
    }
    .to_vec())
}

fn wide_z(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn without_terminal_nul<'a>(value: &'a [u16], error: &'static str) -> Result<&'a [u16]> {
    if value.len() < 2 || value.last() != Some(&0) || value[..value.len() - 1].contains(&0) {
        bail!(error);
    }
    Ok(&value[..value.len() - 1])
}

fn sha256_wide(value: &[u16]) -> String {
    let mut hasher = Sha256::new();
    for unit in value {
        hasher.update(unit.to_le_bytes());
    }
    hex::encode(hasher.finalize())
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

fn query_token_statistics(token: HANDLE) -> Result<TOKEN_STATISTICS> {
    // SAFETY: `TOKEN_STATISTICS` is a plain Win32 output structure and is fully initialized by a
    // successful exact-size `GetTokenInformation` call below.
    let mut value = unsafe { std::mem::zeroed::<TOKEN_STATISTICS>() };
    let mut returned = 0_u32;
    if unsafe {
        GetTokenInformation(
            token,
            TokenStatistics,
            (&mut value as *mut TOKEN_STATISTICS).cast(),
            size_of::<TOKEN_STATISTICS>() as u32,
            &mut returned,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    if returned != size_of::<TOKEN_STATISTICS>() as u32 {
        bail!("COMPUTE_PLUGIN_WINDOWS_TOKEN_STATISTICS_SIZE_CHANGED");
    }
    Ok(value)
}

fn token_logon_session_identity_digest(statistics: &TOKEN_STATISTICS) -> Result<String> {
    jcs_sha256_hex(&json!({
        "schema": "elon.compute_plugin.windows_token_logon_session_identity.v1",
        "authentication_id_low": statistics.AuthenticationId.LowPart,
        "authentication_id_high": statistics.AuthenticationId.HighPart,
    }))
}

impl std::fmt::Debug for SealedWindowsRunnerLaunchSecurity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SealedWindowsRunnerLaunchSecurity")
            .field("primary_token", &"<least-rights-owned-handle>")
            .field("process_descriptor", &"<empty-dacl-self-relative>")
            .field("thread_descriptor", &"<empty-dacl-self-relative>")
            .field("token_profile_digest", &"<redacted>")
            .field("owner_and_label_isolation", &"<sealed-uninhabited>")
            .finish()
    }
}
