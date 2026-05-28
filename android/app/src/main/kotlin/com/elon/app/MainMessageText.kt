package com.elon.app

import com.elon.app.mcp.*
import org.json.JSONObject
import java.util.Locale

internal fun shouldCleanFinalAsDevelopment(content: String, wasDevelopment: Boolean, apkUrl: String?): Boolean {
    if (wasDevelopment || apkUrl != null) return true
    val lower = content.lowercase(Locale.CHINA)
    val strongSignals = listOf(
        "/root/workspaces/",
        "/opt/elon/data/projects/",
        "/root/Elon/",
        "build/android/",
        "src/main/",
        "androidmanifest",
        "mainactivity.",
        ".java:",
        ".kt:",
        ".xml:",
        "gradle",
        "assemble",
        "apksigner",
        "aapt dump",
        "已处理：",
        "改动：",
        "验证情况：",
        "apk 已生成"
    )
    return strongSignals.any { lower.contains(it) }
}

internal fun cleanFinalReplyForUser(content: String, wasDevelopment: Boolean, apkUrl: String?): String {
    if (!wasDevelopment) return content.trim()

    val cleanedLines = stripServerProjectPaths(content)
        .replace(Regex("\\[[^\\]]+\\.apk]\\([^)]*\\)"), "APK 已生成")
        .lineSequence()
        .map { sanitizeFinalReplyLine(it.trimEnd()) }
        .filterNot { line ->
            val lower = line.lowercase(Locale.CHINA)
            containsServerProjectPath(line) ||
                line.contains("build/android/") ||
                isLeakedPlatformPromptLine(line) ||
                line.startsWith("用户可见：") ||
                line.startsWith("用户可见:") ||
                lower.contains("apksigner") ||
                lower.contains("aapt dump") ||
                lower.contains("sha256") ||
                lower.startsWith("下载链接：") ||
                lower.startsWith("验证结果：")
        }
        .joinToString("\n")
        .replace(Regex("\n{3,}"), "\n\n")
        .trim()

    return if (apkUrl != null && cleanedLines.length > 520) {
        cleanedLines
            .lineSequence()
            .filter { line ->
                val trimmed = line.trim()
                trimmed.isNotBlank() &&
                    !trimmed.startsWith("- `") &&
                    !trimmed.startsWith("已检查：")
            }
            .take(6)
            .joinToString("\n")
            .trim()
            .ifBlank { "已完成并生成 APK。你可以先下载安装测试。" }
    } else {
        cleanedLines
    }
}

internal fun containsServerProjectPath(content: String): Boolean {
    return content.contains("/root/workspaces/") ||
        content.contains("/opt/elon/data/projects/") ||
        content.contains("/root/Elon/")
}

internal fun stripServerProjectPaths(content: String): String {
    val pathPrefix = "(?:/root/workspaces|/opt/elon/data/projects|/root/Elon)"
    return content
        .replace(Regex("\\[([^\\]]+)]\\s*\\($pathPrefix/[^)]*\\)"), "$1")
        .replace(Regex("\\s*\\($pathPrefix/[^)]*\\)"), "")
        .replace(Regex("$pathPrefix/\\S+"), "项目文件")
}

internal fun sanitizeFinalReplyLine(line: String): String {
    return line
        .replace("已处理：", "已完成：")
        .replace(Regex("在\\s+[^\\s，。；：]+\\.(kt|java|xml)(:\\d+)?\\s*的"), "")
        .replace(Regex("`([^`]+)`"), "$1")
        .trimEnd()
}

internal fun friendlyErrorMessage(raw: String, code: String? = null, retryable: Boolean? = null): String {
    val nestedMessage = nestedApiErrorMessage(raw)
    val source = listOf(raw, nestedMessage).joinToString(" ").lowercase(Locale.CHINA)
    val normalizedCode = code.orEmpty().lowercase(Locale.US)
    return when {
        isStructuredAiErrorCode(normalizedCode) && raw.isNotBlank() ->
            raw
        isTransientAiServiceConnectionError(source) ->
            transientAiServiceConnectionMessage(retryable)
        source.contains("free_quota_exhausted") ||
            source.contains("payment required") ||
            source.contains("endpoint is inactive") ->
            "当前选择的 AI 模型额度已用尽或接口不可用。请点右下角模型按钮切换可用模型，或联系管理员补充额度后重试。"
        source.contains("unauthorized") ||
            source.contains("invalid api key") ||
            source.contains("api key") && source.contains("invalid") ->
            "当前 AI 模型密钥无效或权限不足。请在 AI 设置里检查密钥，或切换到可用模型。"
        source.contains("rate limit") ||
            source.contains("too many requests") ||
            source.contains("429") ->
            "当前 AI 模型请求过于频繁。请稍后重试，或切换到其他可用模型。"
        source.contains("timeout") || source.contains("超时") ->
            "AI 请求超时了。请检查网络或稍后重试。"
        source.contains("connection") || source.contains("network") ->
            "连接 AI 服务失败。请检查网络、代理地址或稍后重试。"
        nestedMessage.isNotBlank() ->
            summarize(nestedMessage, 90)
        raw.isBlank() ->
            "AI 服务暂时不可用，请稍后重试。"
        else ->
            summarize(raw.replace(Regex("\\{.*"), "").trim().ifBlank { raw }, 90)
    }
}

private fun isStructuredAiErrorCode(code: String): Boolean {
    return code == "ai_service_busy" ||
        code == "ai_provider_connection_unstable" ||
        code == "ai_service_timeout" ||
        code == "ai_rate_limited" ||
        code == "ai_quota_unavailable" ||
        code == "ai_auth_config_error" ||
        code == "project_workspace_error"
}

private fun transientAiServiceConnectionMessage(retryable: Boolean?): String {
    return if (retryable == false) {
        "服务器 AI 通道刚才短暂断开，本轮没有完成。手机 WebSocket 临时断开会自动重连并同步进度；如果看到这条红色提示，说明服务端这轮已结束，请稍后重新发送。"
    } else {
        "服务器 AI 通道刚才拥堵或短暂断开，系统会先自动重试。手机 WebSocket 临时断开会自动重连并同步进度；如果连续重试后仍看到这条提示，说明本轮已暂停，稍后重新发送即可继续。"
    }
}

private fun isTransientAiServiceConnectionError(source: String): Boolean {
    return source.contains("codex cli network unhealthy") ||
        source.contains("responses websocket failed") ||
        source.contains("required provider endpoints are unreachable") ||
        source.contains("stream disconnected before completion") ||
        source.contains("reachability") && source.contains("unreachable")
}

internal fun nestedApiErrorMessage(raw: String): String {
    val jsonStart = raw.indexOf('{')
    if (jsonStart < 0) return ""
    return runCatching {
        val root = JSONObject(raw.substring(jsonStart))
        val error = root.optJSONObject("error")
        error?.optString("message").orEmpty().ifBlank {
            root.optString("message").orEmpty()
        }
    }.getOrDefault("")
}

internal fun evidenceTitle(entries: List<EvidenceEntry>): String {
    val counts = entries.groupingBy { it.kind }.eachCount()
    val parts = mutableListOf<String>()
    counts["command"]?.let { parts.add("已运行 ${it} 条命令") }
    counts["file"]?.let { parts.add("已查看 ${it} 个文件") }
    counts["edit"]?.let { parts.add("已编辑 ${it} 次") }
    counts["build"]?.let { parts.add("构建记录 ${it} 条") }
    counts["cli"]?.let { parts.add("CLI 输出 ${it} 条") }
    counts["env"]?.let { parts.add("环境提示 ${it} 条") }
    counts["connection"]?.let { parts.add("连接事件 ${it} 条") }
    counts["result"]?.let { parts.add("结果 ${it} 条") }
    counts["progress"]?.let { parts.add("进度 ${it} 条") }
    return parts.take(3).joinToString(" · ").ifBlank { "已收起 ${entries.size} 条后台记录" }
}

internal fun evidenceDetails(entries: List<EvidenceEntry>): String {
    return entries.takeLast(24).joinToString("\n") {
        "· ${evidenceKindLabel(it.kind)}：${it.text}"
    }
}

internal fun evidenceKindLabel(kind: String): String {
    return when (kind) {
        "command" -> "命令"
        "file" -> "文件"
        "edit" -> "编辑"
        "build" -> "构建"
        "cli" -> "CLI"
        "env" -> "环境"
        "connection" -> "连接"
        "result" -> "结果"
        else -> "进度"
    }
}

internal fun sanitizeEvidenceDetail(detail: String): String {
    val cleaned = stripServerProjectPaths(detail)
        .replace("用户可见：", "")
        .replace("用户可见:", "")
        .replace(Regex("\\s+"), " ")
        .trim()

    if (cleaned.isBlank()) return ""
    if (isLeakedPlatformPromptMessage(cleaned) || isTechnicalLeakMessage(cleaned)) return ""
    val lower = cleaned.lowercase(Locale.CHINA)
    val noisy = listOf(
        "tokens used",
        "feedback_tags",
        "codex_analytics",
        "original token count",
        "reading additional input",
        "openai codex v",
        "session id:",
        "auth_header"
    )
    if (noisy.any { lower.contains(it) }) return ""
    return cleaned
}

internal fun evidenceKindForCliCategory(category: String): String {
    return when (category) {
        "编译打包" -> "build"
        "执行命令" -> "command"
        "环境提示" -> "env"
        "模型回复" -> "cli"
        else -> "cli"
    }
}

internal fun toolEvidenceKind(tool: String): String {
    return when (tool) {
        "read_file", "list_dir" -> "file"
        "write_file", "init_project" -> "edit"
        "build_project" -> "build"
        "run_shell", "shell", "git_commit" -> "command"
        "file_change" -> "edit"
        else -> "progress"
    }
}

internal fun isCliOutputProgress(content: String): Boolean {
    return content.contains("CLI 输出(") ||
        content.contains("CLI 输出(stdout)") ||
        content.contains("CLI 输出(stderr)")
}

internal fun cleanCliOutputLine(content: String): String {
    val rawLine = content
        .lineSequence()
        .firstOrNull { it.contains("CLI 输出(") }
        ?: content
    return rawLine
        .substringAfter("):", rawLine)
        .replace("（输出过长，已截断）", "")
        .let(::stripServerProjectPaths)
        .trim()
}

internal fun cliOutputCategory(line: String): String {
    val lower = line.lowercase(Locale.CHINA)
    return when {
        lower.contains("gradle") || lower.contains("assemble") || line.contains("APK") -> "编译打包"
        lower.contains("/bin/bash") || lower == "exec" || lower.contains("succeeded in") ||
            lower.contains("process exited") || lower.contains("wall time") -> "执行命令"
        lower.contains("warn") || lower.contains("error") || lower.contains("failed") ||
            line.contains("未检测") || line.contains("失败") -> "环境提示"
        looksLikeAssistantCliLine(line) -> "模型回复"
        else -> "后台输出"
    }
}

internal fun looksLikeAssistantCliLine(line: String): Boolean {
    if (line.length !in 8..220) return false
    if (line.any { it in '\u4e00'..'\u9fff' }) {
        val lower = line.lowercase(Locale.CHINA)
        return !lower.contains("mcp_server") &&
            !lower.contains("event.timestamp") &&
            !lower.contains("feedback_tags")
    }
    return false
}

internal fun shouldKeepCliSample(line: String): Boolean {
    if (line.isBlank()) return false
    val lower = line.lowercase(Locale.CHINA)
    if (line in setOf("codex", "exec", "user", "tokens used", "Output:")) return false
    val noisy = listOf(
        "feedback_tags",
        "model_client.",
        "responses_websocket",
        "mcp_server=",
        "event.timestamp=",
        "original token count",
        "reading additional input",
        "openai codex v",
        "session id:",
        "auth_header",
        "codex_analytics",
        "plugins/featured",
        "plugins/installed",
        "</html>"
    )
    return noisy.none { lower.contains(it) }
}

internal fun compactCliTranscriptMessages(messages: MutableList<ChatMessage>) {
    if (messages.none { it.role == "ai-progress" && isCliOutputProgress(it.content) }) return

    val compacted = mutableListOf<ChatMessage>()
    var count = 0
    val categories = linkedMapOf<String, Int>()

    fun flushCliLog() {
        if (count == 0) return
        compacted.add(ChatMessage("ai-cli-log", genericFoldedCliLogSummary(categories)))
        count = 0
        categories.clear()
    }

    for (message in messages) {
        if (message.role == "ai-progress" && isCliOutputProgress(message.content)) {
            count += 1
            val line = cleanCliOutputLine(message.content)
            val category = cliOutputCategory(line)
            categories[category] = (categories[category] ?: 0) + 1
        } else {
            flushCliLog()
            compacted.add(message)
        }
    }
    flushCliLog()

    messages.clear()
    messages.addAll(compacted)
}

internal fun sanitizeExistingCliLogMessages(messages: MutableList<ChatMessage>) {
    for (index in messages.indices) {
        val message = messages[index]
        if (message.role == "ai-cli-log") {
            messages[index] = ChatMessage("ai-cli-log", genericFoldedCliLogSummary())
        } else if (message.role == "ai-progress") {
            messages[index] = ChatMessage("ai-progress", sanitizeStoredProgressMessage(message.content))
        }
    }
}

internal fun sanitizeExistingUserVisibleMessages(messages: MutableList<ChatMessage>) {
    val roles = setOf("ai", "ai-intent")
    messages.indices.forEach { index ->
        val message = messages[index]
        if (message.role !in roles) return@forEach
        if (!shouldCleanFinalAsDevelopment(message.content, wasDevelopment = false, apkUrl = null)) return@forEach
        val cleaned = cleanFinalReplyForUser(message.content, wasDevelopment = true, apkUrl = null)
        messages[index] = if (cleaned.isBlank()) {
            message.copy(content = "本轮开发任务已完成。")
        } else {
            message.copy(content = cleaned)
        }
    }
}

internal fun isRoutineWorkflowMessage(content: String): Boolean {
    val trimmed = content.trim()
    return trimmed == "正在思考" ||
        trimmed == "正在按这个计划推进。" ||
        trimmed.startsWith("启动助手：") ||
        trimmed.startsWith("准备项目：") ||
        trimmed.startsWith("处理中：") ||
        trimmed.startsWith("暂时没有收到服务器进度") ||
        trimmed.startsWith("连接恢复中") ||
        trimmed.startsWith("连接已恢复") ||
        trimmed.startsWith("检查结果：开发处理已结束") ||
        trimmed.contains("开发助手已启动，正在处理你的需求") ||
        trimmed.contains("项目环境已准备好，正在进入开发流程") ||
        trimmed.contains("开发助手仍在运行")
}

internal fun isLeakedPlatformPromptMessage(content: String): Boolean {
    return content
        .lineSequence()
        .map { it.trim() }
        .any { isLeakedPlatformPromptLine(it) }
}

internal fun isTechnicalLeakMessage(content: String): Boolean {
    val lower = content.lowercase(Locale.CHINA)
    return lower.contains("rmcp::") ||
        lower.contains("worker quit with fatal") ||
        lower.contains("http request failed") ||
        lower.contains("client error:") ||
        lower.contains("event.timestamp=") ||
        lower.contains("mcp_server=")
}

internal fun isLeakedPlatformPromptLine(line: String): Boolean {
    return line.contains("你是「一龙」平台服务器上的本地 AI CLI 编程助手") ||
        line.startsWith("当前 CLI") ||
        line.startsWith("当前工作目录") ||
        line.contains("用户隔离工作区") ||
        line.contains("不要使用固定模板") ||
        line.contains("不要提“CLI/后台/工作区”") ||
        line.startsWith("请直接处理用户请求") ||
        line.startsWith("用户请求：")
}

internal fun sanitizeStoredProgressMessage(content: String): String {
    val cleaned = content
        .replace("启动 CLI", "启动助手")
        .replace("准备工作区", "准备项目")
    return when {
        content.contains("CLI 工作区已准备") ->
            cleaned.replaceAfter("\n", "项目环境已准备好，正在进入开发流程。")
        content.contains("正在启动本地 CLI") ->
            cleaned.replaceAfter("\n", "开发助手已启动，正在处理你的需求。")
        content.contains("CLI 输出") ->
            genericFoldedCliLogSummary()
        else -> cleaned
    }
}

internal fun genericFoldedCliLogSummary(categories: Map<String, Int> = emptyMap()): String {
    val mainWork = categories.entries.maxByOrNull { it.value }?.key
    val friendly = when (mainWork) {
        "编译打包" -> "正在编译 APK"
        "执行命令" -> "正在检查项目文件"
        "环境提示" -> "环境提示已归类"
        "模型回复" -> "正在整理下一步"
        else -> "后台正在处理项目"
    }
    val count = categories.values.sum()
    val suffix = if (count > 0) "（${count} 条）" else ""
    return "后台开发日志已收起$suffix · $friendly"
}

internal fun extractUserVisibleCliMessage(line: String): String? {
    val trimmed = line.trim()
        .removePrefix("AI 回复片段：")
        .removePrefix("AI 回复片段:")
        .trim()
    val marker = when {
        trimmed.startsWith("用户可见：") -> "用户可见："
        trimmed.startsWith("用户可见:") -> "用户可见:"
        else -> return null
    }
    return trimmed.substringAfter(marker).trim().takeIf { it.isNotBlank() }
}

internal fun userSafeCliLine(line: String): String {
    return summarize(
        line
            .replace(Regex("\\s+"), " ")
            .trim(),
        120
    )
}

internal fun isCliProjectEvent(line: String): Boolean {
    return line.contains("CLI 输出(") ||
        line.contains("CLI 工作区") ||
        line.contains("正在启动本地 CLI") ||
        line.contains("CLI 日志已折叠") ||
        line.contains("CLI 日志已归类") ||
        line.contains("CLI 运行日志已折叠")
}

internal fun summarize(text: String, maxLength: Int): String {
    val normalized = text.replace('\n', ' ').trim()
    if (normalized.length <= maxLength) return normalized
    return normalized.take(maxLength - 1) + "…"
}
