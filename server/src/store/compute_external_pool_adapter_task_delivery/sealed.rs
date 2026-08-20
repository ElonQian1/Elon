//! Non-serializable semantic facts created only beside their durable V213 source.

use anyhow::{ensure, Result};

use crate::compute_federation::{
    external_pool_adapter_task_protocol_production::ExternalPoolAdapterTaskExchangeAttemptEnvelope,
    start_outbox::ComputeStartOutboxSendAttemptEnvelope,
};

pub(in crate::store) struct ExternalPoolAdapterTaskExchangeAttemptFactory<'send> {
    send_attempt: &'send ComputeStartOutboxSendAttemptEnvelope,
}

pub(in crate::store) struct SealedExternalPoolAdapterTaskExchangeAttempt {
    envelope: ExternalPoolAdapterTaskExchangeAttemptEnvelope,
}

impl<'send> ExternalPoolAdapterTaskExchangeAttemptFactory<'send> {
    pub(super) fn new(send_attempt: &'send ComputeStartOutboxSendAttemptEnvelope) -> Self {
        Self { send_attempt }
    }

    pub(in crate::store) fn send_attempt(&self) -> &ComputeStartOutboxSendAttemptEnvelope {
        self.send_attempt
    }

    pub(in crate::store) fn seal(
        self,
        envelope: ExternalPoolAdapterTaskExchangeAttemptEnvelope,
    ) -> Result<SealedExternalPoolAdapterTaskExchangeAttempt> {
        let identity = &envelope.attempt.identity;
        let expected_operation = match self.send_attempt.operation_kind.as_str() {
            "prepare" => "prepare",
            "commit" => "idempotent_commit",
            "cancel" => "cancel_no_start",
            _ => anyhow::bail!("V278 outbound send operation is unsupported"),
        };
        ensure!(
            identity.operation_kind == expected_operation
                && identity.source.source_kind == "start_outbox_send_attempt"
                && identity.source.source_id == self.send_attempt.send_attempt_id
                && identity.source.source_digest == self.send_attempt.send_attempt_digest
                && identity.command.command_id == self.send_attempt.command_id
                && identity.command.command_digest == self.send_attempt.command_digest
                && identity.command.outbox_id == self.send_attempt.outbox_id
                && identity.command.outbox_digest == self.send_attempt.outbox_digest
                && identity.command.send_attempt_id == self.send_attempt.send_attempt_id
                && identity.command.send_attempt_digest == self.send_attempt.send_attempt_digest
                && identity.route.route_authorization_id
                    == self.send_attempt.route_authorization_id
                && identity.route.route_authorization_digest
                    == self.send_attempt.route_authorization_digest
                && identity.request_digest == self.send_attempt.request_digest
                && envelope.attempt.started_at == self.send_attempt.started_at,
            "V278 exchange attempt does not bind the exact prepared V213 send"
        );
        Ok(SealedExternalPoolAdapterTaskExchangeAttempt { envelope })
    }
}

impl SealedExternalPoolAdapterTaskExchangeAttempt {
    pub(super) fn envelope(&self) -> &ExternalPoolAdapterTaskExchangeAttemptEnvelope {
        &self.envelope
    }

    pub(super) fn into_envelope(self) -> ExternalPoolAdapterTaskExchangeAttemptEnvelope {
        self.envelope
    }
}
