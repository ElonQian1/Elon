use serde::{Deserialize, Serialize};

pub const PROTO_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerToAgent {
    Exec {
        task_id: String,
        cli: String,
        args: Vec<String>,
        cwd: String,
        #[serde(default)]
        env: Vec<(String, String)>,
    },
    Cancel {
        task_id: String,
    },
    Ping {
        nonce: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentToServer {
    Register {
        agent_id: String,
        version: String,
        proto_version: u32,
        #[serde(default)]
        allowed_clis: Vec<String>,
        #[serde(default)]
        allowed_cwds: Vec<String>,
    },
    TaskStarted {
        task_id: String,
        pid: u32,
    },
    TaskStdout {
        task_id: String,
        data: String,
    },
    TaskStderr {
        task_id: String,
        data: String,
    },
    TaskExit {
        task_id: String,
        code: Option<i32>,
    },
    TaskError {
        task_id: String,
        message: String,
    },
    Pong {
        nonce: Option<String>,
    },
}

impl AgentToServer {
    pub fn task_id(&self) -> Option<&str> {
        match self {
            Self::TaskStarted { task_id, .. }
            | Self::TaskStdout { task_id, .. }
            | Self::TaskStderr { task_id, .. }
            | Self::TaskExit { task_id, .. }
            | Self::TaskError { task_id, .. } => Some(task_id.as_str()),
            Self::Register { .. } | Self::Pong { .. } => None,
        }
    }
}
