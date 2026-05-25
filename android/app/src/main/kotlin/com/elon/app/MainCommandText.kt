package com.elon.app

import java.util.Locale

internal fun quickLocalChatReply(text: String, hasPendingAttachments: Boolean): String? {
    if (hasPendingAttachments) return null
    if (looksLikeDevelopmentRequest(text) || looksLikeDirectImageRequest(text)) return null
    return when (text.trim().lowercase(Locale.CHINA)) {
        "你好", "你好呀", "在吗", "你在吗", "在不在", "hi", "hello" ->
            "你好，我在。你可以直接告诉我想改代码、查问题、构建 APK，或者先聊聊想法。"
        "谢谢", "谢谢你", "辛苦了" ->
            "不客气，我在这边。你继续说下一步想怎么改就行。"
        else -> null
    }
}

internal fun expandShortDevelopmentCommand(text: String, messages: List<ChatMessage>): String {
    val normalized = text.trim().lowercase(Locale.CHINA)
    return when {
        looksLikeResumeCommand(normalized) -> buildResumeDevelopmentCommand(
            originalText = text,
            lastRequest = lastActionableUserRequest(messages),
            latestFailure = latestFailureMessage(messages)
        )
        normalized in setOf("打包", "编译", "生成apk", "生成 apk", "打包apk", "打包 apk") ->
            "请编译当前项目并生成可以下载安装到手机的 APK 下载链接。"
        else -> text
    }
}

private fun buildResumeDevelopmentCommand(
    originalText: String,
    lastRequest: String?,
    latestFailure: String?
): String {
    return buildString {
        append("请继续完成上一次未完成的开发任务，不要只返回之前已经生成过的 APK。")
        if (!lastRequest.isNullOrBlank()) {
            append("\n\n上一条未完成的用户需求：")
            append(lastRequest)
        }
        if (!latestFailure.isNullOrBlank()) {
            append("\n\n最近一次中断或错误：")
            append(latestFailure)
        }
        append("\n\n当前用户补充：")
        append(originalText.trim())
        append("\n\n请结合当前项目文件、上一条用户需求和最近错误继续完成开发；只有确认该需求已经实现，并重新检查或构建后，才返回新的 APK 下载链接和当前进度。")
    }
}

private fun lastActionableUserRequest(messages: List<ChatMessage>): String? {
    return messages
        .asReversed()
        .firstOrNull { message ->
            message.role == "user" &&
                message.content.isNotBlank() &&
                !looksLikeResumeCommand(message.content.trim().lowercase(Locale.CHINA)) &&
                !looksLikeApkDeliveryRequest(message.content)
        }
        ?.content
        ?.trim()
}

private fun latestFailureMessage(messages: List<ChatMessage>): String? {
    return messages
        .asReversed()
        .firstOrNull { it.role == "error" || it.role == "ai-stopped" }
        ?.content
        ?.trim()
        ?.takeIf { it.isNotBlank() }
}
