package com.elon.app

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
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
    val role: String = "member"
)

internal fun StoreProject.toJointAppProject(): AppProject {
    return newAppProject(name, description ?: "联合项目").copy(
        id = id,
        isJointProject = true,
        collaborationProjectId = id,
        collaborationJoinMode = normalizeProjectJoinMode(joinMode),
        iconDataUrl = iconDataUrl
    )
}

/**
 * 将 PC 托管项目还原为"个人独立项目"（适用于用户自己创建的 owner 项目）。
 * id 直接用服务端项目 ID，collaborationProjectId 不设置，这样 resolveProjectId()
 * 仍能通过 id 找到项目档案，同时 isJointDevelopmentProject() 返回 false，
 * 项目出现在"个人独立项目"分组。
 */
internal fun StoreProject.toOwnerAppProject(): AppProject {
    return newAppProject(name, description ?: "我的项目").copy(
        id = id,
        isJointProject = false,
        collaborationProjectId = null,
        iconDataUrl = iconDataUrl
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
    ownerAccount: String? = null
): StoreProject {
    val payload = JSONObject().apply {
        put("name", name)
        put("description", description ?: "")
        put("template", "android")
        put("execution_target", "pc_node")
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
    val project = JSONObject(body).optJSONObject("project")
        ?: error("响应缺少 project")
    return parseCreatedStoreProject(project, ownerAccount)
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

/** PATCH /api/projects/:id/visibility — 仅 owner；需要 Bearer token */
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

/** DELETE /api/me/project-share-messages/:project_id — 撤回自己发出的好友/群聊项目卡片 */
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
    name = obj.getString("name"),
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
    role = obj.optString("role", "member")
)

private fun parseCreatedStoreProject(obj: JSONObject, ownerAccount: String?) = StoreProject(
    id = obj.getString("id"),
    name = obj.optString("name", "联合项目"),
    description = obj.optString("description").takeIf { it.isNotBlank() },
    template = obj.optString("template", "android"),
    ownerAccount = ownerAccount?.takeIf { it.isNotBlank() } ?: "?",
    ownerUserId = "",
    memberCount = 1,
    isPublic = false,
    joinMode = normalizeProjectJoinMode(obj.optString("join_mode", "invite")),
    lastTaskStatus = null,
    latestApkUrl = null,
    iconDataUrl = obj.optProjectIconDataUrl()
)

private fun JSONObject.optProjectIconDataUrl(): String? {
    val keys = arrayOf("iconDataUrl", "icon_data_url", "iconUrl", "icon_url", "icon", "avatar", "logo")
    for (key in keys) {
        val value = optString(key, "").trim()
        if (value.isNotBlank()) return value
    }
    return null
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

/** GET /api/me/projects — 返回当前用户拥有或加入的项目列表，需要登录 */
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
