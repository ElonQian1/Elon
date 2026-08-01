use std::{error::Error, fmt};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct OpenCommerceGrantBudgetDecision {
    pub grant_id: String,
    pub invocation_id: String,
    pub max_invocations: Option<i64>,
    pub max_amount_micros: Option<i64>,
    pub budget_currency: String,
    pub used_invocations: i64,
    pub used_amount_micros: i64,
    pub remaining_invocations: Option<i64>,
    pub remaining_amount_micros: Option<i64>,
}

#[derive(Debug)]
pub(crate) struct OpenCommerceGrantBudgetExceeded {
    pub grant_id: String,
    pub limit_kind: &'static str,
    pub limit: i64,
    pub used: i64,
}

impl fmt::Display for OpenCommerceGrantBudgetExceeded {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = if self.limit_kind == "invocations" {
            "调用次数"
        } else {
            "计量金额"
        };
        write!(
            formatter,
            "授权预算已用尽：{}上限 {}，当前已用 {}（grant_id={}）",
            label, self.limit, self.used, self.grant_id
        )
    }
}

impl Error for OpenCommerceGrantBudgetExceeded {}
