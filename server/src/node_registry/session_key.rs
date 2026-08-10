/// Process-local identity for one accepted legacy WebSocket connection.
///
/// This key only prevents an older reader task from mutating the in-memory
/// state owned by a newer connection. It is not durable, authenticated endpoint
/// authority and must never be projected into Ready, route, Lease, or dispatch
/// custody.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AgentProcessSessionKey {
    agent_id: String,
    session_id: String,
}

impl AgentProcessSessionKey {
    pub(crate) fn new(agent_id: impl Into<String>, session_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: session_id.into(),
        }
    }

    pub(crate) fn agent_id(&self) -> &str {
        &self.agent_id
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }
}

impl std::fmt::Debug for AgentProcessSessionKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AgentProcessSessionKey")
            .field("agent_id", &self.agent_id)
            .field("session_id", &"<process-local>")
            .finish()
    }
}
