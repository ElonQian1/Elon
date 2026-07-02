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

    fun fromTaskEvent(event: String, content: String): Narrative? {
        val cleanEvent = event.trim().lowercase(Locale.US)
        val cleanContent = content.cleanSignalText()
        return when (cleanEvent) {
            "started" -> narrative(
                key = "task_started",
                content = "我已经开始执行这轮任务，后续检查、命令、文件修改和验证结果会折叠记录在同一轮会话里。"
            )
            "runtime_note_received" -> cleanContent
                .takeIf { it.isNotBlank() && !isSelfRecoveringWorkflowProgress(it) }
                ?.let {
                    narrative(
                        key = "runtime_note:${it.take(48)}",
                        content = userFacingProgress(it)
                    )
                }
            else -> null
        }
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

    fun fromToolCall(tool: String): Narrative? {
        return when (tool.trim().lowercase(Locale.US)) {
            "read_file", "list_dir" -> narrative(
                key = "tool_read_project",
                content = "我正在读取项目文件和规则，先确认现有结构再决定怎么改。"
            )
            "run_shell", "shell" -> narrative(
                key = "tool_run_command",
                content = "我开始在项目工作区执行命令检查现状，命令细节会折叠在这条回复下面。"
            )
            "write_file", "file_change", "init_project" -> narrative(
                key = "tool_change_files",
                content = "我已经进入修改阶段，文件变更会继续折叠记录，完成后再统一验证。"
            )
            "build_project" -> narrative(
                key = "tool_build_project",
                content = "我正在运行构建或测试，等结果出来后再判断是否继续修。"
            )
            "git_commit" -> narrative(
                key = "tool_git_commit",
                content = "我正在保存这次改动，提交和发布结果会作为最终交付的一部分展示。"
            )
            else -> null
        }
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
