package com.elon.app

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject

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
    val lastTaskStatus: String?
)

internal fun StoreProject.toJointAppProject(): AppProject {
    return newAppProject(name, description ?: "联合项目").copy(
        id = id,
        isJointProject = true,
        collaborationProjectId = id
    )
}

// ─── API 函数（在调用方手动切换线程） ─────────────────────────────────────────

/** GET /api/store/projects — 无需登录 */
internal fun fetchStoreProjects(
    http: OkHttpClient,
    serverUrl: String,
    search: String? = null,
    limit: Int = 30,
    offset: Int = 0
): List<StoreProject> {
    val qs = buildString {
        append("limit=$limit&offset=$offset")
        if (!search.isNullOrBlank()) append("&q=${java.net.URLEncoder.encode(search, "UTF-8")}")
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

/** POST /api/projects — 创建私有服务器项目，发布为联合项目时再设置公开 */
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
    }
    val resp = http.newCall(
        Request.Builder()
            .url("$serverUrl/api/projects")
            .post(payload.toString().toRequestBody("application/json".toMediaType()))
            .header("Authorization", "Bearer $token")
            .build()
    ).execute()
    val body = resp.body?.string().orEmpty()
    if (!resp.isSuccessful) error(body.ifBlank { "HTTP ${resp.code}" })
    val project = JSONObject(body).optJSONObject("project")
        ?: error("响应缺少 project")
    return parseCreatedStoreProject(project, ownerAccount)
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

// ─── 解析 ─────────────────────────────────────────────────────────────────────

private fun parseStoreProjectList(json: JSONObject): List<StoreProject> {
    val arr = json.optJSONArray("projects") ?: return emptyList()
    return (0 until arr.length()).map { i -> parseStoreProject(arr.getJSONObject(i)) }
}

internal fun parseStoreProject(obj: JSONObject) = StoreProject(
    id = obj.getString("id"),
    name = obj.getString("name"),
    description = obj.optString("description").takeIf { it.isNotBlank() },
    template = obj.optString("template", "custom"),
    ownerAccount = obj.optString("owner_account", "?"),
    ownerUserId = obj.optString("owner_id", ""),
    memberCount = obj.optInt("member_count", 0),
    isPublic = obj.optBoolean("is_public", true),
    joinMode = obj.optString("join_mode", "open"),
    lastTaskStatus = obj.optString("last_task_status").takeIf { it.isNotBlank() }
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
    joinMode = "open",
    lastTaskStatus = null
)

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
