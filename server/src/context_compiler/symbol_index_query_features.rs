use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryFeatures {
    pub(crate) raw_query: String,
    pub(crate) symbol_like_terms: Vec<String>,
    pub(crate) file_like_terms: Vec<String>,
    pub(crate) error_like_terms: Vec<String>,
    pub(crate) route_like_terms: Vec<String>,
    pub(crate) status_codes: Vec<u16>,
    pub(crate) quoted_strings: Vec<String>,
    pub(crate) mentions_test: bool,
    pub(crate) mentions_refactor: bool,
    pub(crate) mentions_error: bool,
    pub(crate) mentions_modify: bool,
    pub(crate) mentions_explain: bool,
    pub(crate) mentions_locate: bool,
    pub(crate) mentions_add_feature: bool,
}

impl QueryFeatures {
    pub(crate) fn empty() -> Self {
        Self {
            raw_query: String::new(),
            symbol_like_terms: Vec::new(),
            file_like_terms: Vec::new(),
            error_like_terms: Vec::new(),
            route_like_terms: Vec::new(),
            status_codes: Vec::new(),
            quoted_strings: Vec::new(),
            mentions_test: false,
            mentions_refactor: false,
            mentions_error: false,
            mentions_modify: false,
            mentions_explain: false,
            mentions_locate: false,
            mentions_add_feature: false,
        }
    }
}

pub(crate) fn analyze_query_features(query: &str) -> QueryFeatures {
    let lower = query.to_ascii_lowercase();
    QueryFeatures {
        raw_query: query.to_string(),
        symbol_like_terms: symbol_like_terms(query),
        file_like_terms: file_like_terms(query),
        error_like_terms: matching_terms(&lower, ERROR_TERMS),
        route_like_terms: route_like_terms(query),
        status_codes: status_codes(query),
        quoted_strings: quoted_strings(query),
        mentions_test: contains_any(&lower, TEST_TERMS),
        mentions_refactor: contains_any(&lower, REFACTOR_TERMS),
        mentions_error: contains_any(&lower, ERROR_TERMS),
        mentions_modify: contains_any(&lower, MODIFY_TERMS),
        mentions_explain: contains_any(&lower, EXPLAIN_TERMS),
        mentions_locate: contains_any(&lower, LOCATE_TERMS),
        mentions_add_feature: contains_any(&lower, ADD_FEATURE_TERMS),
    }
}

fn contains_any(text: &str, terms: &[&str]) -> bool {
    terms.iter().any(|term| text.contains(term))
}

fn matching_terms(text: &str, terms: &[&str]) -> Vec<String> {
    terms
        .iter()
        .filter(|term| text.contains(**term))
        .map(|term| (*term).to_string())
        .collect()
}

fn symbol_like_terms(query: &str) -> Vec<String> {
    split_terms(query)
        .into_iter()
        .filter(|term| term.contains("::") || looks_like_camel_symbol(term))
        .take(12)
        .collect()
}

fn file_like_terms(query: &str) -> Vec<String> {
    split_terms(query)
        .into_iter()
        .filter(|term| {
            term.contains('/')
                || term.contains('\\')
                || [".rs", ".kt", ".java", ".toml", ".gradle", ".json", ".md"]
                    .iter()
                    .any(|suffix| term.ends_with(suffix))
        })
        .take(12)
        .collect()
}

fn route_like_terms(query: &str) -> Vec<String> {
    split_terms(query)
        .into_iter()
        .filter(|term| term.starts_with('/') && term.len() > 1)
        .take(12)
        .collect()
}

fn status_codes(query: &str) -> Vec<u16> {
    let mut codes = Vec::new();
    for term in query.split(|ch: char| !ch.is_ascii_digit()) {
        if term.len() != 3 {
            continue;
        }
        if let Ok(code) = term.parse::<u16>() {
            if (100..=599).contains(&code) && !codes.contains(&code) {
                codes.push(code);
            }
        }
    }
    codes
}

fn quoted_strings(query: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for ch in query.chars() {
        if matches!(ch, '"' | '\'' | '`') {
            if quote == Some(ch) {
                if !current.trim().is_empty() {
                    out.push(current.trim().to_string());
                }
                current.clear();
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            } else {
                current.push(ch);
            }
            continue;
        }
        if quote.is_some() {
            current.push(ch);
        }
    }
    out
}

fn split_terms(query: &str) -> Vec<String> {
    query
        .split(|ch: char| {
            !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '/' | '\\' | '.' | '-' | '#'))
        })
        .map(str::trim)
        .filter(|term| term.len() >= 2)
        .map(ToOwned::to_owned)
        .collect()
}

fn looks_like_camel_symbol(term: &str) -> bool {
    let mut chars = term.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_uppercase() && chars.any(|ch| ch.is_ascii_lowercase())
}

const ERROR_TERMS: &[&str] = &[
    "报错",
    "错误",
    "异常",
    "失败",
    "日志",
    "panic",
    "error",
    "exception",
    "trace",
    "crash",
    "duplicate",
    "failed",
];
const TEST_TERMS: &[&str] = &["测试", "用例", "覆盖", "断言", "test", "tests", "assert"];
const REFACTOR_TERMS: &[&str] = &[
    "重构",
    "改名",
    "移动",
    "抽取",
    "拆分",
    "合并",
    "refactor",
    "rename",
    "extract",
    "references",
    "callers",
];
const MODIFY_TERMS: &[&str] = &[
    "修改", "修复", "改成", "调整", "变更", "删除", "返回", "fix", "change", "update",
];
const EXPLAIN_TERMS: &[&str] = &[
    "解释",
    "说明",
    "流程",
    "机制",
    "架构",
    "调用链",
    "为什么",
    "overview",
    "explain",
    "describe",
];
const LOCATE_TERMS: &[&str] = &[
    "在哪里",
    "哪个文件",
    "哪个函数",
    "入口",
    "定义",
    "where",
    "find",
    "locate",
];
const ADD_FEATURE_TERMS: &[&str] = &[
    "新增",
    "增加",
    "接入",
    "支持",
    "添加",
    "新功能",
    "add feature",
    "implement",
    "create",
];
