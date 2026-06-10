package com.elon.app

import android.content.Context
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject

internal data class ProjectSpaceDocument(
    val title: String,
    val relativePath: String,
    val content: String,
    val sizeBytes: Long,
    val truncated: Boolean
)

internal fun fetchProjectSpaceDocument(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): ProjectSpaceDocument {
    val request = AuthManager.applyAuth(
        context,
        Request.Builder()
            .url(projectSpaceUrl(serverUrl, projectId, route, "docs"))
            .get()
    ).build()
    http.newCall(request).execute().use { response ->
        val body = response.body?.string().orEmpty()
        if (!response.isSuccessful) error(readProjectDocumentError(body, "读取项目文档失败"))
        val doc = JSONObject(body).optJSONObject("document") ?: JSONObject()
        return ProjectSpaceDocument(
            title = doc.optString("title", "README.md"),
            relativePath = doc.optString("path", doc.optString("title", "README.md")),
            content = doc.optString("content", ""),
            sizeBytes = doc.optLong("size_bytes", 0L),
            truncated = doc.optBoolean("truncated", false)
        )
    }
}

private fun readProjectDocumentError(body: String, fallback: String): String {
    if (body.isBlank()) return fallback
    return runCatching {
        JSONObject(body).optString("error", "").ifBlank { fallback }
    }.getOrDefault(fallback)
}
