package com.elon.app

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject
import java.net.URLEncoder

internal fun fetchProjectGitStatus(
    http: OkHttpClient,
    serverUrl: String,
    userId: String,
    project: AppProject
): GitProjectStatus {
    val response = http.newCall(
        Request.Builder()
            .url(projectGitUrl(serverUrl, userId, project.id, project.title, "status"))
            .get()
            .build()
    ).execute()
    val body = response.body?.string().orEmpty()
    if (!response.isSuccessful) error(body.ifBlank { "HTTP ${response.code}" })
    return parseGitProjectStatus(JSONObject(body))
}

internal fun generateProjectDeployKey(
    http: OkHttpClient,
    serverUrl: String,
    userId: String,
    project: AppProject
): Pair<String, GitProjectStatus> {
    val emptyBody = "{}".toRequestBody("application/json".toMediaType())
    val response = http.newCall(
        Request.Builder()
            .url(projectGitUrl(serverUrl, userId, project.id, project.title, "deploy-key"))
            .post(emptyBody)
            .build()
    ).execute()
    val body = response.body?.string().orEmpty()
    if (!response.isSuccessful) error(body.ifBlank { "HTTP ${response.code}" })
    val json = JSONObject(body)
    val publicKey = json.optString("public_key", "")
    val status = parseGitProjectStatus(json.optJSONObject("status") ?: JSONObject())
    return publicKey to status
}

internal fun saveProjectGitConfig(
    http: OkHttpClient,
    serverUrl: String,
    userId: String,
    project: AppProject,
    repoUrl: String,
    branch: String
): GitProjectStatus {
    val payload = JSONObject().apply {
        put("repo_url", repoUrl)
        put("branch", branch)
        put("auth_type", "deploy_key")
    }
    val body = payload.toString().toRequestBody("application/json".toMediaType())
    val response = http.newCall(
        Request.Builder()
            .url(projectGitUrl(serverUrl, userId, project.id, project.title, "config"))
            .post(body)
            .build()
    ).execute()
    val responseBody = response.body?.string().orEmpty()
    if (!response.isSuccessful) error(responseBody.ifBlank { "HTTP ${response.code}" })
    return parseGitProjectStatus(JSONObject(responseBody))
}

internal fun projectGitUrl(
    serverUrl: String,
    userId: String,
    projectId: String,
    projectTitle: String,
    action: String
): String {
    return "$serverUrl/api/user/${urlPart(userId)}/projects/${urlPart(projectId)}/git/$action?title=${urlPart(projectTitle)}"
}

internal fun urlPart(value: String): String {
    return URLEncoder.encode(value, "UTF-8").replace("+", "%20")
}
