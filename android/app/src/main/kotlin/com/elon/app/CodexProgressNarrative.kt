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
        val clean = content.cleanSignalText()
        return when (event) {
            "accepted" -> narrative(
                key = "task_accepted",
                title = "任务受理",
                content = if (clean.isBlank()) {
                    "这条需求已经进入开发队列。我会等到本会话拿到执行权后，再开始读取项目和改代码。"
                } else {
                    "这条需求已经进入开发队列：$clean\n我会等到本会话拿到执行权后继续推进。"
                },
                trigger = clean.ifBlank { "任务已受理" },
                next = "等待执行权，然后进入项目检查。"
            )
            "started" -> narrative(
                key = "task_started",
                title = "开始执行",
                content = "轮到这次需求了。我现在会按项目规则进入开发流程，先看清楚现场，再决定改哪里。",
                trigger = clean.ifBlank { "任务开始执行" },
                next = "确认项目状态和相关文件。"
            )
            else -> null
        }
    }

    fun fromWorkflowProgress(content: String): Narrative? {
        val clean = content.cleanSignalText()
        val lower = clean.lowercase(Locale.CHINA)
        extractUserVisibleCliMessage(clean)?.let { userVisible ->
            if (shouldExposeAssistantLine(userVisible)) {
                return narrative(
                    key = "assistant:${userVisible.take(72)}",
                    title = "开发说明",
                    content = userVisible,
                    trigger = clean,
                    next = "继续按这个判断推进。"
                )
            }
        }

        return when {
            clean.contains("已识别为开发任务") -> narrative(
                key = "route_development",
                title = "确认意图",
                content = "我已经确认这不是普通聊天，而是需要进入开发流程。接下来会把项目规则、当前代码和发布要求一起纳入判断。",
                trigger = clean,
                next = "准备项目环境并启动开发助手。"
            )
            clean.contains("正在确认这是否需要进入开发流程") -> narrative(
                key = "route_checking",
                title = "判断路线",
                content = "我正在判断这条消息是普通对话，还是需要真正改项目代码。这个判断会决定后面是否启动开发流程。",
                trigger = clean,
                next = "确认后给出开发或对话路径。"
            )
            clean.contains("通用项目工作流") -> narrative(
                key = "workflow_rules",
                title = "流程展开",
                content = "我已经把这轮任务切到项目开发流程。后面会按「确认规则、定位代码、修改验证、交付结果」推进，中间关键判断会继续告诉你。",
                trigger = clean,
                next = "读取项目规则并准备独立开发环境。"
            )
            clean.startsWith("正在准备项目工作区") || lower.contains("worktree") -> narrative(
                key = "workspace_prepare",
                title = "准备环境",
                content = "我正在准备这次任务的独立开发环境。这样做是为了保护当前项目现场，避免把别的会话或未提交改动混进来。",
                trigger = clean,
                next = "环境就绪后开始检查相关代码。"
            )
            clean.contains("已轮到本会话任务") ||
                clean.contains("已获得本会话执行权") ||
                clean.contains("已获得项目执行权") -> narrative(
                    key = "execution_turn",
                    title = "拿到执行权",
                    content = "现在轮到这轮需求执行了。我会开始进入真实项目，边检查边推进，而不是只等最终结果。",
                    trigger = clean,
                    next = "启动开发助手并开始项目侦察。"
                )
            clean.contains("AI 助手正在处理") ||
                clean.startsWith("正在启动本地 CLI") -> narrative(
                    key = "assistant_running",
                    title = "开发助手启动",
                    content = "开发助手已经接手。我会把关键判断用这种气泡告诉你，把命令、文件和构建细节折叠到下方证据里。",
                    trigger = clean,
                    next = "定位代码并开始修改或验证。"
                )
            clean.startsWith("CLI 已结束") -> narrative(
                key = "assistant_finished",
                title = "检查结果",
                content = "开发处理已经结束。我正在检查结果是否完整，尤其是 APK 任务有没有生成可安装产物。",
                trigger = clean,
                next = "核对产物、版本和下载链接。"
            )
            clean.startsWith("未找到 APK") -> narrative(
                key = "apk_missing",
                title = "产物检查",
                content = "我暂时没有找到新的 APK 产物。接下来会判断是还没进入发布流程、构建失败，还是需要继续补齐打包步骤。",
                trigger = clean,
                next = "继续检查构建和发布状态。"
            )
            clean.contains("失败") || clean.contains("错误") || clean.contains("不可用") -> narrative(
                key = "workflow_issue:${clean.take(48)}",
                title = "遇到问题",
                content = "当前流程遇到问题，我会先把原因定位清楚，再决定是继续修复、重试，还是需要你确认取舍。",
                trigger = clean,
                next = "定位失败原因。"
            )
            else -> null
        }
    }

    fun fromToolCall(tool: String): Narrative? {
        return when (tool) {
            "shell", "run_shell" -> narrative(
                key = "tool_shell",
                title = "检查现场",
                content = "我开始用命令确认项目状态、文件结构或验证结果了。你看到的是阶段说明，具体命令会收在证据里。",
                trigger = "开始执行命令",
                next = "根据命令结果决定下一步。"
            )
            "read_file", "list_dir" -> narrative(
                key = "tool_read_files",
                title = "定位代码",
                content = "我正在查看相关文件，先确认功能应该落在哪个模块里，再动手改。",
                trigger = "开始读取项目文件",
                next = "找到职责边界后修改代码。"
            )
            "write_file", "file_change", "init_project" -> narrative(
                key = "tool_edit_files",
                title = "修改实现",
                content = "我已经进入代码修改阶段。后面会继续检查这些改动能不能编译、能不能交付给手机安装。",
                trigger = "开始修改文件",
                next = "完成修改后运行验证或构建。"
            )
            "build_project" -> narrative(
                key = "tool_build",
                title = "编译验证",
                content = "我正在进入构建验证。这里会检查代码是否真的跑得通，而不是只停在看起来改好了。",
                trigger = "开始构建项目",
                next = "根据构建结果修复或交付。"
            )
            "git_commit" -> narrative(
                key = "tool_commit",
                title = "保存版本",
                content = "代码和验证通过后，我会把这次改动保存成版本记录，方便追踪和发布。",
                trigger = "开始保存版本",
                next = "提交后进入发布或交付检查。"
            )
            else -> null
        }
    }

    fun fromCliOutput(category: String, line: String): Narrative? {
        val clean = userSafeCliLine(line)
        val lower = clean.lowercase(Locale.CHINA)
        if (category == "模型回复") {
            val userVisible = extractUserVisibleCliMessage(clean) ?: return null
            if (!shouldExposeAssistantLine(userVisible)) return null
            return narrative(
                key = "assistant:${userVisible.take(72)}",
                title = "开发说明",
                content = userVisible,
                trigger = clean,
                next = "继续按这个判断推进。"
            )
        }

        return when (category) {
            "执行命令" -> when {
                lower.contains("git") || lower.contains("rev-parse") || lower.contains("status") ->
                    narrative(
                        key = "cli_git_check",
                        title = "检查现场",
                        content = "我正在确认项目位置、分支和未提交改动。先把现场看清楚，后面才不会把无关改动混进去。",
                        trigger = clean,
                        next = "确认安全后继续定位代码。"
                    )
                else -> narrative(
                    key = "cli_command_check",
                    title = "执行检查",
                    content = "我正在运行必要检查，判断项目当前状态和下一步该怎么走。",
                    trigger = clean,
                    next = "根据检查结果继续开发。"
                )
            }
            "编译打包" -> narrative(
                key = "cli_build_check",
                title = "编译打包",
                content = "现在进入编译或 APK 打包检查。这个阶段会验证代码是否真的可安装、可运行。",
                trigger = clean,
                next = "构建失败就继续修，构建通过就检查产物。"
            )
            "环境提示" -> narrative(
                key = "cli_environment_issue:${clean.take(40)}",
                title = "环境检查",
                content = "环境里出现了需要注意的提示。我会先判断它是否影响本次开发或 APK 构建，再决定要不要修。",
                trigger = clean,
                next = "确认是否需要处理环境问题。"
            )
            else -> null
        }
    }

    private fun narrative(
        key: String,
        title: String,
        content: String,
        trigger: String,
        next: String
    ): Narrative {
        val evidence = listOf(
            "阶段：$title",
            "依据：${trigger.ifBlank { content }.cleanSignalText()}",
            "下一步：$next"
        ).joinToString("\n") { "· $it" }
        return Narrative(
            key = key,
            message = ChatMessage(
                role = "ai-intent",
                content = content,
                evidenceTitle = "阶段说明 · $title",
                evidenceDetails = evidence
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
