package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebConversationPath
import com.elon.app.chatgptweb.ChatGptWebConversationShareReceipt

internal object WebChatConversationSharePolicy {
    private val projectId = Regex("g-p-[a-fA-F0-9]{32}")
    private val conversationId = Regex("[a-fA-F0-9]{8}(?:-[a-fA-F0-9]{4}){3}-[a-fA-F0-9]{12}")

    fun sharePath(path: String, project: String?): String? {
        val normalized = ChatGptWebConversationPath.normalize(path) ?: return null
        val inPath = ChatGptWebConversationPath.projectId(normalized)
        val inMetadata = project?.takeIf(String::isNotBlank)?.let(ChatGptWebConversationPath::canonicalProjectId)
        if (!project.isNullOrBlank() && inMetadata == null || inPath != null && inMetadata != null && inPath != inMetadata) return null
        val selected = inMetadata ?: inPath ?: return normalized
        val id = ChatGptWebConversationPath.identity(normalized) ?: return null
        return if (projectId.matches(selected) && conversationId.matches(id)) "/g/$selected/c/$id" else null
    }

    fun membersOnly(path: String): Boolean = ChatGptWebConversationPath.projectId(path) != null

    fun resultUrl(detail: String?, expectedPath: String? = null): String? {
        val link = ChatGptWebConversationShareReceipt.parse(detail) ?: return null
        val project = expectedPath?.let(ChatGptWebConversationPath::projectId)
        if (project == null) return link.url.takeIf { link.projectId == null }
        val id = ChatGptWebConversationPath.identity(expectedPath) ?: return null
        return link.url.takeIf { link.projectId == project && link.conversationId == id }
    }

    fun sameConversation(path: String, url: String): Boolean {
        val id = ChatGptWebConversationPath.identity(path) ?: return false
        val current = ChatGptWebConversationPath.fromUrl(url) ?: return false
        val expectedProject = ChatGptWebConversationPath.projectId(path)
        val actualProject = ChatGptWebConversationPath.projectId(current)
        return id == ChatGptWebConversationPath.identity(current) &&
            (expectedProject == null || actualProject == null || expectedProject == actualProject)
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
