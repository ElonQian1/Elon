use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use ring::constant_time::verify_slices_are_equal;
use sha2::{Digest, Sha256};

use crate::{
    crypto::random_array32, AuthenticatedExternalPoolAdapterSession,
    ExternalPoolAdapterSessionFrameKind,
};

use super::{
    receipt::ExternalPoolAdapterTaskProtocolHostReceipt,
    request::PreparedExternalPoolAdapterTaskRequest,
    wire::{self, BeginBinding, UpstreamRequest},
};

const MAX_EXCHANGE_TIMEOUT: Duration = Duration::from_millis(15_000);

/// Ordered host side of one authenticated ELTP v1 session.
pub struct ExternalPoolAdapterTaskProtocolHost<'session> {
    session: &'session mut AuthenticatedExternalPoolAdapterSession,
    next_ordinal: u64,
}

/// One outstanding exchange. Dropping it before exact completion terminates the whole session.
pub struct ExternalPoolAdapterTaskProtocolHostExchange<'exchange> {
    session: &'exchange mut AuthenticatedExternalPoolAdapterSession,
    binding: BeginBinding,
    request: UpstreamRequest,
    session_transcript_digest: [u8; 32],
    deadline: Instant,
    active: bool,
}

impl<'session> ExternalPoolAdapterTaskProtocolHost<'session> {
    pub fn new(session: &'session mut AuthenticatedExternalPoolAdapterSession) -> Self {
        Self {
            session,
            next_ordinal: 1,
        }
    }

    pub fn session_transcript_digest_hex(&self) -> String {
        hex::encode(self.session.binding_transcript_digest())
    }

    pub fn begin<'exchange>(
        &'exchange mut self,
        prepared: PreparedExternalPoolAdapterTaskRequest,
        delivery_attempt_digest: &str,
        timeout: Duration,
    ) -> Result<ExternalPoolAdapterTaskProtocolHostExchange<'exchange>> {
        if !valid_timeout(timeout) || self.next_ordinal > wire::MAX_EXCHANGE_ORDINAL {
            return terminal_error(self.session);
        }
        let deadline = match Instant::now().checked_add(timeout) {
            Some(deadline) => deadline,
            None => return terminal_error(self.session),
        };
        let delivery_attempt_digest =
            match wire::decode_digest("ELTP delivery attempt", delivery_attempt_digest) {
                Ok(value) => value,
                Err(error) => return terminal(self.session, error),
            };
        let nonce = match random_array32() {
            Ok(value) if value.iter().any(|byte| *byte != 0) => value,
            Ok(_) => return terminal_error(self.session),
            Err(error) => return terminal(self.session, error),
        };
        let ordinal = self.next_ordinal;
        let begin = wire::encode_begin(ordinal, &nonce, &delivery_attempt_digest, &prepared);
        let send_timeout = match remaining_before(deadline) {
            Ok(timeout) => timeout,
            Err(error) => return terminal(self.session, error),
        };
        self.session.send_with_timeout(
            ExternalPoolAdapterSessionFrameKind::Control,
            &begin,
            send_timeout,
        )?;
        let binding = BeginBinding {
            ordinal,
            operation: prepared.operation,
            nonce: zeroize::Zeroizing::new(nonce),
            request_digest: prepared.request_digest,
            command_digest: prepared.command_digest,
            outbox_operation_digest: prepared.outbox_operation_digest,
            delivery_attempt_digest,
            route_authorization_digest: prepared.route_authorization_digest,
            executor_binding_digest: prepared.executor_binding_digest,
            fence_digest: prepared.fence_digest,
            body: zeroize::Zeroizing::new(prepared.body.to_vec()),
        };
        let receive_timeout = match remaining_before(deadline) {
            Ok(timeout) => timeout,
            Err(error) => return terminal(self.session, error),
        };
        let frame = self.session.receive_with_timeout(receive_timeout)?;
        let request = match wire::parse_request(frame, &binding) {
            Ok(request) => request,
            Err(error) => return terminal(self.session, error),
        };
        self.next_ordinal = self
            .next_ordinal
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("ELTP ordinal overflow"))?;
        let session_transcript_digest = self.session.binding_transcript_digest();
        Ok(ExternalPoolAdapterTaskProtocolHostExchange {
            session: &mut *self.session,
            binding,
            request,
            session_transcript_digest,
            deadline,
            active: true,
        })
    }
}

impl ExternalPoolAdapterTaskProtocolHostExchange<'_> {
    pub fn request(&self) -> &[u8] {
        &self.request.bytes
    }

    pub fn expected_response_bytes(&self) -> usize {
        self.request.expected_response_bytes as usize
    }

    /// Returns only the remaining bounded exchange time; no session or raw transport escapes.
    pub fn remaining_timeout(&self) -> Result<Duration> {
        remaining_before(self.deadline)
    }

    /// Completes exactly once after validating the child-produced semantic observation.
    /// `validate_observation` must be pure and bounded. It executes synchronously and is not
    /// preempted; deadline checks around it make an overrun terminal without returning a receipt.
    pub fn complete<T>(
        mut self,
        response: &[u8],
        validate_observation: impl FnOnce(&[u8]) -> Result<T>,
    ) -> Result<(ExternalPoolAdapterTaskProtocolHostReceipt, T)> {
        if response.is_empty()
            || response.len() > wire::MAX_UPSTREAM_RESPONSE_BYTES
            || response.len() != self.request.expected_response_bytes as usize
        {
            return terminal_error(self.session);
        }
        let encoded_response = wire::encode_response(&self.binding, response);
        let send_timeout = match remaining_before(self.deadline) {
            Ok(timeout) => timeout,
            Err(error) => return terminal(self.session, error),
        };
        self.session.send_with_timeout(
            ExternalPoolAdapterSessionFrameKind::Control,
            &encoded_response,
            send_timeout,
        )?;
        let receive_timeout = match remaining_before(self.deadline) {
            Ok(timeout) => timeout,
            Err(error) => return terminal(self.session, error),
        };
        let frame = self.session.receive_with_timeout(receive_timeout)?;
        let received = match wire::parse_receipt(frame, &self.binding) {
            Ok(receipt) => receipt,
            Err(error) => return terminal(self.session, error),
        };
        let request_sha256: [u8; 32] = Sha256::digest(&self.request.bytes[..]).into();
        let response_sha256: [u8; 32] = Sha256::digest(response).into();
        let observation_sha256: [u8; 32] = Sha256::digest(&received.observation[..]).into();
        let exchange_root = wire::exchange_root(
            &self.session_transcript_digest,
            &self.binding,
            &self.request.bytes,
            response,
            &received.observation,
        );
        if verify_slices_are_equal(&received.request_sha256, &request_sha256).is_err()
            || verify_slices_are_equal(&received.response_sha256, &response_sha256).is_err()
            || verify_slices_are_equal(&received.exchange_root, &exchange_root).is_err()
        {
            return terminal_error(self.session);
        }
        require_before_deadline(self.session, self.deadline)?;
        let observation = match validate_observation(&received.observation) {
            Ok(observation) => observation,
            Err(error) => return terminal(self.session, error),
        };
        require_before_deadline(self.session, self.deadline)?;
        let receipt = ExternalPoolAdapterTaskProtocolHostReceipt {
            ordinal: self.binding.ordinal,
            operation: self.binding.operation,
            exchange_nonce_digest: Sha256::digest(&self.binding.nonce[..]).into(),
            request_digest: self.binding.request_digest,
            command_digest: self.binding.command_digest,
            outbox_operation_digest: self.binding.outbox_operation_digest,
            delivery_attempt_digest: self.binding.delivery_attempt_digest,
            route_authorization_digest: self.binding.route_authorization_digest,
            executor_binding_digest: self.binding.executor_binding_digest,
            fence_digest: self.binding.fence_digest,
            upstream_request_bytes: self.request.bytes.len() as u32,
            upstream_request_sha256: request_sha256,
            upstream_response_bytes: response.len() as u32,
            upstream_response_sha256: response_sha256,
            semantic_observation_bytes: received.observation.len() as u32,
            semantic_observation_sha256: observation_sha256,
            session_transcript_digest: self.session_transcript_digest,
            exchange_root,
        };
        require_before_deadline(self.session, self.deadline)?;
        self.active = false;
        Ok((receipt, observation))
    }
}

impl Drop for ExternalPoolAdapterTaskProtocolHostExchange<'_> {
    fn drop(&mut self) {
        if self.active {
            self.session.terminate();
        }
    }
}

fn valid_timeout(timeout: Duration) -> bool {
    !timeout.is_zero() && timeout <= MAX_EXCHANGE_TIMEOUT
}

fn remaining_before(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| anyhow::anyhow!("ELTP host exchange timed out"))
}

fn require_before_deadline(
    session: &mut AuthenticatedExternalPoolAdapterSession,
    deadline: Instant,
) -> Result<()> {
    match remaining_before(deadline) {
        Ok(_) => Ok(()),
        Err(error) => terminal(session, error),
    }
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
    bail!("ELTP host exchange rejected")
}
