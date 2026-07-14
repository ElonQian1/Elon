use super::*;

#[test]
fn auto_enables_vector_for_semantic_recall_tasks() {
    for query in ["解释登录流程", "新增 refresh token", "权限校验相关逻辑"] {
        let policy = choose_agent_vector_policy(query, None, None, None);

        assert!(policy.enabled, "{query}");
        assert!(!policy.explicit);
        assert_eq!(policy.model.as_deref(), Some(LOCAL_HASH_VECTOR_MODEL));
    }
}

#[test]
fn auto_disables_vector_for_precision_tasks() {
    for query in [
        "登录失败为什么返回 500？",
        "重构 AuthService::login callers",
        "AuthService::login 在哪里定义",
        "补充登录失败测试",
    ] {
        let policy = choose_agent_vector_policy(query, None, None, None);

        assert!(!policy.enabled, "{query}");
        assert!(!policy.explicit);
        assert_eq!(policy.model, None);
    }
}

#[test]
fn explicit_use_vector_overrides_intent_policy() {
    let debug_policy =
        choose_agent_vector_policy("登录失败为什么返回 500？", Some(true), None, None);
    assert!(debug_policy.enabled);
    assert!(debug_policy.explicit);
    assert_eq!(debug_policy.model.as_deref(), Some(LOCAL_HASH_VECTOR_MODEL));

    let explain_policy = choose_agent_vector_policy("解释登录流程", Some(false), None, None);
    assert!(!explain_policy.enabled);
    assert!(explain_policy.explicit);
    assert_eq!(explain_policy.model, None);
}

#[test]
fn configured_embedding_model_becomes_default_when_vector_enabled() {
    let policy = choose_agent_vector_policy(
        "解释登录流程",
        None,
        None,
        Some("openai:text-embedding-3-small".into()),
    );

    assert!(policy.enabled);
    assert_eq!(
        policy.model.as_deref(),
        Some("openai:text-embedding-3-small")
    );
}

#[test]
fn explicit_vector_model_overrides_configured_embedding_model() {
    let policy = choose_agent_vector_policy(
        "解释登录流程",
        None,
        Some("remote:bge-m3".into()),
        Some("openai:text-embedding-3-small".into()),
    );

    assert_eq!(policy.model.as_deref(), Some("remote:bge-m3"));
}
