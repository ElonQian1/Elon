package com.elon.app

import com.google.gson.JsonArray
import com.google.gson.JsonObject

/**
 * 把 Codex CLI 的 tool_call 描述成手机用户能看懂的一行动作卡片，
 * 例如：📖 读取 `MainActivity.kt`、⚙️ 执行 `cargo check`。
 *
 * MainActivity.appendMessage 在 dispatch tool_call 时调用本函数，
 * 把返回值放进 ChatMessage("ai-action", ...)，呈现"AI 正在做什么"的真实交互感。
 */
internal fun describeToolAction(tool: String, args: JsonObject?): String {
    val icon = when (tool) {
        "read_file", "list_dir", "list_directory", "view_file" -> "\uD83D\uDCD6"
        "write_file", "edit_file", "file_change", "patch", "apply_patch" -> "\u270F\uFE0F"
        "shell", "run_shell", "run_command", "command_execution" -> "\u2699\uFE0F"
        "build_project", "compile" -> "\uD83D\uDD28"
        "git_commit", "git", "commit" -> "\uD83D\uDCBE"
        "search", "grep", "find" -> "\uD83D\uDD0D"
        "init_project", "create_project" -> "\uD83C\uDD95"
        else -> "\uD83D\uDD27"
    }
    val verb = when (tool) {
        "read_file", "view_file" -> "读取"
        "list_dir", "list_directory" -> "查看目录"
        "write_file" -> "写入"
        "edit_file" -> "编辑"
        "file_change", "patch", "apply_patch" -> "修改"
        "shell", "run_shell", "run_command", "command_execution" -> "执行"
        "build_project", "compile" -> "编译"
        "git_commit", "commit" -> "提交"
        "git" -> "Git"
        "search", "grep" -> "搜索"
        "find" -> "查找"
        "init_project", "create_project" -> "初始化项目"
        else -> tool
    }
    val detail = toolActionDetail(tool, args)
    return if (detail.isNullOrBlank()) "$icon $verb" else "$icon $verb $detail"
}

private fun toolActionDetail(tool: String, args: JsonObject?): String? {
    if (args == null) return null
    return when (tool) {
        "shell", "run_shell", "run_command", "command_execution" -> {
            val cmd = jsonStringSafe(args, "command")
                ?: jsonStringSafe(args, "cmd")
                ?: jsonStringSafe(args, "input")
            cmd?.let { wrapInline(truncateOneLine(it, 80)) }
        }
        "file_change", "patch", "apply_patch" -> {
            jsonStringSafe(args, "path")
                ?.let(::wrapInline)
                ?: firstPathFromChanges(args.get("changes") as? JsonArray)
                    ?.let(::wrapInline)
        }
        "git_commit", "commit" -> {
            val msg = jsonStringSafe(args, "message")
            msg?.let { wrapInline(truncateOneLine(it, 60)) }
        }
        else -> jsonStringSafe(args, "path")?.let(::wrapInline)
    }
}

private fun firstPathFromChanges(changes: JsonArray?): String? {
    if (changes == null || changes.size() == 0) return null
    val first = changes[0]?.takeIf { it.isJsonObject }?.asJsonObject ?: return null
    val path = jsonStringSafe(first, "path") ?: return null
    return if (changes.size() > 1) "$path 等 ${changes.size()} 处" else path
}

private fun jsonStringSafe(obj: JsonObject, key: String): String? {
    val el = obj.get(key) ?: return null
    if (el.isJsonNull) return null
    if (!el.isJsonPrimitive) return null
    return runCatching { el.asString }.getOrNull()?.takeIf { it.isNotBlank() }
}

private fun wrapInline(text: String): String = "`${text.trim()}`"

private fun truncateOneLine(text: String, max: Int): String {
    val collapsed = text.lineSequence().firstOrNull()?.trim().orEmpty()
    return if (collapsed.length <= max) collapsed else collapsed.take(max) + "…"
}

/**
 * 工具完成后把对应的 ai-action 卡片标成 ✓ 已完成。
 * 不重复加 ✓，也不丢失原 icon。
 */
internal fun markToolActionDone(original: String): String {
    val trimmed = original.trimStart()
    return if (trimmed.startsWith("\u2713 ")) original else "\u2713 $original"
}
