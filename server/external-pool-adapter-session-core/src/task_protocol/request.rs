use anyhow::Result;
use zeroize::{Zeroize, Zeroizing};

use super::{wire, ExternalPoolAdapterTaskOperationKind};

/// One host-prepared semantic task request. Delivery-attempt identity is deliberately absent.
pub struct PreparedExternalPoolAdapterTaskRequest {
    pub(super) operation: ExternalPoolAdapterTaskOperationKind,
    pub(super) command_digest: [u8; 32],
    pub(super) outbox_operation_digest: [u8; 32],
    pub(super) route_authorization_digest: [u8; 32],
    pub(super) executor_binding_digest: [u8; 32],
    pub(super) fence_digest: [u8; 32],
    pub(super) body: Zeroizing<Vec<u8>>,
    pub(super) request_digest: [u8; 32],
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_external_pool_adapter_task_request(
    operation: ExternalPoolAdapterTaskOperationKind,
    command_digest: &str,
    outbox_operation_digest: &str,
    route_authorization_digest: &str,
    executor_binding_digest: &str,
    fence_digest: &str,
    body: &[u8],
) -> Result<PreparedExternalPoolAdapterTaskRequest> {
    wire::validate_semantic_body(body)?;
    let command_digest = wire::decode_digest("ELTP command", command_digest)?;
    let outbox_operation_digest =
        wire::decode_digest("ELTP outbox operation", outbox_operation_digest)?;
    let route_authorization_digest =
        wire::decode_digest("ELTP route authorization", route_authorization_digest)?;
    let executor_binding_digest =
        wire::decode_digest("ELTP executor binding", executor_binding_digest)?;
    let fence_digest = wire::decode_digest("ELTP fence", fence_digest)?;
    let request_digest = wire::task_request_digest(
        operation,
        &command_digest,
        &outbox_operation_digest,
        &route_authorization_digest,
        &executor_binding_digest,
        &fence_digest,
        body,
    );
    Ok(PreparedExternalPoolAdapterTaskRequest {
        operation,
        command_digest,
        outbox_operation_digest,
        route_authorization_digest,
        executor_binding_digest,
        fence_digest,
        body: Zeroizing::new(body.to_vec()),
        request_digest,
    })
}

impl PreparedExternalPoolAdapterTaskRequest {
    pub fn operation(&self) -> ExternalPoolAdapterTaskOperationKind {
        self.operation
    }

    pub fn request_digest_hex(&self) -> String {
        hex::encode(self.request_digest)
    }
}

impl Drop for PreparedExternalPoolAdapterTaskRequest {
    fn drop(&mut self) {
        self.command_digest.zeroize();
        self.outbox_operation_digest.zeroize();
        self.route_authorization_digest.zeroize();
        self.executor_binding_digest.zeroize();
        self.fence_digest.zeroize();
        self.request_digest.zeroize();
    }
}
