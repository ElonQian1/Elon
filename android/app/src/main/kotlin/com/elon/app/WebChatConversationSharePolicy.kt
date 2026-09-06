package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationPath

internal object WebChatConversationSharePolicy {
    private const val PREFIX = "share_link_ready:"
    private val publicLink = Regex("https://chatgpt\\.com/share/[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}")

    fun resultUrl(detail: String?): String? = detail?.takeIf { it.startsWith(PREFIX) }
        ?.removePrefix(PREFIX)?.takeIf { publicLink.matches(it) }

    fun sameConversation(path: String, url: String): Boolean {
        val id = ChatGptWebConversationPath.identity(path) ?: return false
        return id == ChatGptWebConversationPath.identity(url)
    }

    fun errorMessage(code: String?): String = when (code) {
        "share_result_unconfirmed", "share_cooldown" ->
            "未能确认分享结果，链接可能已经创建。请在官网查看，暂不重复创建。"
        "share_moderation_blocked" -> "官网未允许分享这段会话。"
        "share_project_scope_unconfirmed" -> "项目会话有不同的成员访问权限，请在官网确认分享范围。"
        "share_http_401", "share_http_403", "share_auth_unavailable" ->
            "当前登录状态或分享权限尚未确认，请在官网检查。"
        "share_busy", "share_conversation_busy" -> "会话仍在处理其他操作，请结束后再分享。"
        "share_context_changed" -> "会话已经变化，请重新选择要分享的会话。"
        else -> "当前会话的分享条件尚未确认，可以稍后重试或在官网分享。"
    }
}
