use std::time::Duration;

use anyhow::{bail, Result};
use sha2::{Digest, Sha256};

use crate::{AuthenticatedExternalPoolAdapterSession, ExternalPoolAdapterSessionFrameKind};

use super::{wire, ExternalPoolAdapterTaskOperationKind};

const MAX_EXCHANGE_TIMEOUT: Duration = Duration::from_millis(15_000);

/// Ordered child side of one authenticated ELTP v1 session.
pub struct ExternalPoolAdapterTaskProtocolChild<'session> {
    session: &'session mut AuthenticatedExternalPoolAdapterSession,
    next_ordinal: u64,
}

/// One outstanding child exchange. Drop-before-receipt is terminal.
pub struct ExternalPoolAdapterTaskProtocolChildExchange<'exchange> {
    session: &'exchange mut AuthenticatedExternalPoolAdapterSession,
    binding: wire::BeginBinding,
    session_transcript_digest: [u8; 32],
    timeout: Duration,
    active: bool,
}

impl<'session> ExternalPoolAdapterTaskProtocolChild<'session> {
    pub fn new(session: &'session mut AuthenticatedExternalPoolAdapterSession) -> Self {
        Self {
            session,
            next_ordinal: 1,
        }
    }

    pub fn next<'exchange>(
        &'exchange mut self,
        timeout: Duration,
    ) -> Result<ExternalPoolAdapterTaskProtocolChildExchange<'exchange>> {
        if timeout.is_zero()
            || timeout > MAX_EXCHANGE_TIMEOUT
            || self.next_ordinal > wire::MAX_EXCHANGE_ORDINAL
        {
            return terminal_error(self.session);
        }
        let frame = self.session.receive_with_timeout(timeout)?;
        let binding = match wire::parse_begin(frame, self.next_ordinal) {
            Ok(binding) => binding,
            Err(error) => return terminal(self.session, error),
        };
        self.next_ordinal = self
            .next_ordinal
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("ELTP ordinal overflow"))?;
        let session_transcript_digest = self.session.binding_transcript_digest();
        Ok(ExternalPoolAdapterTaskProtocolChildExchange {
            session: &mut *self.session,
            binding,
            session_transcript_digest,
            timeout,
            active: true,
        })
    }
}

impl ExternalPoolAdapterTaskProtocolChildExchange<'_> {
    pub fn ordinal(&self) -> u64 {
        self.binding.ordinal
    }

    pub fn operation(&self) -> ExternalPoolAdapterTaskOperationKind {
        self.binding.operation
    }

    pub fn request_body(&self) -> &[u8] {
        &self.binding.body
    }

    pub fn request_digest_hex(&self) -> String {
        hex::encode(self.binding.request_digest)
    }

    pub fn command_digest_hex(&self) -> String {
        hex::encode(self.binding.command_digest)
    }

    pub fn outbox_operation_digest_hex(&self) -> String {
        hex::encode(self.binding.outbox_operation_digest)
    }

    pub fn delivery_attempt_digest_hex(&self) -> String {
        hex::encode(self.binding.delivery_attempt_digest)
    }

    pub fn route_authorization_digest_hex(&self) -> String {
        hex::encode(self.binding.route_authorization_digest)
    }

    pub fn executor_binding_digest_hex(&self) -> String {
        hex::encode(self.binding.executor_binding_digest)
    }

    pub fn fence_digest_hex(&self) -> String {
        hex::encode(self.binding.fence_digest)
    }

    /// Sends one exact upstream request, validates one exact-length response, and authenticates
    /// the caller-produced bounded semantic observation.
    pub fn complete(
        mut self,
        upstream_request: &[u8],
        expected_exact_response_bytes: usize,
        parse_response_to_observation: impl FnOnce(&[u8]) -> Result<Vec<u8>>,
    ) -> Result<()> {
        let expected_response_bytes = match wire::validate_upstream_request(
            upstream_request,
            expected_exact_response_bytes,
        ) {
            Ok(value) => value,
            Err(error) => return terminal(self.session, error),
        };
        self.session.send(
            ExternalPoolAdapterSessionFrameKind::Control,
            &wire::encode_request(&self.binding, upstream_request, expected_response_bytes),
        )?;
        let frame = self.session.receive_with_timeout(self.timeout)?;
        let response = match wire::parse_response(frame, &self.binding, expected_response_bytes) {
            Ok(response) => response,
            Err(error) => return terminal(self.session, error),
        };
        let observation = match parse_response_to_observation(&response) {
            Ok(observation) => observation,
            Err(error) => return terminal(self.session, error),
        };
        if let Err(error) = wire::validate_observation(&observation) {
            return terminal(self.session, error);
        }
        let request_sha256: [u8; 32] = Sha256::digest(upstream_request).into();
        let response_sha256: [u8; 32] = Sha256::digest(&response[..]).into();
        let exchange_root = wire::exchange_root(
            &self.session_transcript_digest,
            &self.binding,
            upstream_request,
            &response,
            &observation,
        );
        self.session.send(
            ExternalPoolAdapterSessionFrameKind::Control,
            &wire::encode_receipt(
                &self.binding,
                &request_sha256,
                &response_sha256,
                &observation,
                &exchange_root,
            ),
        )?;
        self.active = false;
        Ok(())
    }
}

impl Drop for ExternalPoolAdapterTaskProtocolChildExchange<'_> {
    fn drop(&mut self) {
        if self.active {
            self.session.terminate();
        }
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
    bail!("ELTP child exchange rejected")
}
