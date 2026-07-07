//! Tool planning for fb2 external context queries.

use serde_json::{json, Value};

use crate::{
    external_app_context_config::{
        infer_lottery_type, platform_order_summary_enabled, platform_order_summary_requested,
    },
    external_app_context_scenario_prompt::fb2_domain_scenario_selection,
};

const PLANNER_SCHEMA: &str = "external_app.tool_plan.v1";
const PLANNER_STRATEGY: &str = "deterministic_fb2_chat_v1";

#[derive(Clone)]
pub(crate) struct PlannedTool {
    pub(crate) name: &'static str,
    pub(crate) reason: &'static str,
    pub(crate) arguments: Value,
    pub(crate) requires_external_user: bool,
    trigger: &'static str,
    confidence: u8,
    evidence: Vec<String>,
}

pub(crate) struct Fb2ToolPlan {
    topic_hint: String,
    pub(crate) tools: Vec<PlannedTool>,
    skipped_reasons: Vec<&'static str>,
    domain_scenario_selection: Value,
}

impl Fb2ToolPlan {
    pub(crate) fn tool_names(&self) -> Vec<&'static str> {
        self.tools.iter().map(|tool| tool.name).collect()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub(crate) fn into_tools(self) -> Vec<PlannedTool> {
        self.tools
    }

    pub(crate) fn to_metadata(&self) -> Value {
        json!({
            "schema": PLANNER_SCHEMA,
            "strategy": PLANNER_STRATEGY,
            "topic_hint": self.topic_hint,
            "planned_count": self.tools.len(),
            "planned_tools": self.tools.iter().map(PlannedTool::to_metadata).collect::<Vec<_>>(),
            "domain_scenario_selection": self.domain_scenario_selection,
            "skipped_reasons": self.skipped_reasons
        })
    }
}

impl PlannedTool {
    fn to_metadata(&self) -> Value {
        json!({
            "name": self.name,
            "reason": self.reason,
            "trigger": self.trigger,
            "confidence": self.confidence,
            "requires_external_user": self.requires_external_user,
            "evidence": self.evidence
        })
    }
}

pub(crate) fn plan_fb2_tools(context: &Value, topic_hint: Option<&str>) -> Fb2ToolPlan {
    plan_fb2_tools_with_platform_scope(context, topic_hint, platform_order_summary_enabled())
}


#[path = "external_app_context_tool_planner_impl.rs"]
mod impl_funcs;
use self::impl_funcs::*;
