pub fn chat_reply_after_intent_gate(user_message: &str, codex_reply: Option<String>) -> String {
    if let Some(reply) = codex_reply {
        let reply = reply.trim();
        if !reply.is_empty() && !looks_like_clarification_only(reply) {
            return reply.to_string();
        }
    }

    if looks_like_multi_device_project_question(user_message) {
        return "可以分两层理解：多手机同时登录或聊天本身可以并行；同一项目的多个开发会话会使用各自的 worktree/分支并行编码，再由服务器串行合并、打包或发布。我先按普通讨论处理，不进入改代码、打包或发布流程。"
            .into();
    }

    "我先按普通聊天处理，不进入改代码、编译或发布流程。你可以继续问；如果要我实际检查项目或动代码，再直接说明。".into()
}

pub(crate) fn append_nonempty_ws_text(buffer: &mut String, text: Option<&str>) {
    let Some(text) = text.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    if !buffer.is_empty() && !buffer.ends_with('\n') {
        buffer.push('\n');
    }
    buffer.push_str(text);
}

fn looks_like_clarification_only(reply: &str) -> bool {
    [
        "没看懂",
        "没看清",
        "没法确定",
        "具体想问",
        "你是想问",
        "你可以直接说",
        "没能准确识别",
        "可以直接问",
        "如果是要我立刻",
        "我先按普通聊天处理",
        "不进入改代码",
    ]
    .iter()
    .any(|marker| reply.contains(marker))
}

fn looks_like_multi_device_project_question(message: &str) -> bool {
    let lower = message.to_lowercase();
    ["多手机", "多个手机", "多端", "同时登录", "并行", "冲突"]
        .iter()
        .any(|word| lower.contains(word))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_useful_codex_gate_reply() {
        let reply = chat_reply_after_intent_gate(
            "我们的 apk 能不能多端使用？",
            Some("可以，普通聊天可以并行。".into()),
        );
        assert_eq!(reply, "可以，普通聊天可以并行。");
    }

    #[test]
    fn replaces_clarification_for_multi_device_project_question() {
        let reply = chat_reply_after_intent_gate(
            "我们的apk是否支持多个手机同时登录？",
            Some("我没法确定你具体想问 APK 的哪方面。".into()),
        );
        assert!(reply.contains("多手机同时登录或聊天本身可以并行"));
        assert!(reply.contains("worktree/分支"));
        assert!(reply.contains("不进入改代码"));
    }

    #[test]
    fn replaces_generic_guard_for_multi_device_project_question() {
        let reply = chat_reply_after_intent_gate(
            "我们的apk是否支持多个手机同时登录？",
            Some("我先按普通聊天处理，不进入改代码、编译或发布流程。".into()),
        );
        assert!(reply.contains("多手机同时登录或聊天本身可以并行"));
    }

    #[test]
    fn replaces_recognition_failure_for_multi_device_project_question() {
        let reply = chat_reply_after_intent_gate(
            "我们的apk是否支持多个手机同时登录？",
            Some("我没能准确识别这句话的意思。".into()),
        );
        assert!(reply.contains("多手机同时登录或聊天本身可以并行"));
    }
}
