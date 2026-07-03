package com.elon.app

import android.content.Context
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.net.URLEncoder
import java.time.Instant

internal data class ProjectSpaceRoute(
    val userId: String? = null,
    val projectTitle: String? = null
) {
    val isUserProject: Boolean
        get() = !userId.isNullOrBlank()
}

internal fun fetchProjectSpace(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): ProjectSpace {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url(projectSpaceUrl(serverUrl, projectId, route, "space"))
            .get()
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "加载项目空间失败"))
        return parseProjectSpace(JSONObject(body))
    }
}

internal fun updateProjectSpaceDescription(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    description: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): ProjectSpaceSummary {
    val payload = JSONObject().put("description", description)
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url(projectSpaceUrl(serverUrl, projectId, route, "space/description"))
            .method("PATCH", payload.toString().toRequestBody("application/json".toMediaType()))
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "保存项目简介失败"))
        return parseProjectSpaceSummary(JSONObject(body).optJSONObject("project") ?: JSONObject())
    }
}

internal fun updateProjectGalleryImage(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    slot: Int,
    imageUrl: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): List<String> {
    val payload = JSONObject()
        .put("slot", slot)
        .put("image_url", imageUrl)
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url(projectSpaceUrl(serverUrl, projectId, route, "space/gallery-image"))
            .method("PATCH", payload.toString().toRequestBody("application/json".toMediaType()))
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "保存应用图片失败"))
        return parseProjectImageList(JSONObject(body).optJSONArray("gallery_images") ?: JSONArray())
    }
}

internal fun fetchProjectChannelMessages(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    limit: Int = 120,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): List<ProjectChannelMessage> {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url(projectSpaceUrl(serverUrl, projectId, route, "channels/${projectSpaceUrlPart(channelId)}/messages", "limit=$limit"))
            .get()
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "加载频道消息失败"))
        val arr = JSONObject(body).optJSONArray("messages") ?: JSONArray()
        return List(arr.length()) { parseProjectChannelMessage(arr.optJSONObject(it) ?: JSONObject()) }
    }
}

internal fun fetchProjectMemberConversations(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    memberUserId: String,
    limit: Int = 50
): List<ProjectMemberConversation> {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/members/${projectSpaceUrlPart(memberUserId)}/conversations?limit=$limit")
            .get()
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "加载成员会话失败"))
        val arr = JSONObject(body).optJSONArray("conversations") ?: JSONArray()
        return List(arr.length()) { parseProjectMemberConversation(arr.optJSONObject(it) ?: JSONObject()) }
    }
}

internal fun fetchProjectMemberConversationMessages(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    memberUserId: String,
    conversationId: String,
    limit: Int = 120
): List<ProjectMemberConversationMessage> {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/members/${projectSpaceUrlPart(memberUserId)}/conversations/${projectSpaceUrlPart(conversationId)}/messages?limit=$limit")
            .get()
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "加载会话消息失败"))
        val arr = JSONObject(body).optJSONArray("messages") ?: JSONArray()
        return List(arr.length()) {
            parseProjectMemberConversationMessage(arr.optJSONObject(it) ?: JSONObject())
        }
    }
}

internal fun sendProjectMemberConversationMessage(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    memberUserId: String,
    conversationId: String,
    content: String
): ProjectMemberConversationMessage {
    val payload = JSONObject().put("content", content)
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/members/${projectSpaceUrlPart(memberUserId)}/conversations/${projectSpaceUrlPart(conversationId)}/messages")
            .post(payload.toString().toRequestBody("application/json".toMediaType()))
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "发送成员会话讨论失败"))
        return parseProjectMemberConversationMessage(JSONObject(body).optJSONObject("message") ?: JSONObject())
    }
}

internal fun updateProjectMemberConversationVisibility(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    conversationId: String,
    isPublic: Boolean
): ProjectMemberConversation {
    val payload = JSONObject().put("is_public", isPublic)
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/conversations/${projectSpaceUrlPart(conversationId)}/visibility")
            .method("PATCH", payload.toString().toRequestBody("application/json".toMediaType()))
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "修改会话公开状态失败"))
        return parseProjectMemberConversation(JSONObject(body).optJSONObject("conversation") ?: JSONObject())
    }
}

internal fun fetchProjectInviteFriends(
    http: OkHttpClient,
    serverUrl: String,
    context: Context
): List<AppFriend> {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/me/friends")
            .get()
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "加载好友失败"))
        val arr = JSONObject(body).optJSONArray("friends") ?: JSONArray()
        return List(arr.length()) { parseProjectInviteFriend(arr.optJSONObject(it) ?: JSONObject()) }
            .filter { it.id != PROJECT_INVITE_SOCIAL_AI_USER_ID }
    }
}

internal fun inviteProjectMember(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    account: String,
    role: String
): ProjectMember {
    val payload = JSONObject()
        .put("account", account)
        .put("role", role)
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/members")
            .post(payload.toString().toRequestBody("application/json".toMediaType()))
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "邀请成员失败"))
        return parseProjectMember(JSONObject(body).optJSONObject("member") ?: JSONObject())
    }
}

internal fun updateProjectMemberRole(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    memberUserId: String,
    role: String
): ProjectMember {
    val payload = JSONObject().put("role", role)
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/members/${projectSpaceUrlPart(memberUserId)}")
            .method("PATCH", payload.toString().toRequestBody("application/json".toMediaType()))
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "修改权限失败"))
        return parseProjectMember(JSONObject(body).optJSONObject("member") ?: JSONObject())
    }
}

internal fun removeProjectMember(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    memberUserId: String
) {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/members/${projectSpaceUrlPart(memberUserId)}")
            .delete()
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "移除成员失败"))
    }
}

internal fun sendProjectChannelMessage(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    content: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute(),
    replyToMessageId: String? = null
): ProjectChannelMessage {
    val payload = JSONObject().put("content", content)
    replyToMessageId.cleanProjectSpaceApiString()?.let { payload.put("reply_to_message_id", it) }
    return postProjectChannelPayload(
        http = http,
        serverUrl = serverUrl,
        context = context,
        projectId = projectId,
        channelId = channelId,
        suffix = "messages",
        payload = payload,
        route = route
    )
}

internal fun startProjectChannelAiTask(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    content: String,
    agent: String? = null,
    runtimeRoute: String? = null,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): ProjectChannelMessage {
    val payload = JSONObject().put("content", content)
    agent?.takeIf { it.isNotBlank() }?.let { payload.put("agent", it) }
    runtimeRoute?.takeIf { it.isNotBlank() }?.let { payload.put("runtimeRoute", it) }
    return postProjectChannelPayload(
        http = http,
        serverUrl = serverUrl,
        context = context,
        projectId = projectId,
        channelId = channelId,
        suffix = "ai-tasks",
        payload = payload,
        route = route
    )
}

internal fun summarizeProjectChannelMessages(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    postContent: String,
    summaryPrompt: String,
    agent: String? = null,
    runtimeRoute: String? = null,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): ProjectChannelMessage {
    val payload = JSONObject()
        .put("post_content", postContent)
        .put("summary_prompt", summaryPrompt)
    agent?.takeIf { it.isNotBlank() }?.let { payload.put("agent", it) }
    runtimeRoute?.takeIf { it.isNotBlank() }?.let { payload.put("runtimeRoute", it) }
    return postProjectChannelPayload(
        http = http,
        serverUrl = serverUrl,
        context = context,
        projectId = projectId,
        channelId = channelId,
        suffix = "summaries",
        payload = payload,
        route = route
    )
}

internal fun markProjectSuggestionUpdated(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    messageId: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): ProjectChannelMessage {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url(projectSpaceUrl(
                serverUrl,
                projectId,
                route,
                "channels/${projectSpaceUrlPart(channelId)}/messages/${projectSpaceUrlPart(messageId)}/suggestion"
            ))
            .method("PATCH", "{}".toRequestBody("application/json".toMediaType()))
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "标记失败"))
        return parseProjectChannelMessage(JSONObject(body).optJSONObject("message") ?: JSONObject())
    }
}

private fun postProjectChannelPayload(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    suffix: String,
    payload: JSONObject,
    route: ProjectSpaceRoute
): ProjectChannelMessage {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url(projectSpaceUrl(serverUrl, projectId, route, "channels/${projectSpaceUrlPart(channelId)}/$suffix"))
            .post(payload.toString().toRequestBody("application/json".toMediaType()))
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "发送失败"))
        return parseProjectChannelMessage(JSONObject(body).optJSONObject("message") ?: JSONObject())
    }
}

private fun parseProjectSpace(json: JSONObject): ProjectSpace {
    val project = json.optJSONObject("project") ?: JSONObject()
    val channels = json.optJSONArray("channels") ?: JSONArray()
    val members = json.optJSONArray("members") ?: JSONArray()
    return ProjectSpace(
        project = parseProjectSpaceSummary(project),
        channels = List(channels.length()) { parseProjectChannel(channels.optJSONObject(it) ?: JSONObject()) },
        members = List(members.length()) { parseProjectMember(members.optJSONObject(it) ?: JSONObject()) },
        latestApkUrl = cleanProjectApkUrl(json.optString("latest_apk_url")),
        latestApkIdentity = json.optString(
            "latest_apk_identity",
            json.optString("latestApkIdentity")
        ).cleanProjectSpaceApiString(),
        latestApkUpdatedAt = json.optString(
            "latest_apk_updated_at",
            json.optString("latestApkUpdatedAt")
        ).cleanProjectSpaceApiString(),
        galleryImages = parseProjectImageList(
            json.optJSONArray("gallery_images")
                ?: project.optJSONArray("gallery_images")
                ?: project.optJSONArray("galleryImages")
                ?: JSONArray()
        ),
        landingPreviewImages = parseLandingPreviewImages(json.optJSONObject("landing"))
    )
}

private fun parseProjectSpaceSummary(project: JSONObject) = ProjectSpaceSummary(
    id = project.optString("id", ""),
    name = project.optProjectSpaceDisplayName() ?: project.optString("name", "项目空间"),
    description = project.optString("description").takeIf { it.isNotBlank() },
    role = project.optString("role", "member"),
    joinMode = normalizeProjectJoinMode(project.optString("join_mode", project.optString("joinMode", PROJECT_JOIN_MODE_INVITE))),
    memberCount = project.optInt("member_count", 0),
    iconDataUrl = project.optProjectSpaceIconDataUrl(),
    updatedAt = project.optString("updated_at", "")
)

private fun JSONObject.optProjectSpaceIconDataUrl(): String? {
    val keys = arrayOf("iconDataUrl", "icon_data_url", "iconUrl", "icon_url", "icon", "avatar", "logo")
    for (key in keys) {
        val value = optString(key, "").trim()
        if (value.isNotBlank()) return value
    }
    return null
}

private fun JSONObject.optProjectSpaceDisplayName(): String? {
    val keys = arrayOf("displayName", "display_name", "alias", "project_alias")
    for (key in keys) {
        optString(key, "").cleanProjectSpaceApiString()?.let { return it }
    }
    return null
}

private fun parseProjectImageList(arr: JSONArray): List<String> {
    return List(arr.length()) { index ->
        arr.optString(index, "").cleanProjectSpaceApiString().orEmpty()
    }.take(4)
}

private fun parseLandingPreviewImages(landing: JSONObject?): List<String> {
    val media = landing?.optJSONArray("media") ?: return emptyList()
    return buildList {
        for (index in 0 until media.length()) {
            val item = media.optJSONObject(index) ?: continue
            val kind = item.optString("kind", item.optString("type", "")).trim().lowercase()
            val url = item.optString("url", item.optString("src", "")).cleanProjectSpaceApiString()
                ?: continue
            val looksLikeImage = kind.contains("image") ||
                url.substringBefore('?').lowercase().matches(Regex(""".*\.(png|jpe?g|webp|gif)$"""))
            if (looksLikeImage) add(url)
            if (size >= 4) break
        }
    }
}

private fun parseProjectChannel(json: JSONObject) = ProjectChannel(
    id = json.optString("id", ""),
    projectId = json.optString("project_id", ""),
    name = json.optString("name", "频道"),
    kind = json.optString("kind", "discussion"),
    position = json.optInt("position", 0),
    lastMessage = json.optString("last_message").takeIf { it.isNotBlank() },
    lastMessageAt = json.optString("last_message_at").takeIf { it.isNotBlank() },
    unreadCount = json.optInt("unread_count", 0).coerceAtLeast(0)
)

private fun parseProjectMember(json: JSONObject) = ProjectMember(
    userId = json.optString("user_id", ""),
    account = json.optString("account", ""),
    avatarDataUrl = json.optString("avatar_data_url").takeIf { it.isNotBlank() },
    role = json.optString("role", "member"),
    joinedAt = json.optString("joined_at", "")
)

private fun parseProjectInviteFriend(json: JSONObject): AppFriend {
    val account = json.optString("account", "").trim()
    val phone = json.optString("phone", "").trim().takeIf { it.isNotEmpty() }
    val nickname = json.optString("nickname", "").trim().takeIf { it.isNotEmpty() }
    return AppFriend(
        id = json.optString("id", "").trim(),
        name = nickname ?: account.ifBlank { phone ?: "好友" },
        account = account,
        phone = phone,
        avatarDataUrl = json.optString("avatar_data_url", "").trim().takeIf { it.isNotEmpty() },
        friendSince = json.optString("friend_since", "").trim().takeIf { it.isNotEmpty() },
        lastMessage = json.optString("last_message", "").trim().takeIf { it.isNotEmpty() },
        lastMessageAt = parseProjectInviteServerTime(json.optString("last_message_at", "").trim()),
        unreadCount = json.optInt("unread_count", 0).coerceAtLeast(0),
        isOnline = json.optBoolean("is_online", false)
    )
}

private fun parseProjectInviteServerTime(value: String): Long? {
    if (value.isBlank()) return null
    return runCatching { Instant.parse(value).toEpochMilli() }.getOrNull()
}

private fun parseProjectMemberConversation(json: JSONObject) = ProjectMemberConversation(
    id = json.optString("id", ""),
    projectId = json.optString("project_id", ""),
    userId = json.optString("user_id", ""),
    userAccount = json.optString("user_account", ""),
    title = json.optString("title").takeIf { it.isNotBlank() },
    status = json.optString("status", "active"),
    isPublic = json.optBoolean("is_public", true),
    messageCount = json.optInt("message_count", 0).coerceAtLeast(0),
    taskCount = json.optInt("task_count", 0).coerceAtLeast(0),
    lastMessage = json.optString("last_message").takeIf { it.isNotBlank() },
    lastMessageRole = json.optString("last_message_role").takeIf { it.isNotBlank() },
    lastMessageAt = json.optString("last_message_at").takeIf { it.isNotBlank() },
    lastTaskStatus = json.optString("last_task_status").takeIf { it.isNotBlank() },
    createdAt = json.optString("created_at", ""),
    updatedAt = json.optString("updated_at", "")
)

private fun parseProjectMemberConversationMessage(json: JSONObject) = ProjectMemberConversationMessage(
    id = json.optString("id", ""),
    projectId = json.optString("project_id", ""),
    conversationId = json.cleanProjectSpaceString("conversation_id"),
    taskId = json.cleanProjectSpaceString("task_id"),
    userId = json.cleanProjectSpaceString("user_id"),
    senderName = json.cleanProjectSpaceString("sender_name"),
    senderAvatarDataUrl = json.cleanProjectSpaceString("sender_avatar_data_url"),
    role = json.optString("role", "user"),
    content = json.optString("content", ""),
    createdAt = json.optString("created_at", ""),
    outgoing = json.optBoolean("outgoing", false)
)

internal fun parseProjectChannelMessage(json: JSONObject) = ProjectChannelMessage(
    id = json.optString("id", ""),
    projectId = json.optString("project_id", ""),
    channelId = json.optString("channel_id", ""),
    senderUserId = json.cleanProjectSpaceString("sender_user_id"),
    senderName = json.cleanProjectSpaceString("sender_name"),
    senderAvatarDataUrl = json.cleanProjectSpaceString("sender_avatar_data_url"),
    replyToMessageId = json.cleanProjectSpaceString("reply_to_message_id"),
    kind = json.optString("kind", "text"),
    content = json.optString("content", ""),
    taskId = json.cleanProjectSpaceString("task_id"),
    taskStatus = json.cleanProjectSpaceString("task_status"),
    taskError = json.cleanProjectSpaceString("task_error"),
    taskApkUrl = json.cleanProjectSpaceString("task_apk_url"),
    taskCodexThreadId = json.cleanProjectSpaceString("task_codex_thread_id"),
    suggestionStatus = json.cleanProjectSpaceString("suggestion_status"),
    suggestionResolvedBy = json.cleanProjectSpaceString("suggestion_resolved_by"),
    suggestionResolvedByName = json.cleanProjectSpaceString("suggestion_resolved_by_name"),
    suggestionResolvedAt = json.cleanProjectSpaceString("suggestion_resolved_at"),
    createdAt = json.optString("created_at", ""),
    outgoing = json.optBoolean("outgoing", false)
)

private fun JSONObject.cleanProjectSpaceString(key: String): String? {
    return optString(key, "").cleanProjectSpaceApiString()
}

private fun String?.cleanProjectSpaceApiString(): String? {
    return this?.trim()?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}

private fun readProjectSpaceError(body: String, fallback: String): String {
    if (body.isBlank()) return fallback
    return runCatching {
        JSONObject(body).optString("error", "").ifBlank { fallback }
    }.getOrDefault(fallback)
}

private const val PROJECT_INVITE_SOCIAL_AI_USER_ID = "usr_elon_ai"

private fun projectSpaceUrlPart(value: String): String = URLEncoder.encode(value, Charsets.UTF_8.name())

internal fun projectSpaceUrl(
    serverUrl: String,
    projectId: String,
    route: ProjectSpaceRoute,
    suffix: String,
    extraQuery: String? = null
): String {
    val base = route.userId?.takeIf { it.isNotBlank() }?.let { userId ->
        "$serverUrl/api/user/${projectSpaceUrlPart(userId)}/projects/${projectSpaceUrlPart(projectId)}"
    } ?: "$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}"
    val query = buildList {
        if (route.isUserProject) {
            route.projectTitle?.trim()?.takeIf { it.isNotBlank() }?.let {
                add("title=${projectSpaceUrlPart(it)}")
            }
        }
        extraQuery?.takeIf { it.isNotBlank() }?.let { add(it) }
    }
    return buildString {
        append(base)
        append("/")
        append(suffix.trimStart('/'))
        if (query.isNotEmpty()) append("?").append(query.joinToString("&"))
    }
}
