use super::summary_topic_hint;
use crate::{
    group_summary_context_pack::ensure_fb2_summary_policy_shape, store::GroupSummaryCreateInput,
};

#[test]
fn summary_topic_hint_prefers_topic_and_adds_instructions() {
    let input = GroupSummaryCreateInput {
        title: Some("今日比赛复盘".into()),
        topic: Some("竞彩焦点".into()),
        instructions: Some("重点看我的票和群友观点".into()),
        message_ids: Vec::new(),
        start_at: None,
        end_at: None,
        limit: 120,
        pin: false,
    };

    assert_eq!(
        summary_topic_hint(&input).as_deref(),
        Some("竞彩焦点；今日比赛复盘；重点看我的票和群友观点")
    );
}

#[test]
fn summary_topic_hint_deduplicates_empty_values() {
    let input = GroupSummaryCreateInput {
        title: Some("今日比赛".into()),
        topic: Some("今日比赛".into()),
        instructions: Some(" ".into()),
        message_ids: Vec::new(),
        start_at: None,
        end_at: None,
        limit: 120,
        pin: false,
    };

    assert_eq!(summary_topic_hint(&input).as_deref(), Some("今日比赛"));
}

#[test]
fn fb2_summary_policy_shape_adds_missing_boundaries() {
    let summary = "## 摘要\n- 今天主要讨论 A 队。";
    let context_pack =
        r#"{"external_app_context":{"answer_policy":{"schema":"fb2.answer_policy.v1"}}}"#;

    let shaped = ensure_fb2_summary_policy_shape(summary, context_pack, None);

    assert!(shaped.contains("## 数据事实"));
    assert!(shaped.contains("## AI推断"));
    assert!(shaped.contains("## 风险边界"));
    assert!(shaped.contains("不保证命中"));
    assert!(shaped.contains(summary));
}

#[test]
fn non_fb2_summary_policy_shape_is_unchanged() {
    let summary = "## 摘要\n- 普通群总结。";

    assert_eq!(
        ensure_fb2_summary_policy_shape(summary, r#"{"task":"group_summary_post"}"#, None),
        summary
    );
}
