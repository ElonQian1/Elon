package com.elon.app

import java.util.Locale

internal class MainWorkflowStageActions(
    private val currentStage: () -> String,
    private val updateStage: (String, String) -> Unit,
    private val addProjectEvent: (String) -> Unit,
    private val recordEvidence: (String, String) -> Unit
) {
    fun handleProgress(content: String, recordProgressEvidence: Boolean = true) {
        val lower = content.lowercase(Locale.CHINA)
        val facing = userFacingProgress(content)
        when {
            content.contains("进入队列") || content.contains("排队") ->
                updateStage("任务排队", facing)
            content.contains("通用项目工作流") ||
                content.contains("项目文档") ||
                content.contains("Git/权限") ||
                content.contains("项目自己的规则") ->
                updateStage("需求分析", facing)
            content.contains("未找到 APK") ||
                content.contains("未检测到 java") ||
                content.contains("未检测到 Android SDK") ->
                updateStage("需要处理", facing)
            content.contains("编译") ||
                content.contains("APK") ||
                content.contains("下载链接") ||
                lower.contains("gradle") ||
                lower.contains("assemble") ->
                updateStage("编译打包", facing)
            content.contains("CLI 输出") ||
                content.contains("写入") ||
                content.contains("读取") ||
                content.contains("修改") ||
                content.contains("工具") ->
                updateStage("开发实现", facing)
            content.contains("理解需求") ||
                content.contains("AI 代理") ||
                content.contains("CLI 工作区") ||
                content.contains("启动本地 CLI") ->
                updateStage("需求分析", facing)
            else ->
                updateStage("开发实现", facing)
        }
        // "AI 执行命令：..." 和 "命令执行完毕" 已由 tool_call/tool_result 记录，跳过避免重复
        val isRedundantCommandProgress = content.startsWith("AI 执行命令：") || content == "命令执行完毕"
        if (recordProgressEvidence && !content.startsWith("CLI 仍在运行") && !isRedundantCommandProgress) {
            recordEvidence("progress", userFacingProgress(content))
        }
        addProjectEvent("进度更新：${summarize(content, 30)}")
    }

    fun handleTaskEvent(event: String, taskId: String?, content: String) {
        val suffix = taskId?.takeIf { it.isNotBlank() }?.let { "（任务 $it）" }.orEmpty()
        when (event) {
            "accepted" -> {
                updateStage("任务排队", if (content.isBlank()) "请求已进入任务队列。" else content)
                addProjectEvent("任务已受理$suffix")
            }
            "started" -> {
                updateStage("开发实现", if (content.isBlank()) "任务开始执行。" else content)
                addProjectEvent("任务开始执行$suffix")
            }
            "cancel_requested" -> {
                updateStage("需要处理", if (content.isBlank()) "已请求取消任务。" else content)
                addProjectEvent("任务取消请求已发送$suffix")
            }
            "canceled" -> {
                updateStage("需要处理", if (content.isBlank()) "任务已取消。" else content)
                addProjectEvent("任务已取消$suffix")
            }
            else -> {
                if (content.isNotBlank()) {
                    addProjectEvent("任务事件：${summarize(content, 30)}")
                }
            }
        }
    }

    fun handleToolCall(tool: String) {
        // shell 命令的实际内容由后续 progress "AI 执行命令：..." 记录，这里跳过避免重复
        if (tool != "shell" && tool != "run_shell") {
            recordEvidence(toolEvidenceKind(tool), "开始：${toolLabel(tool)}")
        }
        when (tool) {
            "build_project" -> updateStage("编译打包", "正在编译项目并准备 APK。")
            "git_commit" -> updateStage("交付完成", "正在保存当前开发版本。")
            else -> updateStage("开发实现", "正在执行：${toolLabel(tool)}")
        }
        addProjectEvent("执行工具：${toolLabel(tool)}")
    }

    fun markToolResult(tool: String) {
        updateStage(currentStage(), "${toolLabel(tool)} 已完成，正在判断下一步。")
        addProjectEvent("工具完成：${toolLabel(tool)}")
    }
}
