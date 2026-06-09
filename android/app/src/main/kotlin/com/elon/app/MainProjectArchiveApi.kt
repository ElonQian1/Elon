package com.elon.app

import android.content.Context
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import org.json.JSONObject

internal data class ArchiveProjectRecord(
    val id: String,
    val name: String,
    val description: String?,
    val role: String,
    val isPublic: Boolean,
    val joinMode: String,
    val lastTaskStatus: String?,
    val ownerAccount: String?,
    val ownerUserId: String?,
    val memberCount: Int,
    val updatedAtMs: Long,
    val conversationCount: Int,
    val iconDataUrl: String? = null,
    val systemKey: String? = null,
    val workspaceKind: String? = null,
    val conversationRoute: ArchiveConversationRoute? = null,
    val workspaceStatus: ArchiveWorkspaceStatus? = null
) {
    fun toAppProject(): AppProject {
        val systemKey = systemKey?.trim()
            ?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
        val isSystem = !systemKey.isNullOrBlank()
        val opensAsJoint = !isSystem && (role != "owner" || isPublic)
        val subtitle = when {
            !description.isNullOrBlank() -> description
            isSystem -> "${systemDisplayName(systemKey)}专属会话归档"
            opensAsJoint -> "联合开发项目"
            else -> "个人开发项目"
        }
        return AppProject(
            id = id,
            title = summarize(name, 24),
            subtitle = subtitle,
            updatedAt = updatedAtMs,
            stage = lastTaskStatus?.takeIf { it.isNotBlank() }
                ?: if (isSystem) "会话归档" else "待提交需求",
            isJointProject = opensAsJoint,
            collaborationProjectId = id.takeIf { opensAsJoint },
            collaborationJoinMode = joinMode.takeIf { opensAsJoint },
            iconDataUrl = iconDataUrl,
            systemProjectKey = systemKey,
            ownerAccount = ownerAccount?.takeIf { it.isNotBlank() && it != "?" },
            memberCount = memberCount.coerceAtLeast(0),
            projectDescription = description?.takeIf { it.isNotBlank() },
            remoteConversationCount = conversationCount,
            workspaceKind = workspaceKind?.takeIf { it.isNotBlank() },
            workspaceHealthLabel = workspaceStatus?.displayLabel(systemKey),
            workspaceHealthTone = workspaceStatus?.displayTone(),
            archiveEntryKey = conversationRoute?.entryKey,
            memoryScopeType = conversationRoute?.memoryScopeType,
            conversations = mutableListOf()
        )
    }
}

internal data class ArchiveConversationRoute(
    val entryKey: String,
    val projectId: String,
    val projectName: String,
    val conversationTitle: String,
    val memoryScopeType: String,
    val memoryScopeId: String?
)

internal data class ArchiveWorkspaceStatus(
    val workspaceKind: String,
    val executionTarget: String,
    val nodeId: String?,
    val nodeOnline: Boolean,
    val nodeDisplayName: String?,
    val canRunOnPc: Boolean,
    val latestExecutionStatus: String?,
    val latestExecutionMergeStatus: String?,
    val warningCount: Int,
    val warnings: List<String>
) {
    fun displayLabel(systemKey: String?): String {
        if (latestExecutionStatus.equals("running", ignoreCase = true)) return "运行中"
        return when (workspaceKind) {
            "system_archive" -> "${systemDisplayName(systemKey)}归档"
            "pc_node_workspace" -> when {
                canRunOnPc && warningCount <= 0 -> "PC在线"
                canRunOnPc -> "PC有提醒"
                !nodeId.isNullOrBlank() && !nodeOnline -> "PC离线"
                else -> "PC需配置"
            }
            "external_workspace" -> "外部工作区"
            else -> "服务器工作区"
        }
    }

    fun displayTone(): String {
        if (latestExecutionStatus.equals("running", ignoreCase = true)) return "active"
        return when {
            workspaceKind == "system_archive" -> "neutral"
            workspaceKind == "pc_node_workspace" && canRunOnPc && warningCount <= 0 -> "ok"
            workspaceKind == "pc_node_workspace" && !canRunOnPc -> "bad"
            warningCount > 0 -> "warn"
            else -> "neutral"
        }
    }
}

internal data class ProjectArchiveSnapshot(
    val personalProjects: List<ArchiveProjectRecord>,
    val systemProjects: List<ArchiveProjectRecord>,
    val ownedProjects: List<ArchiveProjectRecord>,
    val sharedProjects: List<ArchiveProjectRecord>
) {
    val allProjects: List<ArchiveProjectRecord>
        get() = personalProjects + sharedProjects
}

internal fun fetchMyProjectArchive(
    http: OkHttpClient,
    serverUrl: String,
    ctx: Context
): ProjectArchiveSnapshot {
    val req = AuthManager.applyAuth(ctx, Request.Builder()
        .url("$serverUrl/api/me/archive")
        .get())
    val resp = http.newCall(req.build()).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
    val json = JSONObject(body)
    val systemProjects = parseArchiveProjectList(json.optJSONArray("system_projects"))
    val ownedProjects = parseArchiveProjectList(json.optJSONArray("owned_projects"))
    val personalProjects = json.optJSONArray("personal_projects")?.let(::parseArchiveProjectList)
        ?: (systemProjects + ownedProjects)
    return ProjectArchiveSnapshot(
        personalProjects = personalProjects,
        systemProjects = systemProjects,
        ownedProjects = ownedProjects,
        sharedProjects = parseArchiveProjectList(json.optJSONArray("shared_projects"))
    )
}

private fun parseArchiveProjectList(arr: JSONArray?): List<ArchiveProjectRecord> {
    if (arr == null) return emptyList()
    return (0 until arr.length()).mapNotNull { index ->
        arr.optJSONObject(index)?.let(::parseArchiveProject)
    }
}

private fun parseArchiveProject(obj: JSONObject): ArchiveProjectRecord {
    val project = obj.optJSONObject("project") ?: obj
    return ArchiveProjectRecord(
        id = project.getString("id"),
        name = project.optCleanString("name") ?: "未命名项目",
        description = project.optCleanString("description"),
        role = project.optCleanString("role") ?: "member",
        isPublic = project.optBoolean("is_public", false),
        joinMode = normalizeProjectJoinMode(
            project.optString("join_mode", "invite")
        ),
        lastTaskStatus = project.optCleanString("last_task_status"),
        ownerAccount = project.optCleanString("owner_account") ?: obj.optCleanString("owner_account"),
        ownerUserId = project.optCleanString("owner_id") ?: obj.optCleanString("owner_id"),
        memberCount = project.optInt("member_count", 0).coerceAtLeast(0),
        updatedAtMs = parseChatMessageCreatedAt(
            project.optCleanString("updated_at").orEmpty()
        ) ?: 0L,
        conversationCount = obj.optInt("conversation_count", 0),
        iconDataUrl = project.optArchiveProjectIconDataUrl(),
        systemKey = obj.optCleanString("system_key"),
        workspaceKind = obj.optCleanString("workspace_kind"),
        conversationRoute = parseArchiveConversationRoute(obj.optJSONObject("conversation_route")),
        workspaceStatus = parseArchiveWorkspaceStatus(obj.optJSONObject("workspace_status"))
    )
}

private fun parseArchiveConversationRoute(obj: JSONObject?): ArchiveConversationRoute? {
    if (obj == null) return null
    return ArchiveConversationRoute(
        entryKey = obj.optCleanString("entry_key") ?: return null,
        projectId = obj.optCleanString("project_id") ?: return null,
        projectName = obj.optCleanString("project_name") ?: "",
        conversationTitle = obj.optCleanString("conversation_title") ?: "",
        memoryScopeType = obj.optCleanString("memory_scope_type") ?: "",
        memoryScopeId = obj.optCleanString("memory_scope_id")
    )
}

private fun parseArchiveWorkspaceStatus(obj: JSONObject?): ArchiveWorkspaceStatus? {
    if (obj == null) return null
    val warnings = obj.optJSONArray("warnings")?.let { arr ->
        (0 until arr.length()).mapNotNull { index ->
            arr.optString(index).trim().takeIf { it.isNotBlank() }
        }
    }.orEmpty()
    return ArchiveWorkspaceStatus(
        workspaceKind = obj.optCleanString("workspace_kind") ?: return null,
        executionTarget = obj.optCleanString("execution_target") ?: "",
        nodeId = obj.optCleanString("node_id"),
        nodeOnline = obj.optBoolean("node_online", false),
        nodeDisplayName = obj.optCleanString("node_display_name"),
        canRunOnPc = obj.optBoolean("can_run_on_pc", false),
        latestExecutionStatus = obj.optCleanString("latest_execution_status"),
        latestExecutionMergeStatus = obj.optCleanString("latest_execution_merge_status"),
        warningCount = obj.optInt("warning_count", warnings.size).coerceAtLeast(0),
        warnings = warnings
    )
}

private fun JSONObject.optCleanString(key: String): String? {
    if (!has(key) || isNull(key)) return null
    val value = optString(key, "").trim()
    return value.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}

private fun JSONObject.optArchiveProjectIconDataUrl(): String? {
    val keys = arrayOf("iconDataUrl", "icon_data_url", "iconUrl", "icon_url", "icon", "avatar", "logo")
    for (key in keys) {
        optCleanString(key)?.let { return it }
    }
    return null
}

private fun systemDisplayName(systemKey: String?): String {
    return when (systemKey?.trim()?.lowercase()) {
        "phone_control" -> "手机控制"
        "chat_memory" -> "聊天记忆"
        else -> "系统档案"
    }
}
