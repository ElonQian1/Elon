package com.elon.app

import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject

internal class MainCodexPrewarm(
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val userId: String,
    private val activeProject: () -> AppProject,
    private val activeConversation: () -> AppConversation,
    private val isActiveConversationWorking: () -> Boolean,
    private val selectedAgentForRequest: () -> String?
) {
    private val prewarmLock = Any()
    private val prewarmingConversationKeys = mutableSetOf<String>()
    private val lastPrewarmAt = mutableMapOf<String, Long>()

    fun maybePrewarmCodexSession(reason: String) {
        if (isActiveConversationWorking()) return
        val project = activeProject()
        val conversation = activeConversation()
        if (conversation.ended) return

        val key = "${project.id}:${conversation.id}"
        val now = System.currentTimeMillis()
        var shouldStart = false
        synchronized(prewarmLock) {
            val lastStartedAt = lastPrewarmAt[key] ?: 0L
            if (!prewarmingConversationKeys.contains(key) && now - lastStartedAt >= PREWARM_COOLDOWN_MS) {
                prewarmingConversationKeys.add(key)
                lastPrewarmAt[key] = now
                shouldStart = true
            }
        }
        if (!shouldStart) return

        val selectedAgent = selectedAgentForRequest()
        val payload = JSONObject().apply {
            put("conversation_id", conversation.id)
            put("conversation_title", conversation.title)
            if (!selectedAgent.isNullOrBlank()) put("agent", selectedAgent)
        }
        val url = "$serverUrl/api/user/${urlPart(userId)}/projects/${urlPart(project.id)}/prewarm?title=${urlPart(project.title)}"
        DebugTraceStore.record(
            "ui_prewarm_start",
            mapOf("reason" to reason, "project_id" to project.id, "conversation_id" to conversation.id)
        )

        Thread {
            val startedAt = System.currentTimeMillis()
            try {
                val body = payload.toString().toRequestBody("application/json".toMediaType())
                http.newCall(
                    Request.Builder()
                        .url(url)
                        .post(body)
                        .build()
                ).execute().use { response ->
                    val responseBody = response.body?.string().orEmpty()
                    val status = runCatching {
                        JSONObject(responseBody).optString("status", "")
                    }.getOrDefault("")
                    DebugTraceStore.record(
                        if (response.isSuccessful) "ui_prewarm_done" else "ui_prewarm_failed",
                        mapOf(
                            "reason" to reason,
                            "project_id" to project.id,
                            "conversation_id" to conversation.id,
                            "http_code" to response.code,
                            "status" to status,
                            "elapsed_ms" to (System.currentTimeMillis() - startedAt)
                        )
                    )
                }
            } catch (e: Exception) {
                DebugTraceStore.record(
                    "ui_prewarm_failed",
                    mapOf(
                        "reason" to reason,
                        "project_id" to project.id,
                        "conversation_id" to conversation.id,
                        "error" to e.message
                    )
                )
            } finally {
                synchronized(prewarmLock) {
                    prewarmingConversationKeys.remove(key)
                }
            }
        }.start()
    }

    private companion object {
        const val PREWARM_COOLDOWN_MS = 120_000L
    }
}
