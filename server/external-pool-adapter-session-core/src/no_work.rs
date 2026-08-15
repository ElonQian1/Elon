use std::time::Duration;

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

const PROBE_MAGIC: &[u8; 4] = b"ELNW";
const PROBE_VERSION: u8 = 1;
const PROBE_FLAGS: u16 = 0;
const PROBE_BEGIN: u8 = 1;
const PROBE_REQUEST: u8 = 2;
const PROBE_RESPONSE: u8 = 3;
const PROBE_RECEIPT: u8 = 4;
const REQUEST_HEADER_BYTES: usize = 48;
const RESPONSE_HEADER_BYTES: usize = 44;
const RECEIPT_BYTES: usize = 136;
const MAX_REQUEST_BYTES: usize = 16_384;
const MAX_RESPONSE_BYTES: usize = 65_536;
const MAX_PROBE_TIMEOUT: Duration = Duration::from_millis(15_000);
const PROBE_ROOT_DOMAIN: &[u8] = b"elon.external_pool_adapter.no_work_probe.root.v1\0";

/// Host-owned request produced by the authenticated child. Raw bytes never leave the relay seam.
pub struct ExternalPoolAdapterNoWorkProbeHostRequest {
    nonce: Zeroizing<[u8; 32]>,
    request: Zeroizing<Vec<u8>>,
    request_sha256: [u8; 32],
    expected_response_bytes: u32,
}

/// Host proof that the same authenticated child semantically accepted the exact response.
pub struct ExternalPoolAdapterNoWorkProbeHostReceipt {
    probe_nonce_digest: [u8; 32],
    probe_root: [u8; 32],
    request_sha256: [u8; 32],
    response_sha256: [u8; 32],
    request_bytes: u32,
    response_bytes: u32,
}

/// Receives exactly one child-generated request from the authenticated ELSP control channel.
pub fn receive_external_pool_adapter_no_work_probe_request(
    session: &mut AuthenticatedExternalPoolAdapterSession,
    timeout: Duration,
) -> Result<ExternalPoolAdapterNoWorkProbeHostRequest> {
    if !valid_timeout(timeout) {
        return terminal_error(session);
    }
    session.send(
        ExternalPoolAdapterSessionFrameKind::Control,
        &prefix_payload(PROBE_BEGIN),
    )?;
    let frame = session.receive_with_timeout(timeout)?;
    match parse_request(frame) {
        Ok(request) => Ok(request),
        Err(error) => terminal(session, error),
    }
}

impl ExternalPoolAdapterNoWorkProbeHostRequest {
    pub fn request(&self) -> &[u8] {
        &self.request
    }

    pub fn expected_response_bytes(&self) -> usize {
        self.expected_response_bytes as usize
    }

    /// Returns a receipt only after the child validates the exact response and authenticates the
    /// resulting probe root. This consumes the request, preventing a second completion.
    pub fn complete(
        mut self,
        session: &mut AuthenticatedExternalPoolAdapterSession,
        response: &[u8],
    ) -> Result<ExternalPoolAdapterNoWorkProbeHostReceipt> {
        if validate_response_size(response, self.expected_response_bytes).is_err() {
            session.terminate();
            bail!("no-work probe response rejected");
        }
        let mut response_sha256: [u8; 32] = Sha256::digest(response).into();
        let probe_root = probe_root(
            &self.nonce,
            self.request.len() as u32,
            self.expected_response_bytes,
            &self.request_sha256,
            &response_sha256,
        );
        session.send(
            ExternalPoolAdapterSessionFrameKind::Control,
            &response_payload(&self.nonce, response),
        )?;
        let receipt = session.receive()?;
        if !valid_receipt(
            &receipt,
            &self.nonce,
            &self.request_sha256,
            &response_sha256,
            &probe_root,
        ) {
            response_sha256.zeroize();
            session.terminate();
            bail!("no-work probe child receipt rejected");
        }
        let output = ExternalPoolAdapterNoWorkProbeHostReceipt {
            probe_nonce_digest: Sha256::digest(&self.nonce[..]).into(),
            probe_root,
            request_sha256: self.request_sha256,
            response_sha256,
            request_bytes: self.request.len() as u32,
            response_bytes: self.expected_response_bytes,
        };
        self.request_sha256.zeroize();
        self.expected_response_bytes.zeroize();
        Ok(output)
    }
}

impl Drop for ExternalPoolAdapterNoWorkProbeHostRequest {
    fn drop(&mut self) {
        self.request_sha256.zeroize();
        self.expected_response_bytes.zeroize();
    }
}

impl ExternalPoolAdapterNoWorkProbeHostReceipt {
    pub fn probe_nonce_digest_hex(&self) -> String {
        hex::encode(self.probe_nonce_digest)
    }

    pub fn probe_root_hex(&self) -> String {
        hex::encode(self.probe_root)
    }

    pub fn request_sha256_hex(&self) -> String {
        hex::encode(self.request_sha256)
    }

    pub fn response_sha256_hex(&self) -> String {
        hex::encode(self.response_sha256)
    }

    pub fn request_bytes(&self) -> u32 {
        self.request_bytes
    }

    pub fn response_bytes(&self) -> u32 {
        self.response_bytes
    }
}

impl Drop for ExternalPoolAdapterNoWorkProbeHostReceipt {
    fn drop(&mut self) {
        self.probe_nonce_digest.zeroize();
        self.probe_root.zeroize();
        self.request_sha256.zeroize();
        self.response_sha256.zeroize();
        self.request_bytes.zeroize();
        self.response_bytes.zeroize();
    }
}

/// Child-side one-shot probe. The validation callback represents Adapter-specific no-task
/// semantics; a callback failure terminates the authenticated session without a receipt.
pub fn execute_external_pool_adapter_no_work_probe(
    session: &mut AuthenticatedExternalPoolAdapterSession,
    request: &[u8],
    expected_response_bytes: usize,
    timeout: Duration,
    validate_response: impl FnOnce(&[u8]) -> Result<()>,
) -> Result<()> {
    if validate_request_size(request).is_err()
        || expected_response_bytes == 0
        || expected_response_bytes > MAX_RESPONSE_BYTES
        || !valid_timeout(timeout)
    {
        return terminal_error(session);
    }
    let begin = session.receive_with_timeout(timeout)?;
    if begin.kind() != ExternalPoolAdapterSessionFrameKind::Control
        || begin.payload().len() != 8
        || !valid_prefix(begin.payload(), PROBE_BEGIN)
    {
        return terminal_error(session);
    }
    let nonce = Zeroizing::new(random_array32()?);
    let mut request_sha256: [u8; 32] = Sha256::digest(request).into();
    session.send(
        ExternalPoolAdapterSessionFrameKind::Control,
        &request_payload(&nonce, request, expected_response_bytes as u32),
    )?;
    let response_frame = session.receive_with_timeout(timeout)?;
    let response = match parse_response(response_frame, &nonce, expected_response_bytes) {
        Ok(response) => response,
        Err(error) => {
            request_sha256.zeroize();
            return terminal(session, error);
        }
    };
    if let Err(error) = validate_response(&response) {
        request_sha256.zeroize();
        return terminal(session, error);
    }
    let mut response_sha256: [u8; 32] = Sha256::digest(&response[..]).into();
    let root = probe_root(
        &nonce,
        request.len() as u32,
        expected_response_bytes as u32,
        &request_sha256,
        &response_sha256,
    );
    session.send(
        ExternalPoolAdapterSessionFrameKind::Control,
        &receipt_payload(&nonce, &request_sha256, &response_sha256, &root),
    )?;
    request_sha256.zeroize();
    response_sha256.zeroize();
    Ok(())
}

fn parse_request(
    frame: AuthenticatedExternalPoolAdapterSessionFrame,
) -> Result<ExternalPoolAdapterNoWorkProbeHostRequest> {
    let payload = frame.payload();
    if frame.kind() != ExternalPoolAdapterSessionFrameKind::Control
        || payload.len() < REQUEST_HEADER_BYTES
        || !valid_prefix(payload, PROBE_REQUEST)
    {
        bail!("no-work probe request rejected");
    }
    let request_bytes = u32::from_be_bytes(payload[40..44].try_into()?) as usize;
    let response_bytes = u32::from_be_bytes(payload[44..48].try_into()?);
    if request_bytes == 0
        || request_bytes > MAX_REQUEST_BYTES
        || response_bytes == 0
        || response_bytes as usize > MAX_RESPONSE_BYTES
        || payload.len() != REQUEST_HEADER_BYTES + request_bytes
    {
        bail!("no-work probe request rejected");
    }
    let mut nonce = Zeroizing::new([0_u8; 32]);
    nonce.copy_from_slice(&payload[8..40]);
    if nonce.iter().all(|byte| *byte == 0) {
        bail!("no-work probe request rejected");
    }
    let request = Zeroizing::new(payload[REQUEST_HEADER_BYTES..].to_vec());
    let request_sha256 = Sha256::digest(&request[..]).into();
    Ok(ExternalPoolAdapterNoWorkProbeHostRequest {
        nonce,
        request,
        request_sha256,
        expected_response_bytes: response_bytes,
    })
}

fn parse_response(
    frame: AuthenticatedExternalPoolAdapterSessionFrame,
    expected_nonce: &[u8; 32],
    expected_bytes: usize,
) -> Result<Zeroizing<Vec<u8>>> {
    let payload = frame.payload();
    if frame.kind() != ExternalPoolAdapterSessionFrameKind::Control
        || payload.len() != RESPONSE_HEADER_BYTES + expected_bytes
        || !valid_prefix(payload, PROBE_RESPONSE)
        || verify_slices_are_equal(&payload[8..40], expected_nonce).is_err()
        || u32::from_be_bytes(payload[40..44].try_into()?) as usize != expected_bytes
    {
        bail!("no-work probe response rejected");
    }
    Ok(Zeroizing::new(payload[RESPONSE_HEADER_BYTES..].to_vec()))
}

fn valid_receipt(
    frame: &AuthenticatedExternalPoolAdapterSessionFrame,
    nonce: &[u8; 32],
    request_sha256: &[u8; 32],
    response_sha256: &[u8; 32],
    root: &[u8; 32],
) -> bool {
    let payload = frame.payload();
    frame.kind() == ExternalPoolAdapterSessionFrameKind::Control
        && payload.len() == RECEIPT_BYTES
        && valid_prefix(payload, PROBE_RECEIPT)
        && verify_slices_are_equal(&payload[8..40], nonce).is_ok()
        && verify_slices_are_equal(&payload[40..72], request_sha256).is_ok()
        && verify_slices_are_equal(&payload[72..104], response_sha256).is_ok()
        && verify_slices_are_equal(&payload[104..136], root).is_ok()
}

fn request_payload(nonce: &[u8; 32], request: &[u8], response_bytes: u32) -> Zeroizing<Vec<u8>> {
    let mut payload = Zeroizing::new(Vec::with_capacity(REQUEST_HEADER_BYTES + request.len()));
    encode_prefix(&mut payload, PROBE_REQUEST);
    payload.extend_from_slice(nonce);
    payload.extend_from_slice(&(request.len() as u32).to_be_bytes());
    payload.extend_from_slice(&response_bytes.to_be_bytes());
    payload.extend_from_slice(request);
    payload
}

fn response_payload(nonce: &[u8; 32], response: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut payload = Zeroizing::new(Vec::with_capacity(RESPONSE_HEADER_BYTES + response.len()));
    encode_prefix(&mut payload, PROBE_RESPONSE);
    payload.extend_from_slice(nonce);
    payload.extend_from_slice(&(response.len() as u32).to_be_bytes());
    payload.extend_from_slice(response);
    payload
}

fn receipt_payload(
    nonce: &[u8; 32],
    request_sha256: &[u8; 32],
    response_sha256: &[u8; 32],
    root: &[u8; 32],
) -> Zeroizing<Vec<u8>> {
    let mut payload = Zeroizing::new(Vec::with_capacity(RECEIPT_BYTES));
    encode_prefix(&mut payload, PROBE_RECEIPT);
    payload.extend_from_slice(nonce);
    payload.extend_from_slice(request_sha256);
    payload.extend_from_slice(response_sha256);
    payload.extend_from_slice(root);
    payload
}

fn prefix_payload(kind: u8) -> Zeroizing<Vec<u8>> {
    let mut payload = Zeroizing::new(Vec::with_capacity(8));
    encode_prefix(&mut payload, kind);
    payload
}

fn probe_root(
    nonce: &[u8; 32],
    request_bytes: u32,
    response_bytes: u32,
    request_sha256: &[u8; 32],
    response_sha256: &[u8; 32],
) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(PROBE_ROOT_DOMAIN);
    digest.update(nonce);
    digest.update(request_bytes.to_be_bytes());
    digest.update(response_bytes.to_be_bytes());
    digest.update(request_sha256);
    digest.update(response_sha256);
    digest.finalize().into()
}

fn encode_prefix(payload: &mut Vec<u8>, kind: u8) {
    payload.extend_from_slice(PROBE_MAGIC);
    payload.push(PROBE_VERSION);
    payload.push(kind);
    payload.extend_from_slice(&PROBE_FLAGS.to_be_bytes());
}

fn valid_prefix(payload: &[u8], kind: u8) -> bool {
    payload.len() >= 8
        && &payload[..4] == PROBE_MAGIC
        && payload[4] == PROBE_VERSION
        && payload[5] == kind
        && u16::from_be_bytes([payload[6], payload[7]]) == PROBE_FLAGS
}

fn validate_request_size(request: &[u8]) -> Result<()> {
    if request.is_empty() || request.len() > MAX_REQUEST_BYTES {
        bail!("no-work probe request rejected");
    }
    Ok(())
}

fn validate_response_size(response: &[u8], expected: u32) -> Result<()> {
    if response.is_empty()
        || response.len() > MAX_RESPONSE_BYTES
        || response.len() != expected as usize
    {
        bail!("no-work probe response rejected");
    }
    Ok(())
}

fn valid_timeout(timeout: Duration) -> bool {
    !timeout.is_zero() && timeout <= MAX_PROBE_TIMEOUT
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
    bail!("no-work probe protocol rejected")
}

#[cfg(test)]
mod tests;
