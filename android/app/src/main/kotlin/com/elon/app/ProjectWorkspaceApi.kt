package com.elon.app

import android.content.Context
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject

internal data class ProjectWorkspaceHealth(
    val healthLabel: String,
    val healthTone: String,
    val recommendedAction: String,
    val nodeDisplay: String,
    val workspacePath: String,
    val gitStatus: String,
    val cliStatus: String,
    val diskFreeText: String,
    val latestExecution: String,
    val warnings: List<String>,
    val recoveryActions: List<ProjectWorkspaceRecoveryAction>
)

internal data class ProjectWorkspaceRecoveryAction(
    val key: String,
    val label: String,
    val description: String,
    val available: Boolean
)

internal data class ProjectWorkspaceRecoveryResult(
    val message: String,
    val archiveProject: ArchiveProjectRecord?
)

internal fun fetchProjectWorkspaceHealth(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String
): ProjectWorkspaceHealth {
    val req = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${workspaceUrlPart(projectId)}/workspace/health")
            .get()
    ).build()
    val resp = http.newCall(req).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(workspaceApiError(body, resp.code))
    return parseProjectWorkspaceHealth(JSONObject(body))
}

internal fun recoverProjectWorkspace(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    action: String,
    nodeId: String? = null
): ProjectWorkspaceRecoveryResult {
    val payload = JSONObject().apply {
        put("action", action)
        if (!nodeId.isNullOrBlank()) put("node_id", nodeId)
    }
    val req = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${workspaceUrlPart(projectId)}/workspace/recover")
            .post(payload.toString().toRequestBody("application/json".toMediaType()))
    ).build()
    val resp = http.newCall(req).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(workspaceApiError(body, resp.code))
    val json = JSONObject(body)
    return ProjectWorkspaceRecoveryResult(
        message = json.optString("message").ifBlank { "PC 工作区已处理" },
        archiveProject = json.optJSONObject("archive_project")?.let(::parseArchiveProject)
    )
}

private fun parseProjectWorkspaceHealth(json: JSONObject): ProjectWorkspaceHealth {
    val node = json.optJSONObject("node")
    val project = json.optJSONObject("project")
    val inspect = json.optJSONObject("live_inspect")
    val warnings = json.optJSONArray("warnings")?.let { arr ->
        (0 until arr.length()).mapNotNull { idx -> arr.optString(idx).takeIf { it.isNotBlank() } }
    }.orEmpty()
    val actions = json.optJSONArray("recovery_actions")?.let { arr ->
        (0 until arr.length()).mapNotNull { idx ->
            arr.optJSONObject(idx)?.let {
                ProjectWorkspaceRecoveryAction(
                    key = it.optString("key"),
                    label = it.optString("label").ifBlank { it.optString("key") },
                    description = it.optString("description"),
                    available = it.optBoolean("available", false)
                )
            }
        }
    }.orEmpty()
    return ProjectWorkspaceHealth(
        healthLabel = json.optString("health_label").ifBlank { "待处理" },
        healthTone = json.optString("health_tone").ifBlank { "warn" },
        recommendedAction = json.optString("recommended_action"),
        nodeDisplay = nodeDisplay(node),
        workspacePath = project?.optString("workspace_path").orEmpty()
            .ifBlank { inspect?.optString("workspace_path").orEmpty() }
            .ifBlank { "未设置" },
        gitStatus = gitStatus(inspect),
        cliStatus = cliStatus(inspect),
        diskFreeText = formatWorkspaceBytes(inspect?.optLong("disk_free_bytes", 0L) ?: 0L),
        latestExecution = latestExecution(json.optJSONObject("latest_execution")),
        warnings = warnings,
        recoveryActions = actions
    )
}

private fun nodeDisplay(node: JSONObject?): String {
    if (node == null) return "未绑定"
    val parts = listOf(
        node.optString("device_name"),
        node.optString("node_id"),
        if (node.optBoolean("online", false)) "在线" else "离线"
    ).mapNotNull { it.trim().takeIf { value -> value.isNotBlank() } }
    return parts.joinToString(" · ").ifBlank { "未绑定" }
}

private fun gitStatus(inspect: JSONObject?): String {
    if (inspect == null) return "未检查"
    if (!inspect.optBoolean("path_exists", false)) return "目录不存在"
    if (!inspect.optBoolean("is_git_worktree", false)) return "不是 Git worktree"
    val parts = listOf(
        inspect.optString("git_branch").ifBlank { "HEAD" },
        inspect.optString("git_head"),
        if (inspect.optBoolean("has_uncommitted_changes", false)) "有未提交改动" else "干净"
    ).mapNotNull { it.trim().takeIf { value -> value.isNotBlank() } }
    return parts.joinToString(" · ")
}

private fun cliStatus(inspect: JSONObject?): String {
    if (inspect == null) return "未检查"
    val names = listOf(
        if (inspect.optBoolean("codex_available", false)) "Codex" else "",
        if (inspect.optBoolean("copilot_available", false)) "Copilot" else ""
    ).mapNotNull { it.takeIf { value -> value.isNotBlank() } }
    return names.joinToString(" / ").ifBlank { "未检测到 CLI" }
}

private fun latestExecution(json: JSONObject?): String {
    if (json == null) return "暂无执行记录"
    return listOf(
        json.optString("status"),
        json.optString("merge_status"),
        json.optString("updated_at").ifBlank { json.optString("completed_at") }.ifBlank { json.optString("started_at") }
    ).mapNotNull { it.trim().takeIf { value -> value.isNotBlank() } }.joinToString(" · ").ifBlank { "暂无执行记录" }
}

private fun formatWorkspaceBytes(bytes: Long): String {
    if (bytes <= 0) return "未检查"
    val units = listOf("B", "KB", "MB", "GB", "TB")
    var value = bytes.toDouble()
    var index = 0
    while (value >= 1024.0 && index < units.lastIndex) {
        value /= 1024.0
        index += 1
    }
    return if (index >= 3) "%.1f %s".format(value, units[index]) else "${value.toLong()} ${units[index]}"
}

private fun workspaceUrlPart(value: String): String {
    return java.net.URLEncoder.encode(value, "UTF-8").replace("+", "%20")
}

private fun workspaceApiError(body: String, code: Int): String {
    if (body.isBlank()) return "HTTP $code"
    return runCatching {
        val obj = JSONObject(body)
        obj.optString("error").ifBlank { obj.optString("message") }.ifBlank { body }
    }.getOrDefault(body)
}
