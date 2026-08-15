use anyhow::{bail, Result};
use ring::constant_time::verify_slices_are_equal;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use super::{
    crypto::random_array32,
    transport::{
        AuthenticatedExternalPoolAdapterSession, AuthenticatedExternalPoolAdapterSessionFrame,
        ExternalPoolAdapterSessionFrameKind,
    },
};

const DELIVERY_ROOT_DOMAIN: &[u8] =
    b"elon.external_pool_adapter.ephemeral_bundle_delivery.root.v1\0";
const DELIVERY_MAGIC: &[u8; 4] = b"ELSD";
const DELIVERY_VERSION: u8 = 1;
const DELIVERY_FLAGS: u16 = 0;
const DELIVERY_BEGIN: u8 = 1;
const DELIVERY_RECEIPT: u8 = 2;
const DELIVERY_COMMIT: u8 = 3;
const DELIVERY_READY: u8 = 4;
const DELIVERY_SHUTDOWN: u8 = 5;
const DELIVERY_SHUTDOWN_ACK: u8 = 6;
const BEGIN_BYTES: usize = 88;
const SIGNAL_BYTES: usize = 40;
const MAX_CANONICAL_GENERATION: u64 = 9_007_199_254_740_991;

/// Host-side preparation for one delivery. It owns no config or credential bytes.
pub struct PreparedExternalPoolAdapterEphemeralBundleDelivery {
    nonce: Zeroizing<[u8; 32]>,
    generation: u64,
    config_size: u32,
    credential_size: u32,
    config_sha256: [u8; 32],
    credential_sha256: [u8; 32],
    bundle_root: [u8; 32],
}

/// Host proof that the child validated and committed the exact delivery root.
pub struct ExternalPoolAdapterEphemeralBundleDeliveryHostReceipt {
    bundle_root: [u8; 32],
}

/// Child-only, zeroize-on-drop payload. It is intentionally not Clone, Debug, or serializable.
pub struct DeliveredExternalPoolAdapterEphemeralBundle {
    generation: u64,
    bundle_root: [u8; 32],
    config: Zeroizing<Vec<u8>>,
    credential: Zeroizing<Vec<u8>>,
}

pub fn prepare_external_pool_adapter_ephemeral_bundle_delivery(
    generation: u64,
    config: &[u8],
    credential: &[u8],
) -> Result<PreparedExternalPoolAdapterEphemeralBundleDelivery> {
    validate_material(generation, config, credential)?;
    let nonce = Zeroizing::new(random_array32()?);
    let config_sha256: [u8; 32] = Sha256::digest(config).into();
    let credential_sha256: [u8; 32] = Sha256::digest(credential).into();
    let config_size = u32::try_from(config.len())?;
    let credential_size = u32::try_from(credential.len())?;
    let bundle_root = delivery_root(
        generation,
        config_size,
        credential_size,
        &nonce,
        &config_sha256,
        &credential_sha256,
    );
    Ok(PreparedExternalPoolAdapterEphemeralBundleDelivery {
        nonce,
        generation,
        config_size,
        credential_size,
        config_sha256,
        credential_sha256,
        bundle_root,
    })
}

impl PreparedExternalPoolAdapterEphemeralBundleDelivery {
    pub fn bundle_root_hex(&self) -> String {
        hex::encode(self.bundle_root)
    }

    pub fn deliver(
        self,
        session: &mut AuthenticatedExternalPoolAdapterSession,
        expected_session_bundle_root: &str,
        config: &[u8],
        credential: &[u8],
    ) -> Result<ExternalPoolAdapterEphemeralBundleDeliveryHostReceipt> {
        if self
            .validate_exact(expected_session_bundle_root, config, credential)
            .is_err()
        {
            session.terminate();
            bail!("ephemeral bundle delivery authority drifted");
        }
        session.send(
            ExternalPoolAdapterSessionFrameKind::Control,
            &self.begin_payload(),
        )?;
        session.send(ExternalPoolAdapterSessionFrameKind::Config, config)?;
        session.send(ExternalPoolAdapterSessionFrameKind::Credential, credential)?;
        receive_signal(session, DELIVERY_RECEIPT, &self.bundle_root)?;
        session.send(
            ExternalPoolAdapterSessionFrameKind::Control,
            &signal_payload(DELIVERY_COMMIT, &self.bundle_root),
        )?;
        receive_signal(session, DELIVERY_READY, &self.bundle_root)?;
        Ok(ExternalPoolAdapterEphemeralBundleDeliveryHostReceipt {
            bundle_root: self.bundle_root,
        })
    }

    fn validate_exact(
        &self,
        expected_session_bundle_root: &str,
        config: &[u8],
        credential: &[u8],
    ) -> Result<()> {
        validate_material(self.generation, config, credential)?;
        let expected = decode_root(expected_session_bundle_root)?;
        let mut config_sha256: [u8; 32] = Sha256::digest(config).into();
        let mut credential_sha256: [u8; 32] = Sha256::digest(credential).into();
        let exact = config.len() == self.config_size as usize
            && credential.len() == self.credential_size as usize
            && verify_slices_are_equal(&expected, &self.bundle_root).is_ok()
            && verify_slices_are_equal(&config_sha256, &self.config_sha256).is_ok()
            && verify_slices_are_equal(&credential_sha256, &self.credential_sha256).is_ok();
        config_sha256.zeroize();
        credential_sha256.zeroize();
        if !exact {
            bail!("ephemeral bundle delivery authority drifted");
        }
        Ok(())
    }

    fn begin_payload(&self) -> Zeroizing<Vec<u8>> {
        let mut payload = Zeroizing::new(Vec::with_capacity(BEGIN_BYTES));
        encode_prefix(&mut payload, DELIVERY_BEGIN);
        payload.extend_from_slice(&self.generation.to_be_bytes());
        payload.extend_from_slice(&self.config_size.to_be_bytes());
        payload.extend_from_slice(&self.credential_size.to_be_bytes());
        payload.extend_from_slice(&self.nonce[..]);
        payload.extend_from_slice(&self.bundle_root);
        payload
    }
}

impl Drop for PreparedExternalPoolAdapterEphemeralBundleDelivery {
    fn drop(&mut self) {
        self.generation.zeroize();
        self.config_size.zeroize();
        self.credential_size.zeroize();
        self.config_sha256.zeroize();
        self.credential_sha256.zeroize();
        self.bundle_root.zeroize();
    }
}

impl ExternalPoolAdapterEphemeralBundleDeliveryHostReceipt {
    pub fn bundle_root_hex(&self) -> String {
        hex::encode(self.bundle_root)
    }

    pub fn shutdown(mut self, session: &mut AuthenticatedExternalPoolAdapterSession) -> Result<()> {
        session.send(
            ExternalPoolAdapterSessionFrameKind::Control,
            &signal_payload(DELIVERY_SHUTDOWN, &self.bundle_root),
        )?;
        receive_signal(session, DELIVERY_SHUTDOWN_ACK, &self.bundle_root)?;
        self.bundle_root.zeroize();
        Ok(())
    }
}

impl Drop for ExternalPoolAdapterEphemeralBundleDeliveryHostReceipt {
    fn drop(&mut self) {
        self.bundle_root.zeroize();
    }
}

pub fn receive_external_pool_adapter_ephemeral_bundle(
    session: &mut AuthenticatedExternalPoolAdapterSession,
    expected_session_bundle_root: &str,
) -> Result<DeliveredExternalPoolAdapterEphemeralBundle> {
    let begin = session.receive()?;
    receive_external_pool_adapter_ephemeral_bundle_from_begin(
        session,
        expected_session_bundle_root,
        begin,
    )
}

pub fn receive_external_pool_adapter_ephemeral_bundle_from_begin(
    session: &mut AuthenticatedExternalPoolAdapterSession,
    expected_session_bundle_root: &str,
    begin: AuthenticatedExternalPoolAdapterSessionFrame,
) -> Result<DeliveredExternalPoolAdapterEphemeralBundle> {
    let expected_root = match decode_root(expected_session_bundle_root) {
        Ok(value) => value,
        Err(error) => return terminal(session, error),
    };
    let begin = match parse_begin(begin) {
        Ok(value) => value,
        Err(error) => return terminal(session, error),
    };
    if verify_slices_are_equal(&begin.bundle_root, &expected_root).is_err() {
        return terminal_error(session);
    }
    let config = receive_material(
        session,
        ExternalPoolAdapterSessionFrameKind::Config,
        begin.config_size,
    )?;
    let credential = receive_material(
        session,
        ExternalPoolAdapterSessionFrameKind::Credential,
        begin.credential_size,
    )?;
    let mut config_sha256: [u8; 32] = Sha256::digest(&config[..]).into();
    let mut credential_sha256: [u8; 32] = Sha256::digest(&credential[..]).into();
    let actual_root = delivery_root(
        begin.generation,
        begin.config_size,
        begin.credential_size,
        &begin.nonce,
        &config_sha256,
        &credential_sha256,
    );
    config_sha256.zeroize();
    credential_sha256.zeroize();
    if verify_slices_are_equal(&actual_root, &expected_root).is_err() {
        return terminal_error(session);
    }
    session.send(
        ExternalPoolAdapterSessionFrameKind::Control,
        &signal_payload(DELIVERY_RECEIPT, &expected_root),
    )?;
    receive_signal(session, DELIVERY_COMMIT, &expected_root)?;
    session.send(
        ExternalPoolAdapterSessionFrameKind::Control,
        &signal_payload(DELIVERY_READY, &expected_root),
    )?;
    Ok(DeliveredExternalPoolAdapterEphemeralBundle {
        generation: begin.generation,
        bundle_root: expected_root,
        config,
        credential,
    })
}

impl DeliveredExternalPoolAdapterEphemeralBundle {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn config(&self) -> &[u8] {
        &self.config
    }

    pub fn credential(&self) -> &[u8] {
        &self.credential
    }

    pub fn wait_for_shutdown(
        mut self,
        session: &mut AuthenticatedExternalPoolAdapterSession,
    ) -> Result<()> {
        receive_signal(session, DELIVERY_SHUTDOWN, &self.bundle_root)?;
        let root = self.bundle_root;
        self.generation.zeroize();
        self.bundle_root.zeroize();
        self.config.zeroize();
        self.credential.zeroize();
        session.send(
            ExternalPoolAdapterSessionFrameKind::Control,
            &signal_payload(DELIVERY_SHUTDOWN_ACK, &root),
        )?;
        Ok(())
    }
}

impl Drop for DeliveredExternalPoolAdapterEphemeralBundle {
    fn drop(&mut self) {
        self.generation.zeroize();
        self.bundle_root.zeroize();
    }
}

struct DeliveryBegin {
    generation: u64,
    config_size: u32,
    credential_size: u32,
    nonce: Zeroizing<[u8; 32]>,
    bundle_root: [u8; 32],
}

fn parse_begin(frame: AuthenticatedExternalPoolAdapterSessionFrame) -> Result<DeliveryBegin> {
    if frame.kind() != ExternalPoolAdapterSessionFrameKind::Control
        || frame.payload().len() != BEGIN_BYTES
        || !valid_prefix(frame.payload(), DELIVERY_BEGIN)
    {
        bail!("ephemeral bundle delivery frame rejected");
    }
    let payload = frame.payload();
    let generation = u64::from_be_bytes(payload[8..16].try_into()?);
    let config_size = u32::from_be_bytes(payload[16..20].try_into()?);
    let credential_size = u32::from_be_bytes(payload[20..24].try_into()?);
    if generation == 0
        || generation > MAX_CANONICAL_GENERATION
        || config_size == 0
        || config_size as usize
            > ExternalPoolAdapterSessionFrameKind::Config.maximum_payload_bytes()
        || credential_size == 0
        || credential_size as usize
            > ExternalPoolAdapterSessionFrameKind::Credential.maximum_payload_bytes()
    {
        bail!("ephemeral bundle delivery frame rejected");
    }
    let mut nonce = Zeroizing::new([0_u8; 32]);
    nonce.copy_from_slice(&payload[24..56]);
    let mut bundle_root = [0_u8; 32];
    bundle_root.copy_from_slice(&payload[56..88]);
    if nonce.iter().all(|byte| *byte == 0) || bundle_root.iter().all(|byte| *byte == 0) {
        bail!("ephemeral bundle delivery frame rejected");
    }
    Ok(DeliveryBegin {
        generation,
        config_size,
        credential_size,
        nonce,
        bundle_root,
    })
}

fn receive_material(
    session: &mut AuthenticatedExternalPoolAdapterSession,
    expected_kind: ExternalPoolAdapterSessionFrameKind,
    expected_size: u32,
) -> Result<Zeroizing<Vec<u8>>> {
    let frame = session.receive()?;
    if frame.kind() != expected_kind || frame.payload().len() != expected_size as usize {
        return terminal_error(session);
    }
    Ok(Zeroizing::new(frame.payload().to_vec()))
}

fn receive_signal(
    session: &mut AuthenticatedExternalPoolAdapterSession,
    expected_kind: u8,
    expected_root: &[u8; 32],
) -> Result<()> {
    let frame = session.receive()?;
    if frame.kind() != ExternalPoolAdapterSessionFrameKind::Control
        || frame.payload().len() != SIGNAL_BYTES
        || !valid_prefix(frame.payload(), expected_kind)
        || verify_slices_are_equal(&frame.payload()[8..40], expected_root).is_err()
    {
        return terminal_error(session);
    }
    Ok(())
}

fn signal_payload(kind: u8, root: &[u8; 32]) -> Zeroizing<Vec<u8>> {
    let mut payload = Zeroizing::new(Vec::with_capacity(SIGNAL_BYTES));
    encode_prefix(&mut payload, kind);
    payload.extend_from_slice(root);
    payload
}

fn encode_prefix(payload: &mut Vec<u8>, kind: u8) {
    payload.extend_from_slice(DELIVERY_MAGIC);
    payload.push(DELIVERY_VERSION);
    payload.push(kind);
    payload.extend_from_slice(&DELIVERY_FLAGS.to_be_bytes());
}

fn valid_prefix(payload: &[u8], kind: u8) -> bool {
    payload.len() >= 8
        && &payload[..4] == DELIVERY_MAGIC
        && payload[4] == DELIVERY_VERSION
        && payload[5] == kind
        && u16::from_be_bytes([payload[6], payload[7]]) == DELIVERY_FLAGS
}

fn validate_material(generation: u64, config: &[u8], credential: &[u8]) -> Result<()> {
    if generation == 0
        || generation > MAX_CANONICAL_GENERATION
        || config.is_empty()
        || config.len() > ExternalPoolAdapterSessionFrameKind::Config.maximum_payload_bytes()
        || credential.is_empty()
        || credential.len()
            > ExternalPoolAdapterSessionFrameKind::Credential.maximum_payload_bytes()
    {
        bail!("ephemeral bundle delivery material rejected");
    }
    Ok(())
}

fn delivery_root(
    generation: u64,
    config_size: u32,
    credential_size: u32,
    nonce: &[u8; 32],
    config_sha256: &[u8; 32],
    credential_sha256: &[u8; 32],
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(DELIVERY_ROOT_DOMAIN);
    digest.update(generation.to_be_bytes());
    digest.update(config_size.to_be_bytes());
    digest.update(credential_size.to_be_bytes());
    digest.update(nonce);
    digest.update(config_sha256);
    digest.update(credential_sha256);
    digest.finalize().into()
}

fn decode_root(value: &str) -> Result<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("ephemeral bundle delivery root rejected");
    }
    let mut root = [0_u8; 32];
    hex::decode_to_slice(value, &mut root)?;
    if root.iter().all(|byte| *byte == 0) {
        bail!("ephemeral bundle delivery root rejected");
    }
    Ok(root)
}

fn terminal<T>(
    session: &mut AuthenticatedExternalPoolAdapterSession,
    error: anyhow::Error,
) -> Result<T> {
    session.terminate();
    Err(error)
}

fn terminal_error<T>(session: &mut AuthenticatedExternalPoolAdapterSession) -> Result<T> {
    session.terminate();
    bail!("ephemeral bundle delivery protocol rejected")
}
