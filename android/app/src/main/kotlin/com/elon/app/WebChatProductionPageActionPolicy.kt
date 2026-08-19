package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationPath
import java.net.URI

internal data class WebChatProductionPageAction(
    val control: WebChatConsumerControl,
    val label: String,
    val requiresUserConfirmation: Boolean,
    val officialFallback: Boolean,
    val nativeSelector: String,
) {
    val controlId: String get() = control.id
    val semantic: String get() = control.semantic
}

internal data class WebChatProductionPageIdentity(
    val pageKind: String,
    val path: String,
    val conversationId: String?,
) {
    val actionPlacement: WebChatConsumerPageActionPlacement
        get() = if (pageKind == CONVERSATION_PAGE_KIND) {
            WebChatConsumerPageActionPlacement.CONVERSATION
        } else {
            WebChatConsumerPageActionPlacement.PAGE
        }

    val sheetTitle: String
        get() = if (actionPlacement == WebChatConsumerPageActionPlacement.CONVERSATION) {
            "会话设置"
        } else {
            "网页功能"
        }

    val hasConversationTarget: Boolean
        get() = actionPlacement == WebChatConsumerPageActionPlacement.CONVERSATION &&
            conversationId != null

    val cacheKey: String get() = "$pageKind:$path"

    companion object {
        fun from(state: WebChatConsumerState): WebChatProductionPageIdentity {
            val kind = state.pageKind.trim().lowercase().ifBlank { "unknown" }
            val path = normalizedPath(state.pageUrl)
            return WebChatProductionPageIdentity(
                pageKind = kind,
                path = path,
                conversationId = if (kind == CONVERSATION_PAGE_KIND) {
                    ChatGptWebConversationPath.identity(path)
                } else null,
            )
        }

        private fun normalizedPath(value: String): String {
            val trimmed = value.trim()
            if (trimmed.isBlank()) return "/"
            val parsed = runCatching { URI(trimmed).path }.getOrNull().orEmpty()
            return (parsed.ifBlank { trimmed.substringBefore('?').substringBefore('#') })
                .trim()
                .ifBlank { "/" }
                .take(MAX_PAGE_PATH_LENGTH)
        }

        private const val CONVERSATION_PAGE_KIND = "conversation"
        private const val MAX_PAGE_PATH_LENGTH = 320
    }
}

internal object WebChatProductionPageActionEntryPolicy {
    fun visible(
        provider: WebChatProviderIdentity,
        currentConversationPath: String?,
        state: String,
    ): Boolean = provider.supports(WebChatProviderCapability.PAGE_ACTIONS) &&
        state == "ready" &&
        ChatGptWebConversationPath.normalize(currentConversationPath) != null
}

internal object WebChatProductionPageActionParser {
    fun parse(
        descriptors: List<WebChatConsumerControlDescriptor>,
        pageIdentity: WebChatProductionPageIdentity,
    ): List<WebChatProductionPageAction> = descriptors.asSequence()
        .filter(::isPresentable)
        .filter { it.pageActionPlacement == pageIdentity.actionPlacement }
        .filter { descriptor -> belongsToCurrentPage(descriptor.control, pageIdentity) }
        .map { descriptor ->
            val control = descriptor.control
            WebChatProductionPageAction(
                control = control,
                label = displayLabel(control),
                requiresUserConfirmation = descriptor.requiresUserConfirmation,
                officialFallback = descriptor.presentation ==
                    WebChatConsumerControlPresentation.OFFICIAL_FALLBACK ||
                    control.semantic in OFFICIAL_COMPLETION_SEMANTICS,
                nativeSelector = descriptor.nativeSelector
                    .orEmpty()
                    .trim()
                    .ifBlank { "web-chat-page-action:${control.semantic}:${control.id}" },
            )
        }
        .distinctBy { action ->
            "${action.semantic}:${action.control.contextId.orEmpty()}:${action.label}"
        }
        .sortedBy { ACTION_ORDER[it.semantic] ?: Int.MAX_VALUE }
        .take(MAX_VISIBLE_ACTIONS)
        .toList()

    private fun isPresentable(descriptor: WebChatConsumerControlDescriptor): Boolean =
        descriptor.control.enabled &&
            descriptor.control.region in PAGE_REGIONS &&
            descriptor.pageActionPlacement != WebChatConsumerPageActionPlacement.NONE &&
            descriptor.presentation in SUPPORTED_PRESENTATIONS

    private fun belongsToCurrentPage(
        control: WebChatConsumerControl,
        identity: WebChatProductionPageIdentity,
    ): Boolean {
        if (identity.actionPlacement != WebChatConsumerPageActionPlacement.CONVERSATION) return true
        if (control.semantic == "temporary_chat") return true
        val expectedContext = identity.conversationId
        val observedContext = control.contextId?.trim().orEmpty()
        if (expectedContext != null && observedContext.isNotBlank()) {
            return observedContext == expectedContext
        }
        if (control.semantic == "conversation_options") {
            return expectedContext == null && observedContext.isBlank() && control.region == "header"
        }
        return control.region == "overlay"
    }

    private fun displayLabel(control: WebChatConsumerControl): String = when (control.semantic) {
        "conversation_options" -> "会话设置"
        "conversation_files" -> "会话文件"
        "temporary_chat" -> if (control.selected) "关闭临时聊天" else "临时聊天"
        "save_to_project" -> "添加到项目"
        "rename" -> "重命名"
        "pin" -> if (UNPIN_LABEL.containsMatchIn(control.label)) "取消置顶" else "置顶"
        "archive" -> if (UNARCHIVE_LABEL.containsMatchIn(control.label)) "取消归档" else "归档"
        "share" -> "分享"
        "delete" -> "删除"
        "profile" -> "账号与设置"
        "personalization" -> "个性化"
        "settings" -> "设置"
        "apps" -> "应用"
        "help" -> "帮助"
        "plan" -> "套餐与订阅"
        "logout" -> "退出登录"
        "create_asset" -> "创建内容"
        "open_media" -> "打开媒体"
        else -> control.label.trim().take(MAX_LABEL_LENGTH)
    }

    private const val MAX_LABEL_LENGTH = 48
    private const val MAX_VISIBLE_ACTIONS = 16
    private val PAGE_REGIONS = setOf("header", "content", "suggestions", "overlay")
    private val SUPPORTED_PRESENTATIONS = setOf(
        WebChatConsumerControlPresentation.DIRECT,
        WebChatConsumerControlPresentation.DEDICATED,
        WebChatConsumerControlPresentation.MENU,
        WebChatConsumerControlPresentation.OFFICIAL_FALLBACK,
    )
    private val OFFICIAL_COMPLETION_SEMANTICS = setOf(
        "conversation_files",
        "share",
        "create_asset",
        "open_media",
        "personalization",
        "plan",
        "logout",
    )
    private val ACTION_ORDER = listOf(
        "conversation_options",
        "share",
        "save_to_project",
        "conversation_files",
        "rename",
        "pin",
        "archive",
        "temporary_chat",
        "delete",
        "profile",
        "personalization",
        "settings",
        "apps",
        "help",
        "plan",
        "logout",
        "create_asset",
        "open_media",
    ).withIndex().associate { (index, semantic) -> semantic to index }
    private val UNPIN_LABEL = Regex("unpin|取消置顶", RegexOption.IGNORE_CASE)
    private val UNARCHIVE_LABEL = Regex("unarchive|取消归档", RegexOption.IGNORE_CASE)
}
