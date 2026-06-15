use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HybridRankProfile {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) source_weights: BTreeMap<String, f64>,
    pub(crate) test_context_bonus: f64,
    pub(crate) reasons: Vec<String>,
}

pub(crate) fn infer_rank_profile(task: &str) -> HybridRankProfile {
    let lower = task.to_ascii_lowercase();
    let contains_any = |terms: &[&str]| terms.iter().any(|term| lower.contains(term));
    if contains_any(&[
        "重构",
        "refactor",
        "rename",
        "改名",
        "迁移",
        "拆分",
        "抽取",
        "references",
        "callers",
    ]) {
        return HybridRankProfile::refactor();
    }
    if contains_any(&[
        "报错",
        "错误",
        "异常",
        "失败",
        "日志",
        "panic",
        "error",
        "exception",
        "crash",
        "stack trace",
        "duplicate key",
    ]) {
        return HybridRankProfile::error();
    }
    if contains_any(&["测试", "用例", "覆盖", "断言", "test", "tests"]) {
        return HybridRankProfile::test();
    }
    if contains_any(&[
        "解释",
        "说明",
        "是什么",
        "为什么",
        "流程",
        "架构",
        "overview",
        "explain",
        "describe",
    ]) {
        return HybridRankProfile::explanation();
    }
    if contains_any(&[
        "修改",
        "修复",
        "实现",
        "完善",
        "继续",
        "新增",
        "接入",
        "改成",
        "fix",
        "implement",
        "change",
        "add",
    ]) {
        return HybridRankProfile::implementation();
    }
    HybridRankProfile::general()
}

impl HybridRankProfile {
    fn general() -> Self {
        Self::new(
            "general",
            "balanced retrieval for broad codebase questions",
            [
                ("symbol", 1000.0),
                ("full_text", 800.0),
                ("vector", 760.0),
                ("graph_symbol", 650.0),
                ("graph_file", 620.0),
                ("graph_test", 700.0),
            ],
            18.0,
            ["default_balanced_profile"],
        )
    }

    fn implementation() -> Self {
        Self::new(
            "implementation",
            "favor exact edit targets, nearby snippets, and validation tests",
            [
                ("symbol", 1020.0),
                ("full_text", 840.0),
                ("vector", 780.0),
                ("graph_symbol", 760.0),
                ("graph_file", 720.0),
                ("graph_test", 790.0),
            ],
            34.0,
            ["matched_task_intent=implementation"],
        )
    }

    fn error() -> Self {
        Self::new(
            "error",
            "favor exact error strings, logs, SQL, config, and failing tests",
            [
                ("symbol", 780.0),
                ("full_text", 1040.0),
                ("vector", 860.0),
                ("graph_symbol", 760.0),
                ("graph_file", 730.0),
                ("graph_test", 840.0),
            ],
            42.0,
            ["matched_task_intent=error"],
        )
    }

    fn refactor() -> Self {
        Self::new(
            "refactor",
            "favor references, callers, public boundaries, and tests",
            [
                ("symbol", 920.0),
                ("full_text", 820.0),
                ("vector", 760.0),
                ("graph_symbol", 1040.0),
                ("graph_file", 860.0),
                ("graph_test", 940.0),
            ],
            52.0,
            ["matched_task_intent=refactor"],
        )
    }

    fn explanation() -> Self {
        Self::new(
            "explanation",
            "favor module overview, relation graph, and semantic context",
            [
                ("symbol", 860.0),
                ("full_text", 900.0),
                ("vector", 880.0),
                ("graph_symbol", 940.0),
                ("graph_file", 980.0),
                ("graph_test", 760.0),
            ],
            8.0,
            ["matched_task_intent=explanation"],
        )
    }

    fn test() -> Self {
        Self::new(
            "test",
            "favor tests, test-covered symbols, and assertions",
            [
                ("symbol", 880.0),
                ("full_text", 880.0),
                ("vector", 760.0),
                ("graph_symbol", 900.0),
                ("graph_file", 820.0),
                ("graph_test", 1080.0),
            ],
            70.0,
            ["matched_task_intent=test"],
        )
    }

    fn new<const N: usize, const M: usize>(
        name: &str,
        description: &str,
        weights: [(&str, f64); N],
        test_context_bonus: f64,
        reasons: [&str; M],
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            source_weights: weights
                .into_iter()
                .map(|(source, weight)| (source.to_string(), weight))
                .collect(),
            test_context_bonus,
            reasons: reasons.into_iter().map(ToOwned::to_owned).collect(),
        }
    }

    pub(crate) fn source_weight(&self, source: &str) -> f64 {
        self.source_weights.get(source).copied().unwrap_or(700.0)
    }

    pub(crate) fn reason(&self, source: &str) -> String {
        format!(
            "rank_profile={} weight={:.0}",
            self.name,
            self.source_weight(source)
        )
    }

    pub(crate) fn test_bonus(&self, is_test_context: bool) -> f64 {
        if is_test_context {
            self.test_context_bonus
        } else {
            0.0
        }
    }

    pub(crate) fn chunk_type_bonus(&self, chunk_type: &str) -> f64 {
        match (self.name.as_str(), chunk_type) {
            ("explanation", "module") => 42.0,
            ("error", "test" | "config" | "error") => 34.0,
            ("implementation", "symbol") => 24.0,
            ("refactor", "symbol" | "test") => 28.0,
            ("test", "test") => 58.0,
            _ => 0.0,
        }
    }
}
