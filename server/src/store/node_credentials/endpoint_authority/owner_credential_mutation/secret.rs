use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ring::rand::{SecureRandom, SystemRandom};
use sha2::{Digest, Sha256};

const ENDPOINT_SECRET_BYTES: usize = 32;

pub(super) struct GeneratedEndpointSecret {
    plaintext: String,
    secret_hash: [u8; 32],
}

impl GeneratedEndpointSecret {
    pub(super) fn secret_hash(&self) -> [u8; 32] {
        self.secret_hash
    }

    pub(super) fn into_plaintext(self) -> String {
        self.plaintext
    }
}

pub(super) fn generate_endpoint_secret() -> Result<GeneratedEndpointSecret> {
    let mut random = [0_u8; ENDPOINT_SECRET_BYTES];
    SystemRandom::new()
        .fill(&mut random)
        .map_err(|_| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_SECRET_CSPRNG_FAILED"))?;
    let plaintext = URL_SAFE_NO_PAD.encode(random);
    random.fill(0);
    let secret_hash = presented_secret_hash(&plaintext);
    Ok(GeneratedEndpointSecret {
        plaintext,
        secret_hash,
    })
}

pub(super) fn presented_secret_hash(plaintext: &str) -> [u8; 32] {
    Sha256::digest(plaintext.as_bytes()).into()
}
