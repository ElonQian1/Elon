package com.elon.app

data class ChatMessage(
    val role: String,
    var content: String,
    var attachments: List<ChatAttachment>? = null,
    var sendStatus: String? = null,
    var evidenceTitle: String? = null,
    var evidenceDetails: String? = null,
    var evidenceExpanded: Boolean = false,
    var evidenceWorking: Boolean = false,
    var senderLabel: String? = null,
    var id: String? = null,
    var apkUrl: String? = null,
    var senderAvatarDataUrl: String? = null,
    var senderAvatarResId: Int? = null,
    var suggestionStatus: String? = null,
    var suggestionResolvedByName: String? = null,
    var suggestionResolvedAt: String? = null,
    var canResolveSuggestion: Boolean = false,
    var createdAtMs: Long = System.currentTimeMillis(),
    /** 仅对自己发出的消息（role == "user"）有效：对方已读时为 true */
    var isRead: Boolean = false,
    /** 回答本条消息的模型 ID，如 "gpt-4o-mini"、"qwen2:7b" */
    var modelUsed: String? = null,
    /** 若回答来自用户贡献的 PC 节点，填写节点 ID */
    var nodeId: String? = null,
    /** Codex 桌面会话深链，用于把公开过程跳转到原生 Codex 线程。 */
    var codexThreadUri: String? = null,
    /** 本条消息只是过程承载层，终态回复出现后会把 evidence 合并到最终回复。 */
    var processLayer: Boolean = false,
    /** 只有任务结束时的真正终态回复为 true，用于显示“最终回复”标签。 */
    var finalReply: Boolean = false,
    /** 流式气泡 ID，用于 AssistantChunk 追加内容（打字机效果） */
    var streamId: String? = null,
    var recalledAt: String? = null,
    var recalledBy: String? = null,
    var revision: Long = 1,
    var editedAt: String? = null,
    var projectPostCard: ChatProjectPostCard? = null,
    var webChatMessage: WebChatProductionMessage? = null,
    var senderUserId: String? = null,
    var groupAiReply: String? = null,
)
