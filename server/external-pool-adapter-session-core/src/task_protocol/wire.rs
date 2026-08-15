use anyhow::{bail, Result};
use ring::constant_time::verify_slices_are_equal;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{AuthenticatedExternalPoolAdapterSessionFrame, ExternalPoolAdapterSessionFrameKind};

use super::{
    request::PreparedExternalPoolAdapterTaskRequest, ExternalPoolAdapterTaskOperationKind,
};

const MAGIC: &[u8; 4] = b"ELTP";
const VERSION: u8 = 1;
const FLAGS: u16 = 0;
const BEGIN: u8 = 1;
const REQUEST: u8 = 2;
const RESPONSE: u8 = 3;
const RECEIPT: u8 = 4;
const PREFIX_BYTES: usize = 8;
const BEGIN_HEADER_BYTES: usize = 284;
const REQUEST_HEADER_BYTES: usize = 128;
const RESPONSE_HEADER_BYTES: usize = 124;
const RECEIPT_OBSERVATION_OFFSET: usize = 188;
const RECEIPT_ROOT_BYTES: usize = 32;
pub(super) const MAX_SEMANTIC_BODY_BYTES: usize = 262_144;
pub(super) const MAX_UPSTREAM_REQUEST_BYTES: usize = 65_536;
pub(super) const MAX_UPSTREAM_RESPONSE_BYTES: usize = 262_144;
pub(super) const MAX_OBSERVATION_BYTES: usize = 262_144;
pub(super) const MAX_EXCHANGE_ORDINAL: u64 = 64;
const REQUEST_DIGEST_DOMAIN: &[u8] = b"elon.external_pool_adapter.task_protocol.request.v1\0";
const EXCHANGE_DIGEST_DOMAIN: &[u8] = b"elon.external_pool_adapter.task_protocol.exchange.v1\0";

pub(super) struct BeginBinding {
    pub(super) ordinal: u64,
    pub(super) operation: ExternalPoolAdapterTaskOperationKind,
    pub(super) nonce: Zeroizing<[u8; 32]>,
    pub(super) request_digest: [u8; 32],
    pub(super) command_digest: [u8; 32],
    pub(super) outbox_operation_digest: [u8; 32],
    pub(super) delivery_attempt_digest: [u8; 32],
    pub(super) route_authorization_digest: [u8; 32],
    pub(super) executor_binding_digest: [u8; 32],
    pub(super) fence_digest: [u8; 32],
    pub(super) body: Zeroizing<Vec<u8>>,
}

pub(super) struct UpstreamRequest {
    pub(super) bytes: Zeroizing<Vec<u8>>,
    pub(super) expected_response_bytes: u32,
}

pub(super) struct ReceiptPayload {
    pub(super) request_sha256: [u8; 32],
    pub(super) response_sha256: [u8; 32],
    pub(super) observation: Zeroizing<Vec<u8>>,
    pub(super) exchange_root: [u8; 32],
}

pub(super) fn encode_begin(
    ordinal: u64,
    nonce: &[u8; 32],
    delivery_attempt_digest: &[u8; 32],
    prepared: &PreparedExternalPoolAdapterTaskRequest,
) -> Zeroizing<Vec<u8>> {
    let mut payload = Zeroizing::new(Vec::with_capacity(BEGIN_HEADER_BYTES + prepared.body.len()));
    encode_operation_header(&mut payload, BEGIN, ordinal, prepared.operation);
    payload.extend_from_slice(nonce);
    payload.extend_from_slice(&prepared.request_digest);
    payload.extend_from_slice(&prepared.command_digest);
    payload.extend_from_slice(&prepared.outbox_operation_digest);
    payload.extend_from_slice(delivery_attempt_digest);
    payload.extend_from_slice(&prepared.route_authorization_digest);
    payload.extend_from_slice(&prepared.executor_binding_digest);
    payload.extend_from_slice(&prepared.fence_digest);
    payload.extend_from_slice(&(prepared.body.len() as u32).to_be_bytes());
    payload.extend_from_slice(&prepared.body);
    payload
}

pub(super) fn parse_begin(
    frame: AuthenticatedExternalPoolAdapterSessionFrame,
    expected_ordinal: u64,
) -> Result<BeginBinding> {
    let payload = control_payload(&frame, BEGIN, BEGIN_HEADER_BYTES)?;
    let (ordinal, operation) = parse_operation_header(payload, BEGIN)?;
    if ordinal != expected_ordinal || ordinal == 0 || ordinal > MAX_EXCHANGE_ORDINAL {
        bail!("ELTP begin ordinal rejected");
    }
    let body_bytes = u32::from_be_bytes(payload[280..284].try_into()?) as usize;
    if body_bytes == 0
        || body_bytes > MAX_SEMANTIC_BODY_BYTES
        || payload.len() != BEGIN_HEADER_BYTES + body_bytes
    {
        bail!("ELTP begin body rejected");
    }
    let nonce = copy_nonzero_digest("ELTP exchange nonce", &payload[24..56])?;
    let request_digest = copy_nonzero_digest("ELTP request", &payload[56..88])?;
    let command_digest = copy_nonzero_digest("ELTP command", &payload[88..120])?;
    let outbox_operation_digest = copy_nonzero_digest("ELTP outbox operation", &payload[120..152])?;
    let delivery_attempt_digest = copy_nonzero_digest("ELTP delivery attempt", &payload[152..184])?;
    let route_authorization_digest =
        copy_nonzero_digest("ELTP route authorization", &payload[184..216])?;
    let executor_binding_digest = copy_nonzero_digest("ELTP executor binding", &payload[216..248])?;
    let fence_digest = copy_nonzero_digest("ELTP fence", &payload[248..280])?;
    let body = Zeroizing::new(payload[BEGIN_HEADER_BYTES..].to_vec());
    let expected_request_digest = task_request_digest(
        operation,
        &command_digest,
        &outbox_operation_digest,
        &route_authorization_digest,
        &executor_binding_digest,
        &fence_digest,
        &body,
    );
    if verify_slices_are_equal(&request_digest, &expected_request_digest).is_err() {
        bail!("ELTP request digest rejected");
    }
    Ok(BeginBinding {
        ordinal,
        operation,
        nonce: Zeroizing::new(nonce),
        request_digest,
        command_digest,
        outbox_operation_digest,
        delivery_attempt_digest,
        route_authorization_digest,
        executor_binding_digest,
        fence_digest,
        body,
    })
}

pub(super) fn encode_request(
    binding: &BeginBinding,
    request: &[u8],
    expected_response_bytes: u32,
) -> Zeroizing<Vec<u8>> {
    let mut payload = Zeroizing::new(Vec::with_capacity(REQUEST_HEADER_BYTES + request.len()));
    encode_operation_header(&mut payload, REQUEST, binding.ordinal, binding.operation);
    encode_exchange_identity(&mut payload, binding);
    payload.extend_from_slice(&(request.len() as u32).to_be_bytes());
    payload.extend_from_slice(&expected_response_bytes.to_be_bytes());
    payload.extend_from_slice(request);
    payload
}

pub(super) fn parse_request(
    frame: AuthenticatedExternalPoolAdapterSessionFrame,
    binding: &BeginBinding,
) -> Result<UpstreamRequest> {
    let payload = control_payload(&frame, REQUEST, REQUEST_HEADER_BYTES)?;
    require_exchange_identity(payload, REQUEST, binding)?;
    let request_bytes = u32::from_be_bytes(payload[120..124].try_into()?) as usize;
    let expected_response_bytes = u32::from_be_bytes(payload[124..128].try_into()?);
    if request_bytes == 0
        || request_bytes > MAX_UPSTREAM_REQUEST_BYTES
        || expected_response_bytes == 0
        || expected_response_bytes as usize > MAX_UPSTREAM_RESPONSE_BYTES
        || payload.len() != REQUEST_HEADER_BYTES + request_bytes
    {
        bail!("ELTP upstream request rejected");
    }
    Ok(UpstreamRequest {
        bytes: Zeroizing::new(payload[REQUEST_HEADER_BYTES..].to_vec()),
        expected_response_bytes,
    })
}

pub(super) fn encode_response(binding: &BeginBinding, response: &[u8]) -> Zeroizing<Vec<u8>> {
    let mut payload = Zeroizing::new(Vec::with_capacity(RESPONSE_HEADER_BYTES + response.len()));
    encode_operation_header(&mut payload, RESPONSE, binding.ordinal, binding.operation);
    encode_exchange_identity(&mut payload, binding);
    payload.extend_from_slice(&(response.len() as u32).to_be_bytes());
    payload.extend_from_slice(response);
    payload
}

pub(super) fn parse_response(
    frame: AuthenticatedExternalPoolAdapterSessionFrame,
    binding: &BeginBinding,
    expected_response_bytes: u32,
) -> Result<Zeroizing<Vec<u8>>> {
    let payload = control_payload(&frame, RESPONSE, RESPONSE_HEADER_BYTES)?;
    require_exchange_identity(payload, RESPONSE, binding)?;
    let response_bytes = u32::from_be_bytes(payload[120..124].try_into()?) as usize;
    if response_bytes == 0
        || response_bytes > MAX_UPSTREAM_RESPONSE_BYTES
        || response_bytes != expected_response_bytes as usize
        || payload.len() != RESPONSE_HEADER_BYTES + response_bytes
    {
        bail!("ELTP upstream response rejected");
    }
    Ok(Zeroizing::new(payload[RESPONSE_HEADER_BYTES..].to_vec()))
}

pub(super) fn encode_receipt(
    binding: &BeginBinding,
    request_sha256: &[u8; 32],
    response_sha256: &[u8; 32],
    observation: &[u8],
    exchange_root: &[u8; 32],
) -> Zeroizing<Vec<u8>> {
    let mut payload = Zeroizing::new(Vec::with_capacity(
        RECEIPT_OBSERVATION_OFFSET + observation.len() + RECEIPT_ROOT_BYTES,
    ));
    encode_operation_header(&mut payload, RECEIPT, binding.ordinal, binding.operation);
    encode_exchange_identity(&mut payload, binding);
    payload.extend_from_slice(request_sha256);
    payload.extend_from_slice(response_sha256);
    payload.extend_from_slice(&(observation.len() as u32).to_be_bytes());
    payload.extend_from_slice(observation);
    payload.extend_from_slice(exchange_root);
    payload
}

pub(super) fn parse_receipt(
    frame: AuthenticatedExternalPoolAdapterSessionFrame,
    binding: &BeginBinding,
) -> Result<ReceiptPayload> {
    let payload = control_payload(
        &frame,
        RECEIPT,
        RECEIPT_OBSERVATION_OFFSET + RECEIPT_ROOT_BYTES,
    )?;
    require_exchange_identity(payload, RECEIPT, binding)?;
    let observation_bytes = u32::from_be_bytes(payload[184..188].try_into()?) as usize;
    if observation_bytes == 0
        || observation_bytes > MAX_OBSERVATION_BYTES
        || payload.len() != RECEIPT_OBSERVATION_OFFSET + observation_bytes + RECEIPT_ROOT_BYTES
    {
        bail!("ELTP receipt observation rejected");
    }
    let observation_end = RECEIPT_OBSERVATION_OFFSET + observation_bytes;
    Ok(ReceiptPayload {
        request_sha256: payload[120..152].try_into()?,
        response_sha256: payload[152..184].try_into()?,
        observation: Zeroizing::new(payload[RECEIPT_OBSERVATION_OFFSET..observation_end].to_vec()),
        exchange_root: payload[observation_end..].try_into()?,
    })
}

pub(super) fn task_request_digest(
    operation: ExternalPoolAdapterTaskOperationKind,
    command_digest: &[u8; 32],
    outbox_operation_digest: &[u8; 32],
    route_authorization_digest: &[u8; 32],
    executor_binding_digest: &[u8; 32],
    fence_digest: &[u8; 32],
    body: &[u8],
) -> [u8; 32] {
    let body_sha256: [u8; 32] = Sha256::digest(body).into();
    let mut digest = Sha256::new();
    digest.update(REQUEST_DIGEST_DOMAIN);
    digest.update([operation as u8]);
    digest.update(command_digest);
    digest.update(outbox_operation_digest);
    digest.update(route_authorization_digest);
    digest.update(executor_binding_digest);
    digest.update(fence_digest);
    digest.update((body.len() as u32).to_be_bytes());
    digest.update(body_sha256);
    digest.finalize().into()
}

#[allow(clippy::too_many_arguments)]
pub(super) fn exchange_root(
    session_transcript_digest: &[u8; 32],
    binding: &BeginBinding,
    request: &[u8],
    response: &[u8],
    observation: &[u8],
) -> [u8; 32] {
    let request_sha256: [u8; 32] = Sha256::digest(request).into();
    let response_sha256: [u8; 32] = Sha256::digest(response).into();
    let observation_sha256: [u8; 32] = Sha256::digest(observation).into();
    let mut digest = Sha256::new();
    digest.update(EXCHANGE_DIGEST_DOMAIN);
    digest.update(session_transcript_digest);
    digest.update(binding.ordinal.to_be_bytes());
    digest.update([binding.operation as u8]);
    digest.update(&binding.nonce[..]);
    digest.update(binding.request_digest);
    digest.update(binding.delivery_attempt_digest);
    digest.update((request.len() as u32).to_be_bytes());
    digest.update(request_sha256);
    digest.update((response.len() as u32).to_be_bytes());
    digest.update(response_sha256);
    digest.update((observation.len() as u32).to_be_bytes());
    digest.update(observation_sha256);
    digest.finalize().into()
}

pub(super) fn validate_semantic_body(body: &[u8]) -> Result<()> {
    if body.is_empty() || body.len() > MAX_SEMANTIC_BODY_BYTES {
        bail!("ELTP semantic request body rejected");
    }
    Ok(())
}

pub(super) fn validate_upstream_request(request: &[u8], response_bytes: usize) -> Result<u32> {
    if request.is_empty()
        || request.len() > MAX_UPSTREAM_REQUEST_BYTES
        || response_bytes == 0
        || response_bytes > MAX_UPSTREAM_RESPONSE_BYTES
    {
        bail!("ELTP upstream request contract rejected");
    }
    Ok(response_bytes as u32)
}

pub(super) fn validate_observation(observation: &[u8]) -> Result<()> {
    if observation.is_empty() || observation.len() > MAX_OBSERVATION_BYTES {
        bail!("ELTP semantic observation rejected");
    }
    Ok(())
}

pub(super) fn decode_digest(label: &str, value: &str) -> Result<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("{label} digest rejected");
    }
    let mut output = [0_u8; 32];
    hex::decode_to_slice(value, &mut output)?;
    if output.iter().all(|byte| *byte == 0) {
        bail!("{label} digest rejected");
    }
    Ok(output)
}

fn encode_operation_header(
    payload: &mut Vec<u8>,
    kind: u8,
    ordinal: u64,
    operation: ExternalPoolAdapterTaskOperationKind,
) {
    payload.extend_from_slice(MAGIC);
    payload.push(VERSION);
    payload.push(kind);
    payload.extend_from_slice(&FLAGS.to_be_bytes());
    payload.extend_from_slice(&ordinal.to_be_bytes());
    payload.push(operation as u8);
    payload.extend_from_slice(&[0_u8; 7]);
}

fn parse_operation_header(
    payload: &[u8],
    expected_kind: u8,
) -> Result<(u64, ExternalPoolAdapterTaskOperationKind)> {
    if payload.len() < 24
        || &payload[..4] != MAGIC
        || payload[4] != VERSION
        || payload[5] != expected_kind
        || u16::from_be_bytes(payload[6..8].try_into()?) != FLAGS
        || payload[17..24].iter().any(|byte| *byte != 0)
    {
        bail!("ELTP frame prefix rejected");
    }
    Ok((
        u64::from_be_bytes(payload[8..16].try_into()?),
        ExternalPoolAdapterTaskOperationKind::from_wire(payload[16])?,
    ))
}

fn encode_exchange_identity(payload: &mut Vec<u8>, binding: &BeginBinding) {
    payload.extend_from_slice(&binding.nonce[..]);
    payload.extend_from_slice(&binding.request_digest);
    payload.extend_from_slice(&binding.delivery_attempt_digest);
}

fn require_exchange_identity(payload: &[u8], kind: u8, binding: &BeginBinding) -> Result<()> {
    let (ordinal, operation) = parse_operation_header(payload, kind)?;
    if ordinal != binding.ordinal
        || operation != binding.operation
        || verify_slices_are_equal(&payload[24..56], &binding.nonce[..]).is_err()
        || verify_slices_are_equal(&payload[56..88], &binding.request_digest).is_err()
        || verify_slices_are_equal(&payload[88..120], &binding.delivery_attempt_digest).is_err()
    {
        bail!("ELTP exchange identity rejected");
    }
    Ok(())
}

fn control_payload<'a>(
    frame: &'a AuthenticatedExternalPoolAdapterSessionFrame,
    expected_kind: u8,
    minimum_bytes: usize,
) -> Result<&'a [u8]> {
    if frame.kind() != ExternalPoolAdapterSessionFrameKind::Control
        || frame.payload().len() < minimum_bytes
        || frame.payload().len() < PREFIX_BYTES
        || frame.payload()[5] != expected_kind
    {
        bail!("ELTP control frame rejected");
    }
    Ok(frame.payload())
}

fn copy_nonzero_digest(label: &str, value: &[u8]) -> Result<[u8; 32]> {
    let output: [u8; 32] = value.try_into()?;
    if output.iter().all(|byte| *byte == 0) {
        bail!("{label} digest rejected");
    }
    Ok(output)
}
