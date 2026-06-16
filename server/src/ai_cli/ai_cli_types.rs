use crate::intent_router;

/// 当前会话对应的原生 CLI 会话作用域（project + user + conversation 三元组）。
#[derive(Debug, Clone)]
pub struct NativeSessionScope {
    pub project_id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub runtime_permission: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiCliRequestMode {
    Execute,
    Plan,
}

impl AiCliRequestMode {
    pub fn is_plan(self) -> bool {
        self == Self::Plan
    }
}

/// 意图网关分类结果。
#[derive(Debug, Clone, PartialEq)]
pub struct IntentGateResult {
    pub route: intent_router::CapabilityRoute,
    pub confidence: f64,
    pub reason: String,
    pub chat_reply: Option<String>,
}

impl IntentGateResult {
    pub fn should_enter_development(&self) -> bool {
        self.route == intent_router::CapabilityRoute::CodeAgent && self.confidence >= 0.75
    }
}
