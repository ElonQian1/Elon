use super::{GroupChatRetrievalInput, GroupSummaryCreateInput, Store};

fn temp_store() -> Store {
    let path = std::env::temp_dir().join(format!(
        "elon_group_summary_{}.db",
        uuid::Uuid::new_v4().simple()
    ));
    Store::open(&path).expect("store should open")
}

#[test]
fn group_summary_post_keeps_context_sources_and_pin_state() {
    let store = temp_store();
    let owner = store
        .create_user("summary-owner@example.com", "secret1", Some("群主"), None)
        .expect("owner should create");
    let member = store
        .create_user("summary-member@example.com", "secret1", Some("成员"), None)
        .expect("member should create");
    store
        .add_friend(&owner.id, Some("email"), "summary-member@example.com")
        .expect("friend pair should create");
    let group = store
        .create_friend_group(&owner.id, Some("总结测试群"), &[member.id.clone()])
        .expect("group should create");
    let first = store
        .send_friend_group_message(&owner.id, &group.id, "上午讨论发布节奏", None)
        .expect("first message should insert");
    let second = store
        .send_friend_group_message(&member.id, &group.id, "下午改成先发灰度版本", None)
        .expect("second message should insert");

    let docs = store
        .list_group_ai_documents(&owner.id, &group.id)
        .expect("docs should seed");
    assert!(docs.iter().any(|doc| doc.path == "AI_CONTEXT_PACK.md"));

    let input = GroupSummaryCreateInput {
        title: Some("发布节奏讨论".into()),
        topic: Some("发布节奏".into()),
        instructions: Some("请聚焦上午和下午的决策变化".into()),
        message_ids: vec![first.id.clone(), second.id.clone()],
        start_at: None,
        end_at: None,
        limit: 120,
        pin: true,
    };
    let sources = store
        .group_summary_messages_for_context(&owner.id, &group.id, &input)
        .expect("sources should load");
    assert_eq!(sources.len(), 2);

    let detail = store
        .create_group_summary_post_draft(
            &owner.id,
            &group.id,
            &input,
            &sources,
            r#"{"task":"group_summary_post"}"#,
        )
        .expect("draft should create");
    assert_eq!(detail.post.status, "generating");
    assert_eq!(detail.sources.len(), 2);
    assert!(detail.post.pinned_at.is_some());

    store
        .update_group_summary_post_result(
            &group.id,
            &detail.post.id,
            "## 摘要\n- 先灰度发布。\n\n## 相关发言\n- `gmsg`",
            "ready",
            Some("test-model"),
            None,
        )
        .expect("summary should update");
    let updated = store
        .group_summary_post_detail(&owner.id, &group.id, &detail.post.id)
        .expect("detail should reload");
    assert_eq!(updated.post.status, "ready");
    assert_eq!(updated.post.model_used.as_deref(), Some("test-model"));

    let unpinned = store
        .edit_group_summary_post(
            &owner.id,
            &group.id,
            &detail.post.id,
            None,
            None,
            Some(false),
        )
        .expect("post should unpin");
    assert!(unpinned.post.pinned_at.is_none());
}

#[test]
fn group_chat_retrieval_scores_keyword_and_sender_matches() {
    let store = temp_store();
    let owner = store
        .create_user("retrieval-owner@example.com", "secret1", Some("群主"), None)
        .expect("owner should create");
    let member = store
        .create_user(
            "retrieval-member@example.com",
            "secret1",
            Some("发布同学"),
            None,
        )
        .expect("member should create");
    store
        .add_friend(&owner.id, Some("email"), "retrieval-member@example.com")
        .expect("friend pair should create");
    let group = store
        .create_friend_group(&owner.id, Some("检索测试群"), &[member.id.clone()])
        .expect("group should create");
    store
        .send_friend_group_message(&owner.id, &group.id, "上午先讨论菜单样式", None)
        .expect("first message should insert");
    let target = store
        .send_friend_group_message(&member.id, &group.id, "下午发布灰度版本，观察错误码", None)
        .expect("target message should insert");

    let result = store
        .search_group_chat_messages(
            &owner.id,
            &group.id,
            &GroupChatRetrievalInput {
                query: Some("灰度 错误码".into()),
                sender: Some("发布同学".into()),
                message_ids: Vec::new(),
                start_at: None,
                end_at: None,
                limit: 10,
            },
        )
        .expect("search should run");
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].message.id, target.id);
    assert!(result.hits[0]
        .match_reasons
        .iter()
        .any(|reason| reason == "sender_filter"));
    assert!(result.strategy.contains(&"keyword_full_text".to_string()));
    assert_eq!(result.vector_status, "pending_group_chat_embedding_index");
}

#[test]
fn group_chat_retrieval_keeps_exact_message_order() {
    let store = temp_store();
    let owner = store
        .create_user(
            "retrieval-id-owner@example.com",
            "secret1",
            Some("群主"),
            None,
        )
        .expect("owner should create");
    let member = store
        .create_user(
            "retrieval-id-member@example.com",
            "secret1",
            Some("成员"),
            None,
        )
        .expect("member should create");
    store
        .add_friend(&owner.id, Some("email"), "retrieval-id-member@example.com")
        .expect("friend pair should create");
    let group = store
        .create_friend_group(&owner.id, Some("ID检索测试群"), &[member.id.clone()])
        .expect("group should create");
    let first = store
        .send_friend_group_message(&owner.id, &group.id, "第一条", None)
        .expect("first message should insert");
    let second = store
        .send_friend_group_message(&member.id, &group.id, "第二条", None)
        .expect("second message should insert");

    let result = store
        .search_group_chat_messages(
            &owner.id,
            &group.id,
            &GroupChatRetrievalInput {
                query: None,
                sender: None,
                message_ids: vec![second.id.clone(), first.id.clone()],
                start_at: None,
                end_at: None,
                limit: 10,
            },
        )
        .expect("search should run");
    assert_eq!(result.hits.len(), 2);
    assert_eq!(result.hits[0].message.id, first.id);
    assert_eq!(result.hits[1].message.id, second.id);
    assert!(result.strategy.contains(&"exact_message_ids".to_string()));
}
