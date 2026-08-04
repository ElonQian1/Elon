use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ring::signature::{UnparsedPublicKey, ED25519};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::{
    install_plan::{
        SignedComputePluginInstallPlan, COMPUTE_PLUGIN_INSTALL_PLAN_SCHEMA,
        COMPUTE_PLUGIN_INSTALL_PLAN_SIGNATURE_DOMAIN, SIGNED_COMPUTE_PLUGIN_INSTALL_PLAN_SCHEMA,
    },
    plugin_manifest::{
        ComputePluginSignature, SignedComputePluginManifest, COMPUTE_PLUGIN_DIGEST_ALGORITHM,
        COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION, COMPUTE_PLUGIN_MANIFEST_SCHEMA,
        COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN, COMPUTE_PLUGIN_SIGNATURE_ALGORITHM,
        SIGNED_COMPUTE_PLUGIN_MANIFEST_SCHEMA,
    },
};

const I_JSON_MAX_SAFE_INTEGER: i128 = 9_007_199_254_740_991;
const ED25519_PUBLIC_KEY_BYTES: usize = 32;
const ED25519_SIGNATURE_BYTES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ComputePluginEd25519PublicKey([u8; ED25519_PUBLIC_KEY_BYTES]);

impl ComputePluginEd25519PublicKey {
    pub(crate) fn from_standard_base64(value: &str) -> Result<Self> {
        let decoded = decode_canonical_base64(value, "COMPUTE_PLUGIN_KEY_ENCODING")?;
        let bytes: [u8; ED25519_PUBLIC_KEY_BYTES] = decoded.try_into().map_err(|_| {
            anyhow::anyhow!("COMPUTE_PLUGIN_KEY_LENGTH: Ed25519 public key must contain 32 bytes")
        })?;
        Ok(Self(bytes))
    }

    pub(super) fn fingerprint(&self) -> String {
        hex::encode(Sha256::digest(self.0))
    }
}

/// Implementations return only currently trusted publisher keys. Unknown, revoked or wrong-purpose
/// keys return None; callers must not fall back to a key embedded in the manifest.
pub(crate) trait ComputePluginPublisherKeyResolver {
    fn resolve_publisher_key(
        &self,
        publisher_id: &str,
        signing_key_id: &str,
    ) -> Result<Option<ComputePluginEd25519PublicKey>>;
}

/// Control-plane InstallPlan keys intentionally use a separate resolver and namespace.
pub(crate) trait ComputePluginControlPlaneKeyResolver {
    fn resolve_control_plane_key(
        &self,
        signing_key_id: &str,
    ) -> Result<Option<ComputePluginEd25519PublicKey>>;
}

#[derive(Debug, Clone)]
pub(super) struct SignatureVerifiedComputePluginManifest {
    signed: SignedComputePluginManifest,
    verification_key_fingerprint: String,
}

impl SignatureVerifiedComputePluginManifest {
    pub(super) fn signed(&self) -> &SignedComputePluginManifest {
        &self.signed
    }

    pub(super) fn verification_key_fingerprint(&self) -> &str {
        &self.verification_key_fingerprint
    }
}

#[derive(Debug, Clone)]
pub(super) struct SignatureVerifiedComputePluginInstallPlan {
    signed: SignedComputePluginInstallPlan,
    verification_key_fingerprint: String,
}

impl SignatureVerifiedComputePluginInstallPlan {
    pub(super) fn signed(&self) -> &SignedComputePluginInstallPlan {
        &self.signed
    }

    pub(super) fn verification_key_fingerprint(&self) -> &str {
        &self.verification_key_fingerprint
    }
}

pub(super) fn verify_manifest_signature(
    signed: &SignedComputePluginManifest,
    resolver: &dyn ComputePluginPublisherKeyResolver,
) -> Result<SignatureVerifiedComputePluginManifest> {
    if signed.schema != SIGNED_COMPUTE_PLUGIN_MANIFEST_SCHEMA
        || signed.manifest.schema != COMPUTE_PLUGIN_MANIFEST_SCHEMA
    {
        bail!("COMPUTE_PLUGIN_MANIFEST_SCHEMA: unsupported signed manifest schema");
    }
    let key = resolver
        .resolve_publisher_key(
            &signed.manifest.publisher_id,
            &signed.signature.signing_key_id,
        )?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "COMPUTE_PLUGIN_PUBLISHER_KEY_UNTRUSTED: publisher key is unknown or inactive"
            )
        })?;
    verify_jcs_ed25519(
        &signed.manifest,
        &signed.canonicalization,
        &signed.manifest_digest_algorithm,
        &signed.manifest_digest,
        &signed.signature,
        COMPUTE_PLUGIN_MANIFEST_SIGNATURE_DOMAIN,
        &key,
    )?;
    Ok(SignatureVerifiedComputePluginManifest {
        signed: signed.clone(),
        verification_key_fingerprint: key.fingerprint(),
    })
}

pub(super) fn verify_install_plan_signature(
    signed: &SignedComputePluginInstallPlan,
    resolver: &dyn ComputePluginControlPlaneKeyResolver,
) -> Result<SignatureVerifiedComputePluginInstallPlan> {
    if signed.schema != SIGNED_COMPUTE_PLUGIN_INSTALL_PLAN_SCHEMA
        || signed.plan.schema != COMPUTE_PLUGIN_INSTALL_PLAN_SCHEMA
    {
        bail!("COMPUTE_PLUGIN_PLAN_SCHEMA: unsupported signed InstallPlan schema");
    }
    let key = resolver
        .resolve_control_plane_key(&signed.signature.signing_key_id)?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "COMPUTE_PLUGIN_CONTROL_KEY_UNTRUSTED: control-plane key is unknown or inactive"
            )
        })?;
    verify_jcs_ed25519(
        &signed.plan,
        &signed.canonicalization,
        &signed.plan_digest_algorithm,
        &signed.plan_digest,
        &signed.signature,
        COMPUTE_PLUGIN_INSTALL_PLAN_SIGNATURE_DOMAIN,
        &key,
    )?;
    Ok(SignatureVerifiedComputePluginInstallPlan {
        signed: signed.clone(),
        verification_key_fingerprint: key.fingerprint(),
    })
}

pub(super) fn verify_jcs_ed25519(
    payload: &impl Serialize,
    canonicalization: &str,
    digest_algorithm: &str,
    declared_digest: &str,
    signature: &ComputePluginSignature,
    domain: &str,
    key: &ComputePluginEd25519PublicKey,
) -> Result<()> {
    if canonicalization != COMPUTE_PLUGIN_MANIFEST_CANONICALIZATION
        || digest_algorithm != COMPUTE_PLUGIN_DIGEST_ALGORITHM
        || signature.algorithm != COMPUTE_PLUGIN_SIGNATURE_ALGORITHM
    {
        bail!("COMPUTE_PLUGIN_SIGNATURE_SUITE: unsupported canonicalization or algorithm");
    }
    validate_identifier(
        "COMPUTE_PLUGIN_SIGNING_KEY_ID",
        &signature.signing_key_id,
        160,
    )?;
    let canonical = jcs_bytes(payload)?;
    let digest: [u8; 32] = Sha256::digest(canonical).into();
    let actual_digest = hex::encode(digest);
    if declared_digest.len() != 64
        || !declared_digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || declared_digest != actual_digest
    {
        bail!("COMPUTE_PLUGIN_DIGEST_MISMATCH: declared payload digest is not canonical");
    }
    let signature_bytes = decode_canonical_base64(
        &signature.signature_base64,
        "COMPUTE_PLUGIN_SIGNATURE_ENCODING",
    )?;
    if signature_bytes.len() != ED25519_SIGNATURE_BYTES {
        bail!("COMPUTE_PLUGIN_SIGNATURE_LENGTH: Ed25519 signature must contain 64 bytes");
    }
    let mut message = Vec::with_capacity(domain.len() + 1 + digest.len());
    message.extend_from_slice(domain.as_bytes());
    message.push(0);
    message.extend_from_slice(&digest);
    UnparsedPublicKey::new(&ED25519, key.0)
        .verify(&message, &signature_bytes)
        .map_err(|_| {
            anyhow::anyhow!("COMPUTE_PLUGIN_SIGNATURE_INVALID: Ed25519 verification failed")
        })
}

fn decode_canonical_base64(value: &str, code: &str) -> Result<Vec<u8>> {
    if value.is_empty() || value.trim() != value {
        bail!("{code}: Base64 value is empty or contains surrounding whitespace");
    }
    let decoded = STANDARD
        .decode(value)
        .with_context(|| format!("{code}: invalid standard Base64"))?;
    if STANDARD.encode(&decoded) != value {
        bail!("{code}: Base64 value is not in canonical padded form");
    }
    Ok(decoded)
}

fn validate_identifier(code: &str, value: &str, maximum_bytes: usize) -> Result<()> {
    if value.is_empty()
        || value.len() > maximum_bytes
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        bail!("{code}: identifier is empty, oversized or non-canonical");
    }
    Ok(())
}

fn jcs_bytes(payload: &impl Serialize) -> Result<Vec<u8>> {
    let value = serde_json::to_value(payload).context("COMPUTE_PLUGIN_JCS_SERIALIZE")?;
    let mut output = Vec::new();
    write_jcs_value(&value, &mut output)?;
    Ok(output)
}

pub(super) fn jcs_sha256_hex(payload: &impl Serialize) -> Result<String> {
    Ok(hex::encode(Sha256::digest(jcs_bytes(payload)?)))
}

fn write_jcs_value(value: &Value, output: &mut Vec<u8>) -> Result<()> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => {
            output.extend(serde_json::to_vec(value).context("COMPUTE_PLUGIN_JCS_SCALAR")?);
        }
        Value::Number(number) => write_jcs_integer(number, output)?,
        Value::Array(items) => {
            output.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                write_jcs_value(item, output)?;
            }
            output.push(b']');
        }
        Value::Object(object) => {
            output.push(b'{');
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort_by(|left, right| {
                left.encode_utf16()
                    .collect::<Vec<_>>()
                    .cmp(&right.encode_utf16().collect::<Vec<_>>())
            });
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(b',');
                }
                output.extend(serde_json::to_vec(key).context("COMPUTE_PLUGIN_JCS_KEY")?);
                output.push(b':');
                write_jcs_value(&object[key], output)?;
            }
            output.push(b'}');
        }
    }
    Ok(())
}

fn write_jcs_integer(number: &serde_json::Number, output: &mut Vec<u8>) -> Result<()> {
    let value = number
        .as_i64()
        .map(i128::from)
        .or_else(|| number.as_u64().map(i128::from))
        .ok_or_else(|| {
            anyhow::anyhow!("COMPUTE_PLUGIN_JCS_NUMBER: floating-point values are forbidden")
        })?;
    if !(-I_JSON_MAX_SAFE_INTEGER..=I_JSON_MAX_SAFE_INTEGER).contains(&value) {
        bail!("COMPUTE_PLUGIN_JCS_NUMBER: integer exceeds the I-JSON safe range");
    }
    output.extend_from_slice(value.to_string().as_bytes());
    Ok(())
}
