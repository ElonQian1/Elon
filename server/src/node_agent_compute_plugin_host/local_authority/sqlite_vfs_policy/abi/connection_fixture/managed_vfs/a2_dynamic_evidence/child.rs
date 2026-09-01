use std::{fmt, path::Path, process::Child};

use sha2::{Digest, Sha256};
pub(super) mod lock_callback_route_unknown;
pub(super) mod lock_lifecycle;
pub(super) mod lock_local_sibling_contention;
pub(super) mod lock_native_acquire_busy;
mod lock_request_validation;
pub(super) mod lock_stored_poison;
pub(super) mod map_lifecycle;
pub(super) mod map_region_loop;
mod payload;
use payload::validate_actual_payload;

pub(in super::super) const A2_DYNAMIC_CHILD_NONCE_ENV: &str = "ELON_SQLITE_A2_DYNAMIC_CHILD_NONCE";

const REPORT_PREFIX: &str = "ELON_A2_WINDOWS_DYNAMIC_CHILD_V2";
const NONCE_HEX_LEN: usize = 32;
const COMMITMENT_HEX_LEN: usize = 64;
const MAX_CAPTURED_STDOUT_BYTES: usize = 64 * 1024;
const MAX_ACTUAL_PAYLOAD_BYTES: usize = 2_048;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SanitizedPayloadFamily {
    RegistrationShutdown,
    Barrier,
    RegistryLifecycle,
    Unmap,
    JointClose,
    MapQuotient,
    LockQuotient,
}

/// Parent-created nonce which must be installed in the child command before spawn.
///
/// The value is not evidence. `bind` consumes the actual spawned child and turns both into the
/// only object which can wait for and validate that process's report.
pub(in super::super) struct ChildLaunchIdentity {
    nonce: String,
}

pub(in super::super) struct BoundDynamicChild {
    child: Child,
    identity: UniqueChildIdentity,
}

/// Failure from the exact spawned child, carrying the only authority for fallback root cleanup.
#[derive(Debug)]
pub(in super::super) struct DynamicChildFailure {
    code: &'static str,
    exit_confirmed: bool,
    detail: Option<String>,
}

pub(super) struct UniqueChildIdentity {
    pub(super) process_id: u32,
    pub(super) nonce: String,
}

pub(super) struct ReportedChildIdentity {
    pub(super) process_id: u32,
    pub(super) nonce: String,
}

#[derive(PartialEq, Eq)]
pub(super) struct ChildIdentityFingerprint(pub(super) [u8; 32]);

/// Commitment independently recomputable by the parent from the exact canonical child root.
#[derive(PartialEq, Eq)]
pub(super) struct RootCommitment(pub(super) [u8; 32]);

/// Opaque commitment to the real child-owned VFS registration identity.
#[derive(PartialEq, Eq)]
pub(super) struct RegistrationCommitment(pub(super) [u8; 32]);

/// Sealed commitment to the exact canonical actual payload. Its bytes are never exposed and no
/// constructor accepts a caller-provided digest.
#[derive(PartialEq, Eq)]
pub(in super::super) struct SanitizedActualPayloadCommitment(pub(super) [u8; 32]);

impl SanitizedActualPayloadCommitment {
    pub(in super::super) fn matches_payload(&self, payload: &str) -> bool {
        self == &payload_commitment(payload)
    }
}

/// Child stdout reduced to allow-listed identity, opaque bindings and canonical actual fields.
pub(in super::super) struct SanitizedChildReport {
    pub(super) identity: ReportedChildIdentity,
    pub(super) root_commitment: RootCommitment,
    pub(super) registration_commitment: RegistrationCommitment,
    pub(super) actual_payload: String,
    family: SanitizedPayloadFamily,
}

/// Linear proof produced only by waiting the `Child` consumed by `BoundDynamicChild`.
pub(in super::super) struct ValidatedChildProcessReceipt {
    pub(super) identity: ReportedChildIdentity,
    pub(super) root_commitment: RootCommitment,
    pub(super) registration_commitment: RegistrationCommitment,
    pub(super) actual_payload: String,
    pub(super) payload_commitment: SanitizedActualPayloadCommitment,
    family: SanitizedPayloadFamily,
    pub(super) exit_code: i32,
}

impl ChildLaunchIdentity {
    pub(in super::super) fn new() -> Self {
        Self {
            nonce: uuid::Uuid::new_v4().simple().to_string(),
        }
    }

    /// Value to set under `A2_DYNAMIC_CHILD_NONCE_ENV` before spawning the child.
    pub(in super::super) fn env_value(&self) -> &str {
        &self.nonce
    }

    /// Consumes the actual child handle so its PID, exit status and output cannot be spliced.
    pub(in super::super) fn bind(
        self,
        mut child: Child,
    ) -> Result<BoundDynamicChild, DynamicChildFailure> {
        let process_id = child.id();
        if process_id == 0 || !valid_nonce(&self.nonce) {
            return Err(super::capture::abort_child(
                &mut child,
                "A2_DYNAMIC_CHILD_IDENTITY_INVALID",
            ));
        }
        Ok(BoundDynamicChild {
            child,
            identity: UniqueChildIdentity {
                process_id,
                nonce: self.nonce,
            },
        })
    }
}

impl BoundDynamicChild {
    /// Concurrently drains bounded stdout/stderr, waits this exact child, requires success, parses
    /// exactly one report and verifies that the report carries this child's PID and launch nonce.
    pub(in super::super) fn wait_for_successful_report(
        mut self,
    ) -> Result<ValidatedChildProcessReceipt, DynamicChildFailure> {
        let output = super::capture::wait_for_bounded_output(&mut self.child)?;
        if !output.status.success() {
            return Err(DynamicChildFailure::exited("A2_DYNAMIC_CHILD_EXIT_FAILED"));
        }
        let exit_code = output
            .status
            .code()
            .ok_or_else(|| DynamicChildFailure::exited("A2_DYNAMIC_CHILD_EXIT_CODE_UNAVAILABLE"))?;
        let report = SanitizedChildReport::parse_captured_stdout(&output.stdout)
            .map_err(DynamicChildFailure::exited)?;
        if !self.identity.matches(&report.identity) {
            return Err(DynamicChildFailure::exited(
                "A2_DYNAMIC_CHILD_IDENTITY_MISMATCH",
            ));
        }
        let payload_commitment = payload_commitment(&report.actual_payload);
        Ok(ValidatedChildProcessReceipt {
            identity: report.identity,
            root_commitment: report.root_commitment,
            registration_commitment: report.registration_commitment,
            actual_payload: report.actual_payload,
            payload_commitment,
            family: report.family,
            exit_code,
        })
    }
}

impl DynamicChildFailure {
    pub(super) fn exited(code: &'static str) -> Self {
        Self {
            code,
            exit_confirmed: true,
            detail: None,
        }
    }

    pub(super) fn exited_with_detail(code: &'static str, detail: String) -> Self {
        Self {
            code,
            exit_confirmed: true,
            detail: Some(detail),
        }
    }

    pub(super) fn exit_unconfirmed(code: &'static str, detail: String) -> Self {
        Self {
            code,
            exit_confirmed: false,
            detail: Some(detail),
        }
    }

    pub(in super::super) fn exit_confirmed(&self) -> bool {
        self.exit_confirmed
    }
}

impl fmt::Display for DynamicChildFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} (child_exit={})",
            self.code,
            if self.exit_confirmed {
                "confirmed"
            } else {
                "unconfirmed"
            }
        )?;
        if let Some(detail) = &self.detail {
            write!(formatter, ": {detail}")?;
        }
        Ok(())
    }
}

impl std::error::Error for DynamicChildFailure {}

impl SanitizedChildReport {
    /// Rejects a substituted or reparse-point root before the child performs any real VFS work.
    pub(in super::super) fn validate_root_before_exercise(
        launch_nonce: &str,
        root: &Path,
    ) -> Result<(), &'static str> {
        if !valid_nonce(launch_nonce) {
            return Err("A2_DYNAMIC_CHILD_NONCE_INVALID");
        }
        let captured_root =
            super::environment::capture_root_binding(root, std::process::id(), launch_nonce)?;
        if captured_root.canonical_root != root {
            return Err("A2_DYNAMIC_ROOT_NOT_CANONICAL");
        }
        Ok(())
    }

    /// Produces the one child report line. Root and registration commitments are computed from the
    /// real inputs here; the caller cannot inject either digest or expose their raw values.
    pub(in super::super) fn encode_for_current_child(
        launch_nonce: &str,
        root: &Path,
        registration_id: u64,
        actual_payload: &str,
    ) -> Result<String, &'static str> {
        if !valid_nonce(launch_nonce) {
            return Err("A2_DYNAMIC_CHILD_NONCE_INVALID");
        }
        if registration_id == 0 {
            return Err("A2_DYNAMIC_REGISTRATION_ID_INVALID");
        }
        let family = validate_actual_payload(actual_payload)?;
        let process_id = std::process::id();
        let captured_root =
            super::environment::capture_root_binding(root, process_id, launch_nonce)?;
        let payload_commitment = payload_commitment(actual_payload);
        let registration_commitment = registration_commitment(
            process_id,
            launch_nonce,
            &captured_root.commitment,
            &payload_commitment,
            family,
            registration_id,
        );
        Ok(format!(
            "{REPORT_PREFIX}|pid={process_id}|nonce={launch_nonce}|root={}|registration={}|actual={actual_payload}",
            hex::encode(captured_root.commitment.0),
            hex::encode(registration_commitment.0),
        ))
    }

    pub(super) fn parse_captured_stdout(stdout: &[u8]) -> Result<Self, &'static str> {
        if stdout.len() > MAX_CAPTURED_STDOUT_BYTES {
            return Err("A2_DYNAMIC_CHILD_STDOUT_TOO_LARGE");
        }
        let stdout = std::str::from_utf8(stdout).map_err(|_| "A2_DYNAMIC_CHILD_STDOUT_NOT_UTF8")?;
        let mut reports = stdout
            .lines()
            .filter(|line| line.starts_with(REPORT_PREFIX));
        let line = reports.next().ok_or("A2_DYNAMIC_CHILD_REPORT_MISSING")?;
        if reports.next().is_some() {
            return Err("A2_DYNAMIC_CHILD_REPORT_DUPLICATE");
        }
        parse_report_line(line)
    }

    #[cfg(test)]
    pub(super) fn actual_payload(&self) -> &str {
        &self.actual_payload
    }
}

impl ValidatedChildProcessReceipt {
    pub(in super::super) fn actual_payload(&self) -> &str {
        &self.actual_payload
    }

    pub(super) fn fingerprint(&self) -> ChildIdentityFingerprint {
        self.identity.fingerprint()
    }

    pub(super) fn matches_registration_id(&self, registration_id: u64) -> bool {
        registration_id != 0
            && self.registration_commitment
                == registration_commitment(
                    self.identity.process_id,
                    &self.identity.nonce,
                    &self.root_commitment,
                    &self.payload_commitment,
                    self.family,
                    registration_id,
                )
    }

    pub(super) fn matches_family(&self, family: SanitizedPayloadFamily) -> bool {
        self.family == family
    }

    pub(super) fn redacted_payload_fingerprint(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"elon-a2-windows-dynamic-redacted-actual-v2\0");
        hasher.update(self.identity.process_id.to_le_bytes());
        hasher.update(self.identity.nonce.as_bytes());
        hasher.update(self.payload_commitment.0);
        hasher.finalize().into()
    }
}

impl UniqueChildIdentity {
    fn matches(&self, reported: &ReportedChildIdentity) -> bool {
        self.process_id == reported.process_id && self.nonce == reported.nonce
    }
}

impl ReportedChildIdentity {
    fn fingerprint(&self) -> ChildIdentityFingerprint {
        identity_fingerprint(self.process_id, &self.nonce)
    }
}

fn parse_report_line(line: &str) -> Result<SanitizedChildReport, &'static str> {
    if line.len() > MAX_ACTUAL_PAYLOAD_BYTES + 320 || line.contains('\r') || line.contains('\n') {
        return Err("A2_DYNAMIC_CHILD_REPORT_INVALID");
    }
    let mut parts = line.split('|');
    if parts.next() != Some(REPORT_PREFIX) {
        return Err("A2_DYNAMIC_CHILD_REPORT_VERSION_INVALID");
    }
    let process_id_text = parts
        .next()
        .and_then(|part| part.strip_prefix("pid="))
        .ok_or("A2_DYNAMIC_CHILD_REPORT_PID_MISSING")?;
    let process_id = process_id_text
        .parse::<u32>()
        .map_err(|_| "A2_DYNAMIC_CHILD_REPORT_PID_INVALID")?;
    if process_id == 0 || process_id.to_string() != process_id_text {
        return Err("A2_DYNAMIC_CHILD_REPORT_PID_INVALID");
    }
    let nonce = parts
        .next()
        .and_then(|part| part.strip_prefix("nonce="))
        .ok_or("A2_DYNAMIC_CHILD_REPORT_NONCE_MISSING")?;
    if !valid_nonce(nonce) {
        return Err("A2_DYNAMIC_CHILD_REPORT_NONCE_INVALID");
    }
    let root_commitment = RootCommitment(parse_commitment(
        parts
            .next()
            .and_then(|part| part.strip_prefix("root="))
            .ok_or("A2_DYNAMIC_CHILD_REPORT_ROOT_MISSING")?,
    )?);
    let registration_commitment = RegistrationCommitment(parse_commitment(
        parts
            .next()
            .and_then(|part| part.strip_prefix("registration="))
            .ok_or("A2_DYNAMIC_CHILD_REPORT_REGISTRATION_MISSING")?,
    )?);
    let actual_payload = parts
        .next()
        .and_then(|part| part.strip_prefix("actual="))
        .ok_or("A2_DYNAMIC_CHILD_REPORT_ACTUAL_MISSING")?;
    if parts.next().is_some() {
        return Err("A2_DYNAMIC_CHILD_REPORT_FIELDS_INVALID");
    }
    let family = validate_actual_payload(actual_payload)?;
    Ok(SanitizedChildReport {
        identity: ReportedChildIdentity {
            process_id,
            nonce: nonce.to_owned(),
        },
        root_commitment,
        registration_commitment,
        actual_payload: actual_payload.to_owned(),
        family,
    })
}

fn parse_commitment(value: &str) -> Result<[u8; 32], &'static str> {
    if value.len() != COMMITMENT_HEX_LEN
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("A2_DYNAMIC_CHILD_REPORT_COMMITMENT_INVALID");
    }
    let mut decoded = [0u8; 32];
    hex::decode_to_slice(value, &mut decoded)
        .map_err(|_| "A2_DYNAMIC_CHILD_REPORT_COMMITMENT_INVALID")?;
    Ok(decoded)
}

fn valid_nonce(value: &str) -> bool {
    value.len() == NONCE_HEX_LEN
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn identity_fingerprint(process_id: u32, nonce: &str) -> ChildIdentityFingerprint {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-windows-dynamic-child-v2\0");
    hasher.update(process_id.to_le_bytes());
    hasher.update(nonce.as_bytes());
    ChildIdentityFingerprint(hasher.finalize().into())
}

fn payload_commitment(payload: &str) -> SanitizedActualPayloadCommitment {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-windows-dynamic-actual-v2\0");
    hasher.update((payload.len() as u64).to_le_bytes());
    hasher.update(payload.as_bytes());
    SanitizedActualPayloadCommitment(hasher.finalize().into())
}

fn registration_commitment(
    process_id: u32,
    nonce: &str,
    root: &RootCommitment,
    payload: &SanitizedActualPayloadCommitment,
    family: SanitizedPayloadFamily,
    registration_id: u64,
) -> RegistrationCommitment {
    let mut hasher = Sha256::new();
    hasher.update(b"elon-a2-windows-dynamic-registration-v2\0");
    hasher.update(process_id.to_le_bytes());
    hasher.update(nonce.as_bytes());
    hasher.update(root.0);
    hasher.update(payload.0);
    hasher.update([match family {
        SanitizedPayloadFamily::RegistrationShutdown => 1,
        SanitizedPayloadFamily::Barrier => 2,
        SanitizedPayloadFamily::RegistryLifecycle => 3,
        SanitizedPayloadFamily::Unmap => 4,
        SanitizedPayloadFamily::JointClose => 5,
        SanitizedPayloadFamily::MapQuotient => 6,
        SanitizedPayloadFamily::LockQuotient => 7,
    }]);
    hasher.update(registration_id.to_le_bytes());
    RegistrationCommitment(hasher.finalize().into())
}

#[cfg(test)]
pub(super) fn validate_payload_for_test(payload: &str) -> Result<(), &'static str> {
    validate_actual_payload(payload).map(|_| ())
}

#[cfg(test)]
pub(super) fn validated_receipt_for_record_test(
    payload: &str,
    registration_id: u64,
) -> Result<ValidatedChildProcessReceipt, &'static str> {
    if registration_id == 0 {
        return Err("A2_DYNAMIC_REGISTRATION_ID_INVALID");
    }
    let family = validate_actual_payload(payload)?;
    let identity = ReportedChildIdentity {
        process_id: std::process::id(),
        nonce: "0123456789abcdef0123456789abcdef".to_owned(),
    };
    let root_commitment = RootCommitment([0x5a; 32]);
    let payload_commitment = payload_commitment(payload);
    let registration_commitment = registration_commitment(
        identity.process_id,
        &identity.nonce,
        &root_commitment,
        &payload_commitment,
        family,
        registration_id,
    );
    Ok(ValidatedChildProcessReceipt {
        identity,
        root_commitment,
        registration_commitment,
        actual_payload: payload.to_owned(),
        payload_commitment,
        family,
        exit_code: 0,
    })
}
