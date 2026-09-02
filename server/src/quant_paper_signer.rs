use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use hmac::{Hmac, Mac};
use ring::signature::Ed25519KeyPair;
#[cfg(test)]
use ring::signature::KeyPair;
use serde::Serialize;
use sha2::Sha256;

const SIGNING_SEED_ENV: &str = "YILONG_QUANT_PAPER_SIGNING_SEED_BASE64URL";
const SUBJECT_SECRET_ENV: &str = "YILONG_QUANT_PAPER_SUBJECT_SECRET_BASE64URL";
const KEY_ID_ENV: &str = "YILONG_QUANT_PAPER_SIGNING_KEY_ID";

type SubjectMac = Hmac<Sha256>;

pub(crate) struct PaperGrantSigner {
    key_id: String,
    signing_key: Ed25519KeyPair,
    subject_secret: [u8; 32],
}

#[derive(Debug)]
pub(crate) enum SignerConfigError {
    Disabled,
    Invalid,
}

impl PaperGrantSigner {
    pub(crate) fn from_env() -> Result<Self, SignerConfigError> {
        let key_id = std::env::var(KEY_ID_ENV).ok();
        let signing_seed = std::env::var(SIGNING_SEED_ENV).ok();
        let subject_secret = std::env::var(SUBJECT_SECRET_ENV).ok();
        match (key_id, signing_seed, subject_secret) {
            (None, None, None) => Err(SignerConfigError::Disabled),
            (Some(key_id), Some(seed), Some(subject)) => {
                let seed = URL_SAFE_NO_PAD
                    .decode(seed)
                    .map_err(|_| SignerConfigError::Invalid)?;
                let subject = URL_SAFE_NO_PAD
                    .decode(subject)
                    .map_err(|_| SignerConfigError::Invalid)?;
                Self::from_material(key_id, &seed, &subject)
            }
            _ => Err(SignerConfigError::Invalid),
        }
    }

    pub(crate) fn from_material(
        key_id: String,
        signing_seed: &[u8],
        subject_secret: &[u8],
    ) -> Result<Self, SignerConfigError> {
        if !valid_identifier(&key_id, 3, 64) || subject_secret.len() != 32 {
            return Err(SignerConfigError::Invalid);
        }
        let signing_key = Ed25519KeyPair::from_seed_unchecked(signing_seed)
            .map_err(|_| SignerConfigError::Invalid)?;
        let mut pinned_subject_secret = [0_u8; 32];
        pinned_subject_secret.copy_from_slice(subject_secret);
        Ok(Self {
            key_id,
            signing_key,
            subject_secret: pinned_subject_secret,
        })
    }

    pub(crate) fn key_id(&self) -> &str {
        &self.key_id
    }

    #[cfg(test)]
    pub(crate) fn public_key_bytes(&self) -> &[u8] {
        self.signing_key.public_key().as_ref()
    }

    pub(crate) fn sign_token<T: Serialize>(&self, prefix: &str, claims: &T) -> Result<String, ()> {
        let payload = serde_json::to_vec(claims).map_err(|_| ())?;
        let signature = self.signing_key.sign(&payload);
        Ok(format!(
            "{prefix}.{}.{}",
            URL_SAFE_NO_PAD.encode(&payload),
            URL_SAFE_NO_PAD.encode(signature.as_ref())
        ))
    }

    pub(crate) fn participant_ref(&self, user_id: &str) -> Result<String, ()> {
        let mut mac = SubjectMac::new_from_slice(&self.subject_secret).map_err(|_| ())?;
        mac.update(b"yilong.quant.paper.subject.v1\0");
        mac.update(user_id.as_bytes());
        let bytes = mac.finalize().into_bytes();
        Ok(format!("yp1_{}", lowercase_hex(&bytes[..20])))
    }
}

fn valid_identifier(value: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn lowercase_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{UnparsedPublicKey, ED25519};

    #[test]
    fn deterministic_material_signs_verifiable_payloads() {
        let signer =
            PaperGrantSigner::from_material("paper-key-test".to_owned(), &[7; 32], &[11; 32])
                .unwrap();
        let token = signer
            .sign_token("test1", &serde_json::json!({ "simulated": true }))
            .unwrap();
        let segments = token.split('.').collect::<Vec<_>>();
        let payload = URL_SAFE_NO_PAD.decode(segments[1]).unwrap();
        let signature = URL_SAFE_NO_PAD.decode(segments[2]).unwrap();

        assert_eq!(segments[0], "test1");
        UnparsedPublicKey::new(&ED25519, signer.public_key_bytes())
            .verify(&payload, &signature)
            .unwrap();
        assert_eq!(
            signer.participant_ref("user-1"),
            signer.participant_ref("user-1")
        );
    }

    #[test]
    fn invalid_material_fails_closed() {
        assert!(
            PaperGrantSigner::from_material("bad key".to_owned(), &[7; 32], &[11; 32]).is_err()
        );
        assert!(
            PaperGrantSigner::from_material("valid-key".to_owned(), &[7; 31], &[11; 32]).is_err()
        );
        assert!(
            PaperGrantSigner::from_material("valid-key".to_owned(), &[7; 32], &[11; 31]).is_err()
        );
    }
}
