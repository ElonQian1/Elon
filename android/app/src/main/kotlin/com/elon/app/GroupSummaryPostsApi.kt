package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONArray
import org.json.JSONObject
import java.net.URLEncoder

internal object GroupSummaryPostsApi {
    fun fetchPosts(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        group: AppGroup
    ): List<GroupSummaryPost> {
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/me/groups/${urlPart(group.id)}/summary-posts?limit=20")
                .get()
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readErrorMessage(body, "加载总结帖失败"))
            val array = JSONObject(body).optJSONArray("posts") ?: JSONArray()
            return List(array.length()) { index ->
                groupSummaryPostFromJson(array.optJSONObject(index) ?: JSONObject())
            }
        }
    }

    fun fetchDetail(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        group: AppGroup,
        postId: String
    ): GroupSummaryPostDetail {
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/me/groups/${urlPart(group.id)}/summary-posts/${urlPart(postId)}")
                .get()
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readErrorMessage(body, "读取总结帖失败"))
            return groupSummaryPostDetailFromJson(JSONObject(body))
        }
    }

    fun create(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        group: AppGroup
    ): GroupSummaryPost {
        val payload = JSONObject()
            .put("limit", 120)
            .put("pin", true)
            .put("instructions", "请按群聊 AI 文档生成可置顶查看的总结帖；优先区分议题、结论、行动项和相关发言。")
            .toString()
            .toRequestBody("application/json".toMediaType())
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/me/groups/${urlPart(group.id)}/summary-posts")
                .post(payload)
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readErrorMessage(body, "生成总结帖失败"))
            return groupSummaryPostFromJson(JSONObject(body).optJSONObject("post") ?: JSONObject())
        }
    }

    fun updatePinned(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        group: AppGroup,
        postId: String,
        pinned: Boolean
    ) {
        val payload = JSONObject()
            .put("pinned", pinned)
            .toString()
            .toRequestBody("application/json".toMediaType())
        val request = AuthManager.applyAuth(
            activity,
            Request.Builder()
                .url("$serverUrl/api/me/groups/${urlPart(group.id)}/summary-posts/${urlPart(postId)}")
                .patch(payload)
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readErrorMessage(body, "更新总结帖失败"))
        }
    }

    private fun readErrorMessage(body: String, fallback: String): String {
        if (body.isBlank()) return fallback
        return runCatching {
            JSONObject(body).optString("error", "").ifBlank { fallback }
        }.getOrDefault(fallback)
    }

    private fun urlPart(value: String): String {
        return URLEncoder.encode(value, Charsets.UTF_8.name())
    }
}

internal data class GroupSummaryPost(
    val id: String,
    val title: String,
    val summary: String,
    val status: String,
    val sourceMessageCount: Int,
    val modelUsed: String?,
    val error: String?,
    val pinnedAt: String?,
    val createdByName: String?,
    val createdAt: String,
    val updatedAt: String
) {
    val isPinned: Boolean get() = !pinnedAt.isNullOrBlank()

    fun isGenerating(): Boolean {
        return status == "generating" || status == "pending" || status == "draft"
    }

    fun stripMeta(): String {
        val state = when {
            isGenerating() -> "生成中"
            status == "ready_with_fallback" -> "提取式总结"
            status == "ready" -> "已生成"
            else -> status.ifBlank { "总结帖" }
        }
        val pin = if (isPinned) "置顶 · " else ""
        val count = if (sourceMessageCount > 0) " · ${sourceMessageCount} 条发言" else ""
        val author = createdByName?.takeIf { it.isNotBlank() }?.let { " · $it" }.orEmpty()
        return "$pin$state$count$author"
    }
}

internal data class GroupSummarySource(
    val id: String,
    val senderName: String,
    val content: String,
    val createdAt: String
)

internal data class GroupSummaryPostDetail(
    val post: GroupSummaryPost,
    val sources: List<GroupSummarySource>
)

private fun groupSummaryPostFromJson(json: JSONObject): GroupSummaryPost {
    return GroupSummaryPost(
        id = json.optString("id", ""),
        title = json.optString("title", "群聊总结"),
        summary = json.optString("summary", ""),
        status = json.optString("status", ""),
        sourceMessageCount = json.optInt("source_message_count", 0),
        modelUsed = json.optString("model_used", "").takeIf { it.isNotBlank() },
        error = json.optString("error", "").takeIf { it.isNotBlank() },
        pinnedAt = json.optString("pinned_at", "").takeIf { it.isNotBlank() },
        createdByName = json.optString("created_by_name", "").takeIf { it.isNotBlank() },
        createdAt = json.optString("created_at", ""),
        updatedAt = json.optString("updated_at", "")
    )
}

private fun groupSummaryPostDetailFromJson(json: JSONObject): GroupSummaryPostDetail {
    val post = groupSummaryPostFromJson(json.optJSONObject("post") ?: JSONObject())
    val sourcesArray = json.optJSONArray("sources") ?: JSONArray()
    val sources = List(sourcesArray.length()) { index ->
        val item = sourcesArray.optJSONObject(index) ?: JSONObject()
        GroupSummarySource(
            id = item.optString("id", ""),
            senderName = item.optString("sender_name", ""),
            content = item.optString("content", ""),
            createdAt = item.optString("created_at", "")
        )
    }
    return GroupSummaryPostDetail(post, sources)
}
