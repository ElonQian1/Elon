package com.elon.app

import org.json.JSONObject

internal data class ForkedConversation(
    val target: SendTarget,
    val outgoingText: String
)

internal data class ForkContextSnapshot(
    val sourceTitle: String,
    val promptContext: String,
    val includedMessageCount: Int,
    val hasRunningTask: Boolean
)

internal fun buildForkContextSnapshot(
    source: AppConversation,
    activeTask: ConversationTaskState?
): ForkContextSnapshot {
    val runningTaskText = activeTask?.payload?.let(::extractTaskMessage)
        ?.takeIf { it.isNotBlank() }
    val recentMessages = source.messages
        .filter(::isUsefulForkContextMessage)
        .takeLast(MAX_FORK_CONTEXT_MESSAGES)
        .map { message ->
            "${forkRoleLabel(message.role)}：${summarize(message.content, MAX_FORK_CONTEXT_MESSAGE_CHARS)}"
        }
    val context = buildString {
        if (!source.codexThreadUri.isNullOrBlank()) {
            append("来源 Codex 线程：")
            append(source.codexThreadUri)
            append('\n')
        }
        if (!runningTaskText.isNullOrBlank()) {
            append("来源会话当前仍有任务运行；原任务请求：")
            append(summarize(runningTaskText, MAX_FORK_CONTEXT_TASK_CHARS))
            append('\n')
        }
        if (recentMessages.isNotEmpty()) {
            append("最近上下文：\n")
            append(recentMessages.joinToString("\n"))
        }
    }.trim().let { summarize(it, MAX_FORK_CONTEXT_TOTAL_CHARS) }
    return ForkContextSnapshot(
        sourceTitle = source.title,
        promptContext = context,
        includedMessageCount = recentMessages.size,
        hasRunningTask = activeTask != null
    )
}

internal fun buildForkOutgoingText(
    userOutgoingText: String,
    snapshot: ForkContextSnapshot
): String {
    val context = snapshot.promptContext.takeIf { it.isNotBlank() } ?: return userOutgoingText
    return buildString {
        append("这是从 APK 会话「")
        append(snapshot.sourceTitle)
        append("」分叉出来的新会话。")
        append("\n原会话会继续运行；请只在本新会话中基于下面上下文探索另一种方案，不要取消或改写原会话任务。")
        append("\n\n来源会话上下文：\n")
        append(context)
        append("\n\n分叉后的用户请求：\n")
        append(userOutgoingText.trim())
    }
}

internal fun forkProgressMessage(snapshot: ForkContextSnapshot): String {
    val contextText = if (snapshot.includedMessageCount > 0) {
        "已带入最近 ${snapshot.includedMessageCount} 条上下文"
    } else {
        "来源会话暂无可带入上下文"
    }
    val taskText = if (snapshot.hasRunningTask) "，原任务继续运行" else ""
    return "已从「${snapshot.sourceTitle}」分叉，$contextText$taskText。"
}

private fun extractTaskMessage(payload: String): String? {
    return runCatching {
        JSONObject(payload).optString("message").takeIf { it.isNotBlank() }
    }.getOrNull()
}

private fun isUsefulForkContextMessage(message: ChatMessage): Boolean {
    val content = message.content.trim()
    if (content.isBlank()) return false
    if (content.startsWith("你可以直接描述想开发的 App 功能")) return false
    if (content.startsWith("你可以直接告诉我想给 APK 加什么功能")) return false
    return message.role in setOf(
        "user",
        "ai",
        "ai-intent",
        "ai-progress",
        "ai-complete",
        "ai-stopped",
        "error"
    )
}

private fun forkRoleLabel(role: String): String {
    return when (role) {
        "user" -> "用户"
        "ai-intent" -> "意图"
        "ai-progress" -> "进度"
        "ai-complete" -> "完成"
        "ai-stopped" -> "中止"
        "error" -> "错误"
        else -> "助手"
    }
}

private const val MAX_FORK_CONTEXT_MESSAGES = 8
private const val MAX_FORK_CONTEXT_MESSAGE_CHARS = 220
private const val MAX_FORK_CONTEXT_TASK_CHARS = 360
private const val MAX_FORK_CONTEXT_TOTAL_CHARS = 1800
