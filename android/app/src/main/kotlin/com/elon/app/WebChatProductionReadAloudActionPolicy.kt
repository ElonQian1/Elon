package com.elon.app

import com.elon.app.chatgptweb.ChatGptNativeControlPresentation

internal object WebChatProductionReadAloudActionPolicy {
    const val OFFICIAL_SEMANTIC = "read_aloud"
    const val SYSTEM_SEMANTIC = "system_read_aloud"
    const val PENDING_OFFICIAL_SEMANTIC = "official_read_aloud_pending"
    const val PRIVATE_CONTROL_PREFIX = "private_read_aloud:"

    fun officialLabel(rawLabel: String): String =
        if (isStopLabel(rawLabel)) "停止官网朗读" else "官网朗读"

    fun systemLabel(active: Boolean): String =
        if (active) "停止系统朗读" else "系统朗读"

    fun privateAction(
        contextId: String,
        state: WebChatConsumerState?,
    ): WebChatContextAction? {
        if (state?.privateReadAloudReady != true || contextId.isBlank()) return null
        val active = state.privateReadAloudContextId == contextId
        val phase = state.privateReadAloudState
        val label = when {
            active && phase == "loading" -> "取消官网朗读"
            active && phase == "playing" -> "停止官网朗读"
            else -> "官网朗读"
        }
        return WebChatContextAction(
            controlId = PRIVATE_CONTROL_PREFIX + contextId,
            semantic = OFFICIAL_SEMANTIC,
            label = label,
            requiresUserConfirmation = false,
            nativeSelector = officialSelector(contextId),
            subtitle = when (phase) {
                "loading" -> "正在准备官网声音"
                "cooldown" -> "官网朗读正在恢复"
                "failed" -> "上次连接失败，可重试"
                else -> null
            },
            enabled = phase != "cooldown",
        )
    }

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

    fun isPrivate(action: WebChatContextAction): Boolean =
        action.controlId.startsWith(PRIVATE_CONTROL_PREFIX)

    fun privateContextId(action: WebChatContextAction): String? = action.controlId
        .takeIf { it.startsWith(PRIVATE_CONTROL_PREFIX) }
        ?.removePrefix(PRIVATE_CONTROL_PREFIX)
        ?.takeIf(String::isNotBlank)

    fun needsOfficialPreparation(
        actions: List<WebChatContextAction>,
        portAvailable: Boolean,
    ): Boolean = portAvailable && actions.none(::isOfficial)

    fun isStopLabel(label: String): Boolean = STOP_LABEL.containsMatchIn(label.trim())

    private val STOP_LABEL = Regex("stop|pause|停止|暂停|结束", RegexOption.IGNORE_CASE)
}
