package com.elon.app

import java.util.Locale

/**
 * Turns real workflow signals into Codex-style user-facing narration.
 *
 * The signal decides whether a bubble is useful; command output and raw logs stay
 * in evidence unless they explain a meaningful phase change.
 */
object CodexProgressNarrative {
    data class Narrative(
        val key: String,
        val message: ChatMessage
    )

    @Suppress("UNUSED_PARAMETER")
    fun fromTaskEvent(event: String, content: String): Narrative? {
        return null
    }

    fun fromWorkflowProgress(content: String): Narrative? {
        val clean = content.cleanSignalText()
        extractUserVisibleCliMessage(clean)?.let { userVisible ->
            if (shouldExposeAssistantLine(userVisible)) {
                return narrative(
                    key = "assistant:${userVisible.take(72)}",
                    content = userVisible
                )
            }
        }
        if (isSelfRecoveringWorkflowProgress(clean)) return null

        return when {
            clean.contains("失败") || clean.contains("错误") || clean.contains("不可用") -> narrative(
                key = "workflow_issue:${clean.take(48)}",
                content = "当前流程遇到问题，我会先把原因定位清楚，再决定是继续修复、重试，还是需要你确认取舍。"
            )
            else -> null
        }
    }

    @Suppress("UNUSED_PARAMETER")
    fun fromToolCall(tool: String): Narrative? {
        return null
    }

    fun fromCliOutput(category: String, line: String): Narrative? {
        val clean = userSafeCliLine(line)
        val lower = clean.lowercase(Locale.CHINA)
        if (isSelfRecoveringWorkflowProgress(clean)) return null
        if (category == "模型回复") {
            val userVisible = extractUserVisibleCliMessage(clean) ?: return null
            if (!shouldExposeAssistantLine(userVisible)) return null
            return narrative(
                key = "assistant:${userVisible.take(72)}",
                content = userVisible
            )
        }

        return when {
            category == "编译打包" && lower.contains("failed") -> narrative(
                key = "cli_build_failed:${clean.take(40)}",
                content = "构建没有通过。我会根据失败原因继续修复，而不是直接把失败结果交给你。"
            )
            category == "环境提示" && (lower.contains("failed") || lower.contains("error") || clean.contains("失败")) -> narrative(
                key = "cli_environment_issue:${clean.take(40)}",
                content = "环境或执行过程出现了会影响任务的阻塞。我会先确认它是不是本次失败的原因。"
            )
            else -> null
        }
    }

    private fun narrative(
        key: String,
        content: String
    ): Narrative {
        return Narrative(
            key = key,
            message = ChatMessage(
                role = "ai-intent",
                content = content
            )
        )
    }

    private fun shouldExposeAssistantLine(line: String): Boolean {
        if (!shouldKeepCliSample(line)) return false
        if (line.length !in 6..280) return false
        val lower = line.lowercase(Locale.CHINA)
        val finalLike = listOf(
            "apk 已生成",
            "下载链接",
            "本轮开发任务已完成",
            "最终回复"
        )
        if (finalLike.any { lower.contains(it) }) return false
        val technical = listOf(
            "```",
            "/root/",
            "/home/",
            "build.gradle",
            "androidmanifest",
            ".kt",
            ".xml",
            "tokens used",
            "不要使用固定模板",
            "不要提"
        )
        return technical.none { lower.contains(it) }
    }

    private fun String.cleanSignalText(): String {
        return replace(Regex("\\s+"), " ")
            .trim()
            .let(::stripServerProjectPaths)
    }
}
