package com.elon.app

internal fun ProjectChannelMessage.toChatMessage(projectRole: String? = null): ChatMessage {
    val role = when (kind) {
        "ai_progress" -> "ai-progress"
        "ai_result" -> "ai-complete"
        "system" -> "ai"
        else -> if (outgoing) "user" else "friend"
    }
    val displayContent = parseProjectSpacePostText(content).detailText
    return ChatMessage(
        role = role,
        content = displayContent,
        senderLabel = if (role == "friend") senderName else null,
        id = id,
        suggestionStatus = suggestionStatus,
        suggestionResolvedByName = suggestionResolvedByName,
        suggestionResolvedAt = suggestionResolvedAt,
        canResolveSuggestion = canResolveSuggestion(projectRole),
        createdAtMs = parseChatMessageCreatedAt(createdAt) ?: 0L
    )
}

internal fun ProjectMemberConversationMessage.toChatMessage(): ChatMessage {
    val chatRole = when (role) {
        "assistant", "system" -> "ai"
        else -> if (outgoing) "user" else "friend"
    }
    return ChatMessage(
        role = chatRole,
        content = content,
        senderLabel = if (chatRole == "friend") senderName ?: userId ?: "成员" else null,
        id = id,
        createdAtMs = parseChatMessageCreatedAt(createdAt) ?: 0L
    )
}

internal fun ProjectChannelMessage.canResolveSuggestion(projectRole: String?): Boolean {
    return kind == "suggestion" &&
        suggestionStatus != "updated" &&
        canResolveProjectSuggestion(projectRole)
}

internal fun projectChannelHint(channel: ProjectChannel, projectRole: String?): String {
    channel.lastMessage?.takeIf { it.isNotBlank() }?.let { return it }
    return when (channel.kind) {
        "announcements" -> "项目公告、规则和重要更新。"
        "discussion" -> "成员日常讨论和协作交流。"
        "docs" -> "固定展示项目里的 AGENTS、CODEX、README、GitHub Copilot 指令和 docs 文档。"
        "requirements" -> "集中提出功能想法，后续可转为 AI 开发任务。"
        "suggestions" -> "游客和成员在这里发布建议，开发者完成后可标记已更新。"
        "issues" -> "反馈 bug、安装问题和体验问题。"
        "ai_development" -> if (projectRole == "observer") {
            "只读模式下可以询问 AI；涉及修改代码、编译或发布的请求会被拒绝。"
        } else {
            "在这里发消息会发起集体 AI 开发任务，过程和结果对成员可见。"
        }
        "builds" -> "构建、发布、APK 下载和部署结果记录。"
        else -> "项目成员共享频道。"
    }
}

internal data class ProjectSpacePostText(
    val title: String,
    val body: String,
    val structured: Boolean
) {
    val detailText: String
        get() = if (structured) {
            listOf(title, body).filter { it.isNotBlank() }.joinToString("\n\n")
        } else {
            body.ifBlank { title }
        }
}

internal fun formatProjectSpacePostContent(title: String, body: String): String {
    return buildString {
        append(PROJECT_SPACE_POST_TITLE_PREFIX)
        append(title.trim())
        append("\n\n")
        append(body.trim())
    }
}

internal fun parseProjectSpacePostText(content: String): ProjectSpacePostText {
    val trimmed = content.trim()
    if (trimmed.isBlank()) {
        return ProjectSpacePostText(title = "未命名帖子", body = "", structured = false)
    }
    if (trimmed.startsWith(PROJECT_SPACE_POST_TITLE_PREFIX)) {
        val withoutPrefix = trimmed.removePrefix(PROJECT_SPACE_POST_TITLE_PREFIX).trimStart()
        val parts = withoutPrefix.split(Regex("""\n\s*\n"""), limit = 2)
        val title = parts.getOrNull(0)?.trim().orEmpty().ifBlank { "未命名帖子" }
        val body = parts.getOrNull(1)?.trim().orEmpty()
        return ProjectSpacePostText(title = title, body = body, structured = true)
    }
    val lines = trimmed.lines()
    val first = lines.firstOrNull { it.isNotBlank() }?.trim().orEmpty()
    val title = first.removePrefix("#").trim().ifBlank { "未命名帖子" }
    return ProjectSpacePostText(title = title, body = trimmed, structured = false)
}

internal fun ProjectChannelMessage.isProjectSpaceFeedPost(): Boolean {
    return parseProjectSpacePostText(content).structured
}

internal fun ProjectChannel.isProjectSpaceFeedChannel(): Boolean {
    return kind in setOf("discussion", "requirements", "suggestions", "issues")
}

internal fun projectSpaceTopicLabel(channel: ProjectChannel): String {
    return when (channel.kind) {
        "discussion" -> "讨论"
        "requirements" -> "需求"
        "suggestions" -> "意见"
        "issues" -> "问题反馈"
        "announcements" -> "公告"
        else -> channel.name.ifBlank { "话题" }
    }
}

private const val PROJECT_SPACE_POST_TITLE_PREFIX = "标题："
