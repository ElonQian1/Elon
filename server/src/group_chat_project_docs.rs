//! Default AI documents for group chat projects.

pub(crate) struct GroupChatDefaultDoc {
    pub path: &'static str,
    pub title: &'static str,
    pub content: &'static str,
}

const DEFAULT_GROUP_CHAT_DOCS: &[GroupChatDefaultDoc] = &[
    GroupChatDefaultDoc {
        path: "AGENTS.md",
        title: "群聊项目 AI 工作入口",
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../group-chat-project-docs/files/AGENTS.md"
        )),
    },
    GroupChatDefaultDoc {
        path: "AI_GROUP_CHAT.md",
        title: "AI 群聊项目说明",
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../group-chat-project-docs/files/AI_GROUP_CHAT.md"
        )),
    },
    GroupChatDefaultDoc {
        path: "AI_SUMMARY_POLICY.md",
        title: "群聊总结帖规则",
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../group-chat-project-docs/files/AI_SUMMARY_POLICY.md"
        )),
    },
    GroupChatDefaultDoc {
        path: "AI_TOPIC_SPLIT.md",
        title: "群聊议题拆分规则",
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../group-chat-project-docs/files/AI_TOPIC_SPLIT.md"
        )),
    },
    GroupChatDefaultDoc {
        path: "AI_RAG_POLICY.md",
        title: "群聊 RAG 检索规则",
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../group-chat-project-docs/files/AI_RAG_POLICY.md"
        )),
    },
    GroupChatDefaultDoc {
        path: "AI_CONTEXT_PACK.md",
        title: "群聊 Context Pack 规则",
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../group-chat-project-docs/files/AI_CONTEXT_PACK.md"
        )),
    },
    GroupChatDefaultDoc {
        path: ".elon/group-chat.json",
        title: "群聊项目元数据",
        content: include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../group-chat-project-docs/files/elon/group-chat.json"
        )),
    },
];

pub(crate) fn default_group_chat_docs() -> &'static [GroupChatDefaultDoc] {
    DEFAULT_GROUP_CHAT_DOCS
}
