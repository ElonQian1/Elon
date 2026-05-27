package com.elon.app

internal data class SelectedDiscussionSummary(
    val transcript: String,
    val channelPost: String,
    val channelPrompt: String,
    val personalPrompt: String
)

internal fun buildSelectedDiscussionSummary(messages: List<ChatMessage>): SelectedDiscussionSummary {
    val transcript = selectedDiscussionTranscript(messages)
    return SelectedDiscussionSummary(
        transcript = transcript,
        channelPost = selectedDiscussionChannelPost(messages.size, transcript),
        channelPrompt = selectedDiscussionChannelPrompt(transcript),
        personalPrompt = selectedDiscussionPersonalPrompt(transcript)
    )
}

internal fun selectedDiscussionTranscript(messages: List<ChatMessage>): String {
    val builder = StringBuilder()
    messages.forEachIndexed { index, message ->
        if (builder.length >= MAX_SELECTED_DISCUSSION_CHARS) return@forEachIndexed
        val line = "${index + 1}. ${message.selectionSpeakerLabel()}: ${message.selectionContent()}"
        val remaining = MAX_SELECTED_DISCUSSION_CHARS - builder.length
        builder.appendLine(line.take(remaining))
    }
    if (builder.length >= MAX_SELECTED_DISCUSSION_CHARS) {
        builder.appendLine("（后续内容过长，已截断）")
    }
    return builder.toString().trim()
}

private fun selectedDiscussionChannelPost(count: Int, transcript: String): String {
    return """
        【合并聊天记录】共 ${count} 条

        $transcript
    """.trimIndent()
}

private fun selectedDiscussionChannelPrompt(transcript: String): String {
    return """
        你正在项目频道里总结一组成员多选出来的聊天记录。

        要求：
        - 只做讨论总结，不要修改代码，不要启动开发流程，不要发布 APK。
        - 用中文直接输出总结。
        - 不要重复粘贴原始聊天记录，频道里上一条消息已经展示了这些记录。

        输出结构：
        1. 核心结论
        2. 已决定事项
        3. 待确认问题
        4. 下一步行动

        选中的讨论：
        $transcript
    """.trimIndent()
}

private fun selectedDiscussionPersonalPrompt(transcript: String): String {
    return """
        请总结下面我多选的聊天讨论，直接给出中文结论。

        输出结构：
        1. 核心结论
        2. 已决定事项
        3. 待确认问题
        4. 下一步行动

        选中的讨论：
        $transcript
    """.trimIndent()
}

private fun ChatMessage.selectionSpeakerLabel(): String {
    return when (role) {
        "user" -> "我"
        "friend" -> senderLabel?.takeIf { it.isNotBlank() } ?: "对方"
        "ai", "ai-intent", "ai-complete" -> "AI"
        "ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-stopped" -> "开发进度"
        "error" -> "错误"
        else -> role
    }
}

private fun ChatMessage.selectionContent(): String {
    val attachmentText = attachments
        ?.takeIf { it.isNotEmpty() }
        ?.let { " [附件 ${it.size} 个]" }
        .orEmpty()
    val text = content.replace(Regex("\\s+"), " ").trim().ifBlank { "（无文字内容）" }
    return "${summarize(text, MAX_SELECTED_MESSAGE_CHARS)}$attachmentText"
}

private const val MAX_SELECTED_MESSAGE_CHARS = 1200
private const val MAX_SELECTED_DISCUSSION_CHARS = 7000
