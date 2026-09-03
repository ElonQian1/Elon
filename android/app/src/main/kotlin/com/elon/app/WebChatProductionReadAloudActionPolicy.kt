package com.elon.app

import com.elon.app.chatgptweb.ChatGptNativeControlPresentation

internal object WebChatProductionReadAloudActionPolicy {
    const val OFFICIAL_SEMANTIC = "read_aloud"
    const val SYSTEM_SEMANTIC = "system_read_aloud"
    const val PENDING_OFFICIAL_SEMANTIC = "official_read_aloud_pending"

    fun officialLabel(rawLabel: String): String =
        if (isStopLabel(rawLabel)) "停止官网朗读" else "官网朗读"

    fun systemLabel(active: Boolean): String =
        if (active) "停止系统朗读" else "系统朗读"

    fun officialSelector(contextId: String): String =
        "web-chat-message-context-action:" +
            ChatGptNativeControlPresentation.stableContextId(contextId) +
            ":official-read-aloud"

    fun systemSelector(contextId: String): String =
        "web-chat-message-context-action:" +
            ChatGptNativeControlPresentation.stableContextId(contextId) +
            ":system-read-aloud"

    fun pendingOfficialAction(contextId: String): WebChatContextAction = WebChatContextAction(
        controlId = "official_read_aloud_pending:$contextId",
        semantic = PENDING_OFFICIAL_SEMANTIC,
        label = "官网朗读",
        requiresUserConfirmation = false,
        nativeSelector = officialSelector(contextId),
        subtitle = "正在准备官网声音",
        enabled = false,
    )

    fun isOfficial(action: WebChatContextAction): Boolean =
        action.semantic == OFFICIAL_SEMANTIC

    fun needsOfficialPreparation(
        actions: List<WebChatContextAction>,
        portAvailable: Boolean,
    ): Boolean = portAvailable && actions.none(::isOfficial)

    fun isStopLabel(label: String): Boolean = STOP_LABEL.containsMatchIn(label.trim())

    private val STOP_LABEL = Regex("stop|pause|停止|暂停|结束", RegexOption.IGNORE_CASE)
}
