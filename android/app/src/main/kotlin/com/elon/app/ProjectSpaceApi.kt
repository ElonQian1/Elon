package com.elon.app

import android.content.Context
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.net.URLEncoder

internal fun fetchProjectSpace(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String
): ProjectSpace {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/space")
            .get()
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectSpaceError(body, "加载项目空间失败"))
        return parseProjectSpace(JSONObject(body))
    }
}

internal fun fetchProjectChannelMessages(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    limit: Int = 120
): List<ProjectChannelMessage> {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/channels/${projectSpaceUrlPart(channelId)}/messages?limit=$limit")
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

internal fun sendProjectChannelMessage(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    content: String
): ProjectChannelMessage {
    return postProjectChannelPayload(
        http = http,
        serverUrl = serverUrl,
        context = context,
        projectId = projectId,
        channelId = channelId,
        suffix = "messages",
        payload = JSONObject().put("content", content)
    )
}

internal fun startProjectChannelAiTask(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    content: String
): ProjectChannelMessage {
    return postProjectChannelPayload(
        http = http,
        serverUrl = serverUrl,
        context = context,
        projectId = projectId,
        channelId = channelId,
        suffix = "ai-tasks",
        payload = JSONObject().put("content", content)
    )
}

internal fun summarizeProjectChannelMessages(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    postContent: String,
    summaryPrompt: String
): ProjectChannelMessage {
    return postProjectChannelPayload(
        http = http,
        serverUrl = serverUrl,
        context = context,
        projectId = projectId,
        channelId = channelId,
        suffix = "summaries",
        payload = JSONObject()
            .put("post_content", postContent)
            .put("summary_prompt", summaryPrompt)
    )
}

private fun postProjectChannelPayload(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    channelId: String,
    suffix: String,
    payload: JSONObject
): ProjectChannelMessage {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url("$serverUrl/api/projects/${projectSpaceUrlPart(projectId)}/channels/${projectSpaceUrlPart(channelId)}/$suffix")
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
        project = ProjectSpaceSummary(
            id = project.optString("id", ""),
            name = project.optString("name", "项目空间"),
            description = project.optString("description").takeIf { it.isNotBlank() },
            role = project.optString("role", "member"),
            memberCount = project.optInt("member_count", 0),
            updatedAt = project.optString("updated_at", "")
        ),
        channels = List(channels.length()) { parseProjectChannel(channels.optJSONObject(it) ?: JSONObject()) },
        members = List(members.length()) { parseProjectMember(members.optJSONObject(it) ?: JSONObject()) }
    )
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

private fun parseProjectMemberConversation(json: JSONObject) = ProjectMemberConversation(
    id = json.optString("id", ""),
    projectId = json.optString("project_id", ""),
    userId = json.optString("user_id", ""),
    userAccount = json.optString("user_account", ""),
    title = json.optString("title").takeIf { it.isNotBlank() },
    status = json.optString("status", "active"),
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
    conversationId = json.optString("conversation_id").takeIf { it.isNotBlank() },
    taskId = json.optString("task_id").takeIf { it.isNotBlank() },
    userId = json.optString("user_id").takeIf { it.isNotBlank() },
    role = json.optString("role", "user"),
    content = json.optString("content", ""),
    createdAt = json.optString("created_at", "")
)

internal fun parseProjectChannelMessage(json: JSONObject) = ProjectChannelMessage(
    id = json.optString("id", ""),
    projectId = json.optString("project_id", ""),
    channelId = json.optString("channel_id", ""),
    senderUserId = json.optString("sender_user_id").takeIf { it.isNotBlank() },
    senderName = json.optString("sender_name").takeIf { it.isNotBlank() },
    kind = json.optString("kind", "text"),
    content = json.optString("content", ""),
    taskId = json.optString("task_id").takeIf { it.isNotBlank() },
    createdAt = json.optString("created_at", ""),
    outgoing = json.optBoolean("outgoing", false)
)

private fun readProjectSpaceError(body: String, fallback: String): String {
    if (body.isBlank()) return fallback
    return runCatching {
        JSONObject(body).optString("error", "").ifBlank { fallback }
    }.getOrDefault(fallback)
}

private fun projectSpaceUrlPart(value: String): String = URLEncoder.encode(value, Charsets.UTF_8.name())
