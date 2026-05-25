package com.elon.app

import android.content.Context
import android.content.SharedPreferences
import org.json.JSONObject
import java.net.URLEncoder

internal fun taskProjectWsUrl(
    context: Context,
    prefs: SharedPreferences,
    payload: String?
): String {
    val json = payload
        ?.let { runCatching { JSONObject(it) }.getOrNull() }
    val userId = json
        ?.optString("user_id")
        ?.takeIf { it.isNotBlank() }
        ?: AuthManager.effectiveUserId(context)
    val projectId = json
        ?.optString("project_id")
        ?.takeIf { it.isNotBlank() }
        ?: prefs.getString(TaskWorkService.PREF_ACTIVE_PROJECT_ID, null)
        ?: "elon-self"
    val projectTitle = json
        ?.optString("project_title")
        ?.takeIf { it.isNotBlank() }
    val query = mutableListOf("app_version_code=${BuildConfig.VERSION_CODE}")
    projectTitle?.let { query += "title=${taskWsPathPart(it)}" }
    return "ws://43.139.149.158:8080/ws/user/${taskWsPathPart(userId)}/projects/${taskWsPathPart(projectId)}?${query.joinToString("&")}"
}

private fun taskWsPathPart(value: String): String {
    return URLEncoder.encode(value, "UTF-8").replace("+", "%20")
}
