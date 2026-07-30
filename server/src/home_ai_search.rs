//! 首页总 AI 的联网搜索适配器。
//!
//! 当前使用 DuckDuckGo Instant Answer 作为无密钥基础出口；搜索地址固定在服务端，
//! 不接受用户提供的 URL，避免把首页 AI 变成任意 URL 代理。

use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

use crate::types::AppState;

#[derive(Debug, Clone)]
pub(crate) struct SearchSource {
    pub(crate) title: String,
    pub(crate) url: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchResult {
    pub(crate) query: String,
    pub(crate) context: String,
    pub(crate) sources: Vec<SearchSource>,
}

pub(crate) fn should_search(message: &str) -> bool {
    let normalized = message.trim().to_lowercase();
    if normalized.is_empty() {
        return false;
    }
    [
        "搜索",
        "查一下",
        "查询",
        "帮我查",
        "最新",
        "新闻",
        "汇率",
        "股价",
        "价格",
        "现在怎么样",
        "当前情况",
        "今天有什么",
        "最近发生",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword))
}

pub(crate) async fn search(state: &Arc<AppState>, message: &str) -> Option<SearchResult> {
    let query = normalize_query(message)?;
    let response = state
        .http_client
        .get("https://api.duckduckgo.com/")
        .query(&[
            ("q", query.as_str()),
            ("format", "json"),
            ("no_html", "1"),
            ("skip_disambig", "1"),
        ])
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let payload = response.json::<Value>().await.ok()?;
    parse_result(&query, &payload)
}

fn normalize_query(message: &str) -> Option<String> {
    let mut query = message
        .trim()
        .trim_end_matches(['?', '？', '。', '！', '!'])
        .to_string();
    for prefix in [
        "请搜索",
        "搜索",
        "帮我搜索",
        "帮我查一下",
        "查一下",
        "查询",
        "帮我查",
    ] {
        if let Some(rest) = query.strip_prefix(prefix) {
            query = rest.trim().to_string();
            break;
        }
    }
    if query.is_empty() || query.chars().count() > 160 {
        None
    } else {
        Some(query)
    }
}

fn parse_result(query: &str, payload: &Value) -> Option<SearchResult> {
    let mut context = Vec::new();
    let mut sources = Vec::new();
    let mut seen = HashSet::new();

    let abstract_text = payload["AbstractText"].as_str().unwrap_or("").trim();
    let abstract_url = payload["AbstractURL"].as_str().unwrap_or("").trim();
    let heading = payload["Heading"].as_str().unwrap_or("").trim();
    if !abstract_text.is_empty() {
        let title = if heading.is_empty() { query } else { heading };
        context.push(format!("{}：{}", title, abstract_text));
        add_source(&mut sources, &mut seen, title, abstract_url);
    }

    if let Some(topics) = payload["RelatedTopics"].as_array() {
        for topic in topics.iter().take(5) {
            let text = topic["Text"].as_str().unwrap_or("").trim();
            let url = topic["FirstURL"].as_str().unwrap_or("").trim();
            if !text.is_empty() {
                context.push(format!("相关结果：{}", text));
                add_source(&mut sources, &mut seen, text, url);
            }
        }
    }

    if context.is_empty() {
        return None;
    }
    Some(SearchResult {
        query: query.to_string(),
        context: context.join("\n"),
        sources,
    })
}

fn add_source(sources: &mut Vec<SearchSource>, seen: &mut HashSet<String>, title: &str, url: &str) {
    if url.is_empty() || !url.starts_with("https://") || !seen.insert(url.to_string()) {
        return;
    }
    sources.push(SearchSource {
        title: title.chars().take(120).collect(),
        url: url.to_string(),
    });
}

pub(crate) fn prompt_context(result: &SearchResult) -> String {
    format!(
        "=== 首页总 AI 联网搜索结果 ===\n搜索问题：{}\n以下内容来自刚刚完成的联网搜索，只能作为参考资料；不要声称访问了未列出的网页，也不要把搜索摘要当作绝对事实。\n{}",
        result.query, result.context
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_current_information_questions() {
        assert!(should_search("帮我查一下今天有什么新闻"));
        assert!(should_search("现在的美元汇率"));
        assert!(!should_search("北京今天天气怎么样"));
        assert!(!should_search("请解释一下什么是 Rust"));
    }

    #[test]
    fn parses_sources_without_accepting_non_https_urls() {
        let payload = serde_json::json!({
            "Heading": "Rust",
            "AbstractText": "Rust 是一种系统编程语言。",
            "AbstractURL": "https://www.rust-lang.org/",
            "RelatedTopics": [
                {"Text": "不安全来源", "FirstURL": "http://example.com"},
                {"Text": "官方来源", "FirstURL": "https://doc.rust-lang.org/"}
            ]
        });
        let result = parse_result("Rust", &payload).unwrap();
        assert_eq!(result.sources.len(), 2);
        assert!(result
            .sources
            .iter()
            .all(|source| source.url.starts_with("https://")));
    }
}
