use serde::{Deserialize, Serialize};

use crate::{AgentToServer, CliProjectContext, CliWorkspaceStatus};

/// Immutable identity of the node installation that produced a durable CLI
/// completion. A machine-global outbox can survive logout/rebinding, so replay
/// must be scoped to all three values instead of inheriting the current login.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CliCompletionProducerIdentity {
    pub owner_user_id: String,
    pub agent_id: String,
    pub install_id: String,
}

/// Server-authorized Codex credential source for one cloud dispatch. Managed
/// vault tasks bind an exact lease; all other Codex work must use an unmanaged
/// local login so a stale borrowed slot cannot escape billing policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CliCodexCredentialBinding {
    pub managed: bool,
    #[serde(default)]
    pub lease_id: Option<String>,
}

/// A self-contained, durable record of a PC CLI terminal event.
///
/// The node persists this envelope before attempting to send it. If the WebSocket
/// drops before the server acknowledges durable receipt, the same `event_id` is
/// replayed after reconnect. The envelope deliberately does not require a prompt:
/// cloud-dispatched work already has its user message on the server. `prompt` is
/// reserved for work created by the local offline control plane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliCompletionEnvelope {
    /// Stable idempotency key generated once and persisted by the node.
    pub event_id: String,
    /// The original `CliPrompt.req_id` / `CliDone.req_id`.
    pub req_id: String,
    /// Actual CLI selected by the node, for example `codex` or `api-runtime`.
    #[serde(default)]
    pub cli: String,
    /// Completion producer, for example `cloud_dispatch`, `managed_sidecar`, or
    /// `local_offline`.
    #[serde(default)]
    pub origin: String,
    /// Login/node/install identity frozen when the task was admitted. Optional
    /// only for wire decoding; protocol-v5 producers and servers require it.
    #[serde(default)]
    pub producer_identity: Option<CliCompletionProducerIdentity>,
    /// Project/conversation identity known to the node. Old cloud dispatches and
    /// non-project requests may omit it.
    #[serde(default)]
    pub project_context: Option<CliProjectContext>,
    /// Exact channel target for a locally-created offline task. Cloud-dispatched
    /// work normally leaves this empty and relies on the server's request binding.
    #[serde(default)]
    pub channel_id: Option<String>,
    /// Original locally-entered message. It is only persisted for
    /// `origin=local_offline`; cloud-dispatched work must leave it empty.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Bounded final public output needed to reconstruct the result after the
    /// original streaming receiver has disappeared.
    #[serde(default)]
    pub final_output: String,
    pub exit_ok: bool,
    #[serde(default)]
    pub error: Option<String>,
    /// Native CLI thread/session id. For Codex this can be opened as a Codex task.
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub prompt_tokens: Option<u64>,
    #[serde(default)]
    pub cached_input_tokens: Option<u64>,
    #[serde(default)]
    pub completion_tokens: Option<u64>,
    #[serde(default)]
    pub reasoning_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub workspace_status: Option<CliWorkspaceStatus>,
    /// Node wall-clock timestamp when the completion became terminal.
    pub created_at_ms: u64,
}

impl CliCompletionEnvelope {
    /// Rebuild the legacy live-stream terminal message from a durable envelope.
    /// This lets live and replay handlers share the established `CliDone` path.
    pub fn to_cli_done(&self) -> AgentToServer {
        AgentToServer::CliDone {
            req_id: self.req_id.clone(),
            exit_ok: self.exit_ok,
            error: self.error.clone(),
            session_id: self.session_id.clone(),
            prompt_tokens: self.prompt_tokens,
            cached_input_tokens: self.cached_input_tokens,
            completion_tokens: self.completion_tokens,
            reasoning_tokens: self.reasoning_tokens,
            total_tokens: self.total_tokens,
            model: self.model.clone(),
            workspace_status: self.workspace_status.clone(),
        }
    }
}
