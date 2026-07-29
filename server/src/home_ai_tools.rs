//! 首页总 AI 的确定性基础能力。
//!
//! 这类问题不应该交给模型猜测：日期、时间和简单计算由服务端直接处理，
//! 既节省一次模型调用，也避免模型因没有实时钟表而拒答或答错。

use crate::store::{is_system_project_source_type, ProjectSummary};
use chrono::{DateTime, Datelike, FixedOffset, Utc};

pub(crate) struct DeterministicAnswer {
    pub(crate) tool: &'static str,
    pub(crate) reply: String,
}

pub(crate) fn now() -> DateTime<FixedOffset> {
    let offset_hours = std::env::var("ELON_ASSISTANT_UTC_OFFSET_HOURS")
        .ok()
        .and_then(|value| value.trim().parse::<i32>().ok())
        .filter(|hours| (-12..=14).contains(hours))
        .unwrap_or(8);
    let seconds = offset_hours * 60 * 60;
    let offset = FixedOffset::east_opt(seconds)
        .unwrap_or_else(|| FixedOffset::east_opt(8 * 60 * 60).unwrap());
    Utc::now().with_timezone(&offset)
}

pub(crate) fn runtime_note(current: DateTime<FixedOffset>) -> String {
    format!(
        "=== 首页总 AI 运行环境 ===\n当前日期时间：{}（UTC{}，默认按北京时间 Asia/Shanghai）\n你可以回答普通知识；涉及最新新闻、天气、价格等实时信息时，优先使用首页提供的联网搜索结果，并明确区分搜索资料与确定事实。日期、时间和简单计算由服务端确定性处理。涉及代码修改、项目文件或构建任务时，引导用户进入对应项目 AI。",
        format_datetime(current),
        format_offset(current.offset().local_minus_utc())
    )
}

pub(crate) fn deterministic_answer(
    message: &str,
    current: DateTime<FixedOffset>,
) -> Option<DeterministicAnswer> {
    let normalized = message
        .trim()
        .to_lowercase()
        .replace('？', "?")
        .replace('。', "")
        .replace('！', "!");

    if asks_weekday(&normalized) {
        return Some(DeterministicAnswer {
            tool: "current_datetime",
            reply: format!(
                "今天是{}年{}月{}日，{}。",
                current.year(),
                current.month(),
                current.day(),
                weekday_name(current)
            ),
        });
    }

    if asks_clock(&normalized) {
        return Some(DeterministicAnswer {
            tool: "current_datetime",
            reply: format!("现在是{}（北京时间）。", current.format("%H:%M:%S")),
        });
    }

    if asks_date(&normalized) {
        return Some(DeterministicAnswer {
            tool: "current_datetime",
            reply: format!(
                "今天是{}年{}月{}日，{}。",
                current.year(),
                current.month(),
                current.day(),
                weekday_name(current)
            ),
        });
    }

    let expression = extract_expression(&normalized)?;
    let value = Calculator::new(&expression).parse().ok()?;
    if !value.is_finite() {
        return None;
    }
    Some(DeterministicAnswer {
        tool: "calculator",
        reply: format!("计算结果：{}", format_number(value)),
    })
}

pub(crate) fn needs_project_handoff(message: &str) -> bool {
    let normalized = message.trim().to_lowercase();
    let has_action = [
        "修改",
        "开发",
        "创建",
        "构建",
        "编译",
        "打包",
        "修复",
        "读取",
        "执行",
        "写代码",
        "codex",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword));
    let has_project_subject = [
        "代码", "项目", "app", "应用", "apk", "文件", "命令", "程序", "功能", "bug",
    ]
    .iter()
    .any(|keyword| normalized.contains(keyword));
    has_action && has_project_subject
}

pub(crate) fn project_candidates(
    message: &str,
    projects: &[ProjectSummary],
) -> Vec<serde_json::Value> {
    let normalized = message.trim().to_lowercase();
    let mut matched = projects
        .iter()
        .filter(|project| !is_system_project_source_type(&project.source_type))
        .filter_map(|project| {
            let name = project.name.to_lowercase();
            let display_name = project.display_name.as_deref().unwrap_or("").to_lowercase();
            let score = if (!name.is_empty() && normalized.contains(&name))
                || (!display_name.is_empty() && normalized.contains(&display_name))
            {
                2
            } else {
                1
            };
            Some((score, project))
        })
        .collect::<Vec<_>>();
    let has_explicit_match = matched.iter().any(|(score, _)| *score == 2);
    matched.retain(|(score, _)| has_explicit_match && *score == 2 || !has_explicit_match);
    matched.sort_by(|(left, _), (right, _)| right.cmp(left));
    matched
        .into_iter()
        .take(5)
        .map(|(_, project)| {
            serde_json::json!({
                "id": project.id,
                "name": project.display_name.as_deref().unwrap_or(&project.name),
                "description": project.description,
            })
        })
        .collect()
}

fn asks_weekday(message: &str) -> bool {
    (message.contains("星期几") || message.contains("周几"))
        && (message.contains("今天") || message.contains("现在") || message.contains("当前"))
}

fn asks_clock(message: &str) -> bool {
    (message.contains("几点") || message.contains("时间"))
        && (message.contains("现在") || message.contains("当前") || message == "时间")
}

fn asks_date(message: &str) -> bool {
    (message.contains("几号") || message.contains("日期") || message.contains("几月几日"))
        && (message.contains("今天") || message.contains("现在") || message.contains("当前"))
}

fn extract_expression(message: &str) -> Option<String> {
    let mut candidate = message.trim().to_string();
    for prefix in ["请计算", "计算", "算一下", "帮我算", "帮我计算"] {
        if let Some(rest) = candidate.strip_prefix(prefix) {
            candidate = rest.trim().to_string();
            break;
        }
    }
    candidate = candidate
        .trim_end_matches('?')
        .trim_end_matches('=')
        .trim()
        .replace("乘以", "*")
        .replace("乘", "*")
        .replace("除以", "/")
        .replace("除", "/")
        .replace('加', "+")
        .replace('减', "-")
        .replace('×', "*")
        .replace('÷', "/")
        .replace('－', "-");

    if candidate.is_empty()
        || candidate.len() > 120
        || !candidate
            .chars()
            .any(|ch| matches!(ch, '+' | '-' | '*' | '/'))
        || !candidate.chars().all(|ch| {
            ch.is_ascii_digit() || matches!(ch, '+' | '-' | '*' | '/' | '.' | '(' | ')' | ' ')
        })
    {
        return None;
    }
    Some(candidate)
}

struct Calculator<'a> {
    chars: Vec<char>,
    position: usize,
    source: &'a str,
}

impl<'a> Calculator<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            position: 0,
            source,
        }
    }

    fn parse(mut self) -> Result<f64, ()> {
        let value = self.parse_expression()?;
        self.skip_spaces();
        if self.position == self.chars.len() {
            Ok(value)
        } else {
            Err(())
        }
    }

    fn parse_expression(&mut self) -> Result<f64, ()> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_spaces();
            let operation = self.peek();
            if !matches!(operation, Some('+') | Some('-')) {
                break;
            }
            self.position += 1;
            let rhs = self.parse_term()?;
            value = if operation == Some('+') {
                value + rhs
            } else {
                value - rhs
            };
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<f64, ()> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_spaces();
            let operation = self.peek();
            if !matches!(operation, Some('*') | Some('/')) {
                break;
            }
            self.position += 1;
            let rhs = self.parse_factor()?;
            if operation == Some('/') && rhs == 0.0 {
                return Err(());
            }
            value = if operation == Some('*') {
                value * rhs
            } else {
                value / rhs
            };
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<f64, ()> {
        self.skip_spaces();
        if self.peek() == Some('(') {
            self.position += 1;
            let value = self.parse_expression()?;
            self.skip_spaces();
            if self.peek() != Some(')') {
                return Err(());
            }
            self.position += 1;
            return Ok(value);
        }
        let start = self.position;
        if self.peek() == Some('-') {
            self.position += 1;
        }
        while self
            .peek()
            .is_some_and(|ch| ch.is_ascii_digit() || ch == '.')
        {
            self.position += 1;
        }
        if start == self.position {
            return Err(());
        }
        self.source[start..self.position]
            .parse::<f64>()
            .map_err(|_| ())
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn skip_spaces(&mut self) {
        while self.peek() == Some(' ') {
            self.position += 1;
        }
    }
}

fn weekday_name(current: DateTime<FixedOffset>) -> &'static str {
    match current.weekday().number_from_monday() {
        1 => "星期一",
        2 => "星期二",
        3 => "星期三",
        4 => "星期四",
        5 => "星期五",
        6 => "星期六",
        _ => "星期日",
    }
}

fn format_datetime(current: DateTime<FixedOffset>) -> String {
    format!(
        "{}年{}月{}日 {}",
        current.year(),
        current.month(),
        current.day(),
        current.format("%H:%M:%S")
    )
}

fn format_offset(seconds: i32) -> String {
    let sign = if seconds >= 0 { '+' } else { '-' };
    let hours = seconds.unsigned_abs() / 3600;
    format!("{}{}", sign, hours)
}

fn format_number(value: f64) -> String {
    if (value - value.round()).abs() < 1e-10 {
        return format!("{:.0}", value);
    }
    format!("{:.10}", value)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample_time() -> DateTime<FixedOffset> {
        FixedOffset::east_opt(8 * 60 * 60)
            .unwrap()
            .with_ymd_and_hms(2026, 7, 29, 16, 0, 0)
            .unwrap()
    }

    #[test]
    fn answers_weekday_from_runtime_clock() {
        let answer = deterministic_answer("今天星期几？", sample_time()).unwrap();
        assert_eq!(answer.tool, "current_datetime");
        assert!(answer.reply.contains("星期三"));
    }

    #[test]
    fn answers_simple_calculation_without_model() {
        let answer = deterministic_answer("计算 (12 + 8) * 3", sample_time()).unwrap();
        assert_eq!(answer.tool, "calculator");
        assert_eq!(answer.reply, "计算结果：60");
    }

    #[test]
    fn rejects_non_math_text() {
        assert!(deterministic_answer("帮我写一个计算器页面", sample_time()).is_none());
    }

    #[test]
    fn identifies_project_work_handoff() {
        assert!(needs_project_handoff("帮我修改这个项目的登录页面"));
        assert!(needs_project_handoff("请用 Codex 修复 APK 的 bug"));
        assert!(!needs_project_handoff("项目 AI 是做什么的？"));
    }

    #[test]
    fn prefers_explicit_project_name_for_handoff() {
        let project = |id: &str, name: &str| ProjectSummary {
            id: id.into(),
            name: name.into(),
            display_name: None,
            description: None,
            workspace_key: id.into(),
            template: "android".into(),
            source_type: "template".into(),
            repo_url: None,
            branch: None,
            workspace_path: None,
            node_id: None,
            storage_node_id: None,
            storage_repo_path: None,
            storage_repo_url: None,
            storage_worktree_path: None,
            storage_status: "none".into(),
            status: "active".into(),
            role: "owner".into(),
            member_count: 1,
            is_public: false,
            join_mode: "invite".into(),
            runtime_permission: "project_write".into(),
            last_task_status: None,
            last_apk_url: None,
            icon_data_url: None,
            updated_at: "2026-01-01T00:00:00Z".into(),
        };
        let candidates = project_candidates(
            "修改天气 App 的代码",
            &[project("one", "天气 App"), project("two", "记账 App")],
        );
        assert_eq!(candidates[0]["id"], "one");
    }
}
