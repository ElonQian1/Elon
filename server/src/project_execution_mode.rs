#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectExecutionMode {
    Execute,
    Plan,
    /// 悬浮球手机控制专用：绕过本地 intent_router 分流，
    /// 直接进入 Codex CLI 意图闸判断。
    /// Codex 会先读 AGENTS.md，自己判断是闲聊还是生成手机控制脚本。
    ForceCli,
}

impl ProjectExecutionMode {
    pub fn from_request(execution_mode: Option<&str>, plan_mode: Option<bool>) -> Self {
        if plan_mode.unwrap_or(false) {
            return Self::Plan;
        }
        match execution_mode
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "plan" | "planning" | "planner" | "readonly_plan" => Self::Plan,
            "force_cli" | "forcecli" | "always_cli" => Self::ForceCli,
            _ => Self::Execute,
        }
    }

    pub fn is_plan(self) -> bool {
        self == Self::Plan
    }

    pub fn is_force_cli(self) -> bool {
        self == Self::ForceCli
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Execute => "execute",
            Self::Plan => "plan",
            Self::ForceCli => "force_cli",
        }
    }
}
