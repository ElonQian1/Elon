package com.elon.app

import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject

internal fun fetchServerVersionInfo(http: OkHttpClient, serverVersionUrl: String): ServerVersionInfo? = try {
    val request = Request.Builder()
        .url(serverVersionUrl)
        .addHeader("Cache-Control", "no-cache")
        .build()
    http.newCall(request).execute().use { resp ->
        if (!resp.isSuccessful) return null
        val body = resp.body?.string() ?: return null
        val json = JSONObject(body)
        val versionName = json.optString("versionName", json.optString("version_name", ""))
        val gitSha = json.optString("gitSha", json.optString("git_sha", ""))
        if (versionName.isBlank()) return null
        ServerVersionInfo(versionName = versionName, gitSha = gitSha.takeIf { it.isNotBlank() })
    }
} catch (_: Exception) {
    null
}
