package com.elon.app

import android.content.Context
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.net.URLEncoder

// ─── 数据模型 ─────────────────────────────────────────────────────────────────

internal data class StoreProject(
    val id: String,
    val name: String,
    val description: String?,
    val template: String,
    val ownerAccount: String,
    val ownerUserId: String = "",
    val memberCount: Int,
    val isPublic: Boolean,
    val joinMode: String,
    val lastTaskStatus: String?,
    val latestApkUrl: String? = null,
    val iconDataUrl: String? = null,
    val role: String = "member",
    val projectOriginType: String? = null,
    val projectOriginLabel: String? = null,
    val remoteConversationCount: Int? = null,
    val workspaceKind: String? = null,
    val workspaceHealthLabel: String? = null,
    val workspaceHealthTone: String? = null,
    val archiveEntryKey: String? = null,
    val archiveConversationTitle: String? = null,
    val memoryScopeType: String? = null,
    val memoryScopeId: String? = null,
    val workspacePending: Boolean = false
)

internal data class ProjectCreateNodeOption(
    val nodeId: String,
    val displayName: String,
    val shortId: String,
    val online: Boolean,
    val projectCount: Int = 0,
    val projectLimit: Int = 0,
    val projectSlotsRemaining: Int = 0,
    val cliProjectReady: Boolean = false,
    val workspaceProvisionReady: Boolean = false,
    val aiCliReady: Boolean = false,
    val allowedClis: List<String> = emptyList(),
    val canAcceptProject: Boolean = false,
    val capacityLabel: String = "",
    val capacityTone: String = "",
    val capacityWarnings: List<String> = emptyList(),
    val diskFreeBytes: Long? = null
)

internal fun StoreProject.toJointAppProject(): AppProject {
    return newAppProject(name, description ?: "联合项目").copy(
        id = id,
        isJointProject = true,
        collaborationProjectId = id,
        collaborationJoinMode = normalizeProjectJoinMode(joinMode),
        iconDataUrl = iconDataUrl,
        ownerAccount = ownerAccount.takeIf { it.isNotBlank() && it != "?" },
        projectOriginType = projectOriginType,
        projectOriginLabel = projectOriginLabel,
        memberCount = memberCount.coerceAtLeast(0),
        projectDescription = description?.takeIf { it.isNotBlank() },
        remoteConversationCount = remoteConversationCount,
        workspaceKind = workspaceKind,
        workspaceHealthLabel = workspaceHealthLabel,
        workspaceHealthTone = workspaceHealthTone,
        archiveEntryKey = archiveEntryKey,
        archiveConversationTitle = archiveConversationTitle,
        memoryScopeType = memoryScopeType,
        memoryScopeId = memoryScopeId
    )
}

/**
 * 将 PC 托管项目还原为"个人项目"（适用于用户自己创建的 owner 项目）。
 * id 直接用服务端项目 ID，collaborationProjectId 不设置，这样 resolveProjectId()
 * 仍能通过 id 找到项目档案，同时 isJointDevelopmentProject() 返回 false，
 * 项目出现在"个人项目"分组。
 */
internal fun StoreProject.toOwnerAppProject(): AppProject {
    return newAppProject(name, description ?: "我的项目").copy(
        id = id,
        isJointProject = false,
        collaborationProjectId = null,
        iconDataUrl = iconDataUrl,
        ownerAccount = ownerAccount.takeIf { it.isNotBlank() && it != "?" },
        projectOriginType = projectOriginType ?: "self",
        projectOriginLabel = projectOriginLabel ?: "我创建",
        memberCount = memberCount.coerceAtLeast(0),
        projectDescription = description?.takeIf { it.isNotBlank() },
        remoteConversationCount = remoteConversationCount,
        workspaceKind = workspaceKind,
        workspaceHealthLabel = workspaceHealthLabel,
        workspaceHealthTone = workspaceHealthTone,
        archiveEntryKey = archiveEntryKey,
        archiveConversationTitle = archiveConversationTitle,
        memoryScopeType = memoryScopeType,
        memoryScopeId = memoryScopeId
    )
}

// ─── API 函数（在调用方手动切换线程） ─────────────────────────────────────────

/** GET /api/store/projects — 无需登录 */
internal fun fetchStoreProjects(
    http: OkHttpClient,
    serverUrl: String,
    search: String? = null,
    limit: Int = 30,
    offset: Int = 0,
    joinMode: String? = null,
    hasApk: Boolean? = null,
    sort: String? = null
): List<StoreProject> {
    val qs = buildString {
        append("limit=$limit&offset=$offset")
        fun appendParam(key: String, value: String?) {
            if (!value.isNullOrBlank()) append("&$key=${storeUrlPart(value)}")
        }
        appendParam("q", search)
        appendParam("join_mode", joinMode)
        appendParam("sort", sort)
        if (hasApk != null) append("&has_apk=$hasApk")
    }
    val resp = http.newCall(
        Request.Builder()
            .url("$serverUrl/api/store/projects?$qs")
            .get()
            .build()
    ).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
    return parseStoreProjectList(JSONObject(body))
}

/** POST /api/projects — 创建私有 PC 托管项目，发布为联合项目时再设置公开 */
internal fun createStoreProject(
    http: OkHttpClient,
    serverUrl: String,
    name: String,
    description: String?,
    token: String,
    ownerAccount: String? = null,
    nodeId: String? = null
): StoreProject {
    val payload = JSONObject().apply {
        put("name", name)
        put("description", description ?: "")
        put("template", "android")
        put("execution_target", "pc_node")
        if (!nodeId.isNullOrBlank()) put("node_id", nodeId)
    }
    val resp = http.newCall(
        Request.Builder()
            .url("$serverUrl/api/projects")
            .post(payload.toString().toRequestBody("application/json".toMediaType()))
            .header("Authorization", "Bearer $token")
            .build()
    ).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(apiErrorMessage(body, resp.code))
    val root = JSONObject(body)
    val workspacePending = root.optString("workspace_status") == "pending"
    root.optJSONObject("archive_project")?.let { archive ->
        return parseArchiveProject(archive).toStoreProject(ownerAccount)
            .copy(workspacePending = workspacePending)
    }
    val project = root.optJSONObject("project")
        ?: error("响应缺少 project")
    return parseCreatedStoreProject(project, ownerAccount)
        .copy(workspacePending = workspacePending)
}

internal fun fetchProjectCreateNodes(
    http: OkHttpClient,
    serverUrl: String,
    ctx: Context
): List<ProjectCreateNodeOption> {
    val req = AuthManager.applyAuth(
        ctx,
        Request.Builder().url("$serverUrl/api/nodes").get()
    ).build()
    val resp = http.newCall(req).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(apiErrorMessage(body, resp.code))
    val arr: JSONArray = JSONObject(body).optJSONArray("nodes") ?: return emptyList()
    return (0 until arr.length()).mapNotNull { index ->
        val obj = arr.optJSONObject(index) ?: return@mapNotNull null
        val nodeId = obj.optString("node_id", obj.optString("agent_id", "")).trim()
        if (nodeId.isBlank()) return@mapNotNull null
        val shortId = obj.optString("short_id").ifBlank {
            if (nodeId.length > 16) "...${nodeId.takeLast(14)}" else nodeId
        }
        val deviceName = obj.optString("device_name").trim()
        val displayName = obj.optString("display_name")
            .ifBlank { obj.optString("label") }
            .ifBlank { deviceName }
            .ifBlank { shortId }
        val allowedClis = obj.optJSONArray("allowed_clis")?.let { clis ->
            (0 until clis.length()).mapNotNull { idx ->
                clis.optString(idx).trim().takeIf { it.isNotBlank() }
            }
        }.orEmpty()
        val cliReady = obj.optBoolean("cli_project_ready", false) ||
            allowedClis.any { it.equals("codex", ignoreCase = true) || it.equals("copilot", ignoreCase = true) }
        val workspaceReady = when {
            obj.has("workspace_provision_ready") -> obj.optBoolean("workspace_provision_ready", false)
            obj.has("workspaceProvisionReady") -> obj.optBoolean("workspaceProvisionReady", false)
            else -> cliReady
        }
        val aiCliReady = when {
            obj.has("ai_cli_ready") -> obj.optBoolean("ai_cli_ready", false)
            obj.has("aiCliReady") -> obj.optBoolean("aiCliReady", false)
            else -> cliReady
        }
        val projectCount = obj.optInt("project_count", 0).coerceAtLeast(0)
        val projectLimit = obj.optInt("project_limit", 0).coerceAtLeast(0)
        val projectSlotsRemaining = obj.optInt(
            "project_slots_remaining",
            (projectLimit - projectCount).coerceAtLeast(0)
        ).coerceAtLeast(0)
        val capacityTone = obj.optString("capacity_tone").trim()
        val capacityWarnings = obj.optJSONArray("capacity_warnings")?.let { warnings ->
            (0 until warnings.length()).mapNotNull { idx ->
                warnings.optString(idx).trim().takeIf { it.isNotBlank() }
            }
        }.orEmpty()
        val canAcceptProject = when {
            obj.has("can_accept_project") -> obj.optBoolean("can_accept_project", false)
            obj.has("canAcceptProject") -> obj.optBoolean("canAcceptProject", false)
            else -> obj.optBoolean("online", false) &&
                workspaceReady &&
                projectSlotsRemaining > 0 &&
                !capacityTone.equals("bad", ignoreCase = true)
        }
        val diskFreeBytes = if (obj.has("disk_free_bytes") && !obj.isNull("disk_free_bytes")) {
            obj.optLong("disk_free_bytes").takeIf { it > 0L }
        } else {
            null
        }
        ProjectCreateNodeOption(
            nodeId = nodeId,
            displayName = displayName,
            shortId = shortId,
            online = obj.optBoolean("online", false),
            projectCount = projectCount,
            projectLimit = projectLimit,
            projectSlotsRemaining = projectSlotsRemaining,
            cliProjectReady = cliReady,
            workspaceProvisionReady = workspaceReady,
            aiCliReady = aiCliReady,
            allowedClis = allowedClis,
            canAcceptProject = canAcceptProject,
            capacityLabel = obj.optString("capacity_label").trim(),
            capacityTone = capacityTone,
            capacityWarnings = capacityWarnings,
            diskFreeBytes = diskFreeBytes
        )
    }
}

private fun apiErrorMessage(body: String, code: Int): String {
    if (body.isBlank()) return "HTTP $code"
    return runCatching {
        val obj = JSONObject(body)
        obj.optString("error").ifBlank { obj.optString("message") }.ifBlank { body }
    }.getOrDefault(body)
}

/** POST /api/projects/:id/join — 需要 Bearer token */
internal fun joinStoreProject(
    http: OkHttpClient,
    serverUrl: String,
    projectId: String,
    token: String
) {
    val resp = http.newCall(
        Request.Builder()
            .url("$serverUrl/api/projects/$projectId/join")
            .post("{}".toRequestBody("application/json".toMediaType()))
            .header("Authorization", "Bearer $token")
            .build()
    ).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
}

/** POST /api/projects/:id/request-join — 需要 Bearer token */
internal fun requestJoinStoreProject(
    http: OkHttpClient,
    serverUrl: String,
    projectId: String,
    token: String,
    message: String? = null
) {
    val payload = JSONObject().apply {
        put("message", message.orEmpty())
    }
    val resp = http.newCall(
        Request.Builder()
            .url("$serverUrl/api/projects/$projectId/request-join")
            .post(payload.toString().toRequestBody("application/json".toMediaType()))
            .header("Authorization", "Bearer $token")
            .build()
    ).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
}

/** DELETE /api/projects/:id/leave — 需要 Bearer token */
internal fun leaveStoreProject(
    http: OkHttpClient,
    serverUrl: String,
    projectId: String,
    token: String
) {
    val resp = http.newCall(
        Request.Builder()
            .url("$serverUrl/api/projects/$projectId/leave")
            .delete()
            .header("Authorization", "Bearer $token")
            .build()
    ).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
}

/** DELETE /api/projects/:id 或旧匿名 /api/user/:user_id/projects/:id — 删除项目档案和服务器托管文件 */
internal fun deleteServerProject(
    http: OkHttpClient,
    serverUrl: String,
    projectId: String,
    token: String?,
    userId: String
): Boolean {
    val encodedProjectId = storeUrlPart(projectId)
    val builder = if (!token.isNullOrBlank()) {
        Request.Builder()
            .url("$serverUrl/api/projects/$encodedProjectId")
            .delete()
            .header("Authorization", "Bearer $token")
    } else {
        Request.Builder()
            .url("$serverUrl/api/user/${storeUrlPart(userId)}/projects/$encodedProjectId")
            .delete()
    }
    val resp = http.newCall(builder.build()).execute()
    val body = resp.body?.string().orEmpty()
    if (resp.code == 404) return false
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
    return true
}

/** PATCH /api/projects/:id/visibility — 仅 owner/admin；需要 Bearer token */
internal fun setProjectVisibility(
    http: OkHttpClient,
    serverUrl: String,
    projectId: String,
    isPublic: Boolean,
    joinMode: String,
    token: String
) {
    val payload = JSONObject().apply {
        put("is_public", isPublic)
        put("join_mode", joinMode)
    }
    val resp = http.newCall(
        Request.Builder()
            .url("$serverUrl/api/projects/$projectId/visibility")
            .method("PATCH", payload.toString().toRequestBody("application/json".toMediaType()))
            .header("Authorization", "Bearer $token")
            .build()
    ).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
}

/** PATCH /api/projects/:id/icon — 同步项目 APK 图标 */
internal fun updateProjectIconDataUrl(
    http: OkHttpClient,
    serverUrl: String,
    ctx: android.content.Context,
    projectId: String,
    iconDataUrl: String?
) {
    val payload = JSONObject().apply {
        if (iconDataUrl.isNullOrBlank()) put("icon_data_url", JSONObject.NULL)
        else put("icon_data_url", iconDataUrl)
    }
    val req = AuthManager.applyAuth(
        ctx,
        Request.Builder()
            .url("$serverUrl/api/projects/${storeUrlPart(projectId)}/icon")
            .method("PATCH", payload.toString().toRequestBody("application/json".toMediaType()))
    ).build()
    val resp = http.newCall(req).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(apiErrorMessage(body, resp.code))
}

internal fun revokeProjectShareMessages(
    http: OkHttpClient,
    serverUrl: String,
    projectId: String,
    token: String
): Int {
    val encodedProjectId = storeUrlPart(projectId)
    val resp = http.newCall(
        Request.Builder()
            .url("$serverUrl/api/me/project-share-messages/$encodedProjectId")
            .delete()
            .header("Authorization", "Bearer $token")
            .build()
    ).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
    return runCatching { JSONObject(body).optInt("deleted", 0) }.getOrDefault(0)
}

// ─── 解析 ─────────────────────────────────────────────────────────────────────

private fun parseStoreProjectList(json: JSONObject): List<StoreProject> {
    val arr = json.optJSONArray("projects") ?: return emptyList()
    return (0 until arr.length()).map { i -> parseStoreProject(arr.getJSONObject(i)) }
}

private fun storeUrlPart(value: String): String =
    URLEncoder.encode(value, "UTF-8").replace("+", "%20")

internal fun parseStoreProject(obj: JSONObject) = StoreProject(
    id = obj.getString("id"),
    name = obj.optStoreProjectDisplayName() ?: obj.getString("name"),
    description = obj.optString("description").takeIf { it.isNotBlank() },
    template = obj.optString("template", "custom"),
    ownerAccount = obj.optString("owner_account", "?"),
    ownerUserId = obj.optString("owner_id", ""),
    memberCount = obj.optInt("member_count", 0),
    isPublic = obj.optBoolean("is_public", true),
    joinMode = normalizeProjectJoinMode(obj.optString("join_mode", "open")),
    lastTaskStatus = obj.optString("last_task_status").takeIf { it.isNotBlank() },
    latestApkUrl = obj.optString("latest_apk_url").takeIf { it.isNotBlank() }
        ?: obj.optString("last_apk_url").takeIf { it.isNotBlank() },
    iconDataUrl = obj.optProjectIconDataUrl(),
    role = obj.optString("role", "member"),
    projectOriginType = obj.optCleanStoreString("project_origin_type")
        ?: obj.optCleanStoreString("projectOriginType"),
    projectOriginLabel = obj.optCleanStoreString("project_origin_label")
        ?: obj.optCleanStoreString("projectOriginLabel"),
    remoteConversationCount = obj.optNullableInt("conversation_count"),
    workspaceKind = obj.optCleanStoreString("workspace_kind"),
    workspaceHealthLabel = obj.optCleanStoreString("workspace_health_label"),
    workspaceHealthTone = obj.optCleanStoreString("workspace_health_tone"),
    archiveEntryKey = obj.optCleanStoreString("archive_entry_key"),
    archiveConversationTitle = obj.optCleanStoreString("archive_conversation_title"),
    memoryScopeType = obj.optCleanStoreString("memory_scope_type"),
    memoryScopeId = obj.optCleanStoreString("memory_scope_id")
)

private fun parseCreatedStoreProject(obj: JSONObject, ownerAccount: String?) = StoreProject(
    id = obj.getString("id"),
    name = obj.optStoreProjectDisplayName() ?: obj.optString("name", "联合项目"),
    description = obj.optString("description").takeIf { it.isNotBlank() },
    template = obj.optString("template", "android"),
    ownerAccount = obj.optString("owner_account").takeIf { it.isNotBlank() }
        ?: ownerAccount?.takeIf { it.isNotBlank() }
        ?: "?",
    ownerUserId = "",
    memberCount = obj.optInt("member_count", 1).coerceAtLeast(0),
    isPublic = false,
    joinMode = normalizeProjectJoinMode(obj.optString("join_mode", "invite")),
    lastTaskStatus = null,
    latestApkUrl = null,
    iconDataUrl = obj.optProjectIconDataUrl(),
    projectOriginType = obj.optCleanStoreString("project_origin_type")
        ?: obj.optCleanStoreString("projectOriginType")
        ?: "self",
    projectOriginLabel = obj.optCleanStoreString("project_origin_label")
        ?: obj.optCleanStoreString("projectOriginLabel")
        ?: "我创建",
    workspaceKind = obj.optCleanStoreString("workspace_kind"),
    workspaceHealthLabel = obj.optCleanStoreString("workspace_health_label"),
    workspaceHealthTone = obj.optCleanStoreString("workspace_health_tone"),
    archiveEntryKey = obj.optCleanStoreString("archive_entry_key"),
    archiveConversationTitle = obj.optCleanStoreString("archive_conversation_title"),
    memoryScopeType = obj.optCleanStoreString("memory_scope_type"),
    memoryScopeId = obj.optCleanStoreString("memory_scope_id")
)

private fun JSONObject.optProjectIconDataUrl(): String? {
    val keys = arrayOf("iconDataUrl", "icon_data_url", "iconUrl", "icon_url", "icon", "avatar", "logo")
    for (key in keys) {
        val value = optString(key, "").trim()
        if (value.isNotBlank()) return value
    }
    return null
}

private fun JSONObject.optStoreProjectDisplayName(): String? {
    val keys = arrayOf("displayName", "display_name", "alias", "project_alias")
    for (key in keys) {
        optCleanStoreString(key)?.let { return it }
    }
    return null
}

private fun JSONObject.optCleanStoreString(key: String): String? {
    if (!has(key) || isNull(key)) return null
    val value = optString(key, "").trim()
    return value.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}

private fun JSONObject.optNullableInt(key: String): Int? {
    if (!has(key) || isNull(key)) return null
    return optInt(key)
}

/** GET /api/store/joined — 返回当前用户已加入的项目 ID 集合，需要登录 */
internal fun fetchJoinedProjectIds(
    http: OkHttpClient,
    serverUrl: String,
    ctx: android.content.Context
): Set<String> {
    val req = AuthManager.applyAuth(ctx, Request.Builder()
        .url("$serverUrl/api/store/joined")
        .get())
    val resp = http.newCall(req.build()).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
    val obj = JSONObject(body)
    val arr = obj.optJSONArray("projects") ?: return emptySet()
    return (0 until arr.length()).map { arr.getJSONObject(it).getString("id") }.toSet()
}

/** GET /api/me/projects — 返回当前用户拥有或加入的代码项目列表（不含手机控制/聊天记忆归档），需要登录 */
internal fun fetchMyProjects(
    http: OkHttpClient,
    serverUrl: String,
    ctx: android.content.Context
): List<StoreProject> {
    val req = AuthManager.applyAuth(ctx, Request.Builder()
        .url("$serverUrl/api/me/projects")
        .get())
    val resp = http.newCall(req.build()).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
    return parseStoreProjectList(JSONObject(body))
}

/** PUT /api/me/avatar — 同步头像到服务器，需要登录 */
internal fun syncAvatarToServer(
    http: OkHttpClient,
    serverUrl: String,
    ctx: android.content.Context,
    avatarDataUrl: String
) {
    val body = org.json.JSONObject().put("avatar_data_url", avatarDataUrl)
        .toString().toRequestBody("application/json".toMediaType())
    val req = AuthManager.applyAuth(
        ctx,
        Request.Builder()
            .url("$serverUrl/api/me/avatar")
            .put(body)
    ).build()
    val resp = http.newCall(req).execute()
    val respBody = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(respBody.ifBlank { "HTTP ${resp.code}" })
}

/** GET /api/me — 获取当前登录用户信息，返回 avatar_data_url（可为 null） */
internal fun fetchMyAvatarDataUrl(
    http: OkHttpClient,
    serverUrl: String,
    ctx: android.content.Context
): String? {
    val req = AuthManager.applyAuth(ctx, Request.Builder().url("$serverUrl/api/me").get()).build()
    val resp = http.newCall(req).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) return null
    return JSONObject(body).optJSONObject("user")?.optString("avatar_data_url")
        ?.takeIf { it.isNotBlank() }
}
