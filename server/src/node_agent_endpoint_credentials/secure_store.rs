use anyhow::{bail, Context, Result};
use base64::Engine as _;

use super::types::{EndpointSecret, WINDOWS_PROTECTION};

const DPAPI_ENTROPY: &[u8] = b"elon.node-endpoint-credential.v1";
const MAX_PROTECTED_BYTES: usize = 16 * 1024;

pub(super) fn require_available() -> Result<()> {
    if !cfg!(windows) {
        bail!(
            "NODE_ENDPOINT_SECRET_PROTECTION_UNAVAILABLE: endpoint credential 仅支持 Windows DPAPI"
        );
    }
    Ok(())
}

pub(super) fn protection_name() -> &'static str {
    if cfg!(windows) {
        WINDOWS_PROTECTION
    } else {
        "UNAVAILABLE"
    }
}

pub(super) fn protect(secret: &EndpointSecret) -> Result<String> {
    let protected = protect_for_current_user(secret.plaintext_bytes())?;
    Ok(base64::engine::general_purpose::STANDARD.encode(protected))
}

pub(super) fn unprotect(protected_base64: &str) -> Result<EndpointSecret> {
    if protected_base64.is_empty() || protected_base64.len() > MAX_PROTECTED_BYTES * 2 {
        bail!("NODE_ENDPOINT_PROTECTED_SECRET_INVALID");
    }
    let protected = base64::engine::general_purpose::STANDARD
        .decode(protected_base64)
        .context("NODE_ENDPOINT_PROTECTED_SECRET_INVALID")?;
    if protected.is_empty() || protected.len() > MAX_PROTECTED_BYTES {
        bail!("NODE_ENDPOINT_PROTECTED_SECRET_INVALID");
    }
    EndpointSecret::from_bytes(unprotect_for_current_user(&protected)?)
}

#[cfg(windows)]
fn protect_for_current_user(plaintext: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Security::Cryptography::{
        CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = blob(plaintext)?;
    let mut entropy = blob(DPAPI_ENTROPY)?;
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &mut input,
            std::ptr::null(),
            &mut entropy,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if ok == 0 {
        bail!(
            "NODE_ENDPOINT_SECRET_PROTECT_FAILED: {}",
            std::io::Error::last_os_error()
        );
    }
    take_local_blob(output)
}

#[cfg(windows)]
fn unprotect_for_current_user(protected: &[u8]) -> Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    };

    let mut input = blob(protected)?;
    let mut entropy = blob(DPAPI_ENTROPY)?;
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    let mut description = std::ptr::null_mut();
    let ok = unsafe {
        CryptUnprotectData(
            &mut input,
            &mut description,
            &mut entropy,
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    if !description.is_null() {
        unsafe {
            LocalFree(description.cast());
        }
    }
    if ok == 0 {
        bail!(
            "NODE_ENDPOINT_SECRET_UNPROTECT_FAILED: {}",
            std::io::Error::last_os_error()
        );
    }
    take_local_blob(output)
}

#[cfg(windows)]
fn blob(bytes: &[u8]) -> Result<windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB> {
    let len = u32::try_from(bytes.len()).context("endpoint secret 超过 Windows DPAPI 输入上限")?;
    Ok(
        windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB {
            cbData: len,
            pbData: bytes.as_ptr().cast_mut(),
        },
    )
}

#[cfg(windows)]
fn take_local_blob(
    output: windows_sys::Win32::Security::Cryptography::CRYPT_INTEGER_BLOB,
) -> Result<Vec<u8>> {
    use windows_sys::Win32::Foundation::LocalFree;

    if output.pbData.is_null() {
        bail!("Windows DPAPI 返回空 endpoint secret");
    }
    if output.cbData == 0 {
        unsafe {
            LocalFree(output.pbData.cast());
        }
        bail!("Windows DPAPI 返回空 endpoint secret");
    }
    let output_bytes =
        unsafe { std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize) };
    let bytes = output_bytes.to_vec();
    output_bytes.fill(0);
    unsafe {
        LocalFree(output.pbData.cast());
    }
    Ok(bytes)
}

#[cfg(not(windows))]
fn protect_for_current_user(_plaintext: &[u8]) -> Result<Vec<u8>> {
    bail!("NODE_ENDPOINT_SECRET_PROTECTION_UNAVAILABLE: endpoint credential 仅支持 Windows DPAPI")
}

#[cfg(not(windows))]
fn unprotect_for_current_user(_protected: &[u8]) -> Result<Vec<u8>> {
    bail!("NODE_ENDPOINT_SECRET_PROTECTION_UNAVAILABLE: endpoint credential 仅支持 Windows DPAPI")
}
