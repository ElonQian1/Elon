package com.elon.app

/**
 * Builds the user-facing interaction layer for Codex-style conversations.
 *
 * The phone UI should show a clear human-readable story first, while keeping
 * command/file/build details in expandable evidence under the assistant bubble.
 */
object CodexInteractionPresentation {
    fun intentMessage(
        visibleText: String,
        outgoingText: String,
        isDevelopment: Boolean,
        executionMode: ProjectRequestExecutionMode = ProjectRequestExecutionMode.Execute,
        hasAttachments: Boolean
    ): ChatMessage {
        val summary = summarizeIntent(visibleText.ifBlank { outgoingText })
        val isPlanMode = executionMode.isPlan
        val routeLabel = when {
            isPlanMode -> "先规划"
            isDevelopment -> "开发任务"
            else -> "普通对话"
        }
        val nextAction = if (isPlanMode) {
            "我会先生成计划，不改代码；你确认后再开始实现。"
        } else if (isDevelopment) {
            "我会按「确认意图 → 定位代码 → 修改验证 → 交付结果」推进。"
        } else {
            "我会直接用对话方式回复，不启动项目代码修改。"
        }
        val attachmentLine = if (hasAttachments) {
            "\n我也会把你附上的文件或图片作为本轮上下文。"
        } else {
            ""
        }

        val content = if (isPlanMode) {
            "我理解你想先做计划：$summary\n$nextAction$attachmentLine"
        } else if (isDevelopment) {
            "我理解你是想让我进入开发流程：$summary\n$nextAction$attachmentLine"
        } else {
            "我理解你是在普通交流：$summary\n$nextAction$attachmentLine"
        }

        val evidenceItems = mutableListOf(
            "理解：$summary",
            "判断：$routeLabel",
            "行动：$nextAction"
        )
        if (hasAttachments) {
            evidenceItems.add("上下文：本轮包含附件，后台会先上传后再交给助手处理")
        }
        if (isDevelopment) {
            evidenceItems.add("证据：后续命令、文件、构建和结果会继续折叠在回复下方")
        }
        val evidence = evidenceItems.joinToString("\n") { "· $it" }

        return ChatMessage(
            role = "ai-intent",
            content = content,
            evidenceTitle = if (isPlanMode) {
                "理解意图 · 先规划"
            } else if (isDevelopment) {
                "理解意图 · 进入开发流程"
            } else {
                "理解意图 · 直接回复"
            },
            evidenceDetails = evidence,
            processLayer = true
        )
    }

    private fun summarizeIntent(text: String, maxLength: Int = 58): String {
        val clean = text
            .replace(Regex("\\s+"), " ")
            .trim()
            .ifBlank { "处理你刚刚这条消息" }
        if (clean.length <= maxLength) return clean
        return clean.take(maxLength - 1).trimEnd() + "…"
    }
}
