package com.elon.app

internal fun ProjectChannelMessage.toChatMessage(projectRole: String? = null): ChatMessage {
    val role = when (kind) {
        "ai_progress" -> "ai-progress"
        "ai_result" -> "ai-complete"
        "system" -> "ai"
        else -> if (outgoing) "user" else "friend"
    }
    return ChatMessage(
        role = role,
        content = content,
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
