#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectExecutionMode {
    Execute,
    Plan,
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
            _ => Self::Execute,
        }
    }

    pub fn is_plan(self) -> bool {
        self == Self::Plan
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Execute => "execute",
            Self::Plan => "plan",
        }
    }
}
