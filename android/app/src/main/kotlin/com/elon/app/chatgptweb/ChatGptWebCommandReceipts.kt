package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebCommandReceipts {
    fun requestsJson(value: ChatGptWebObservedState.Snapshot): JSONArray = JSONArray().apply {
        value.commandRequests.forEach { put(requestJson(it)) }
    }

    fun requestJson(value: ChatGptWebObservedState.CommandRequest): JSONObject = JSONObject()
        .put("request_id", value.id)
        .put("expected_web_action", value.expectedAction)
        .put("status", value.status)
        .put("started_at_ms", value.startedAtMs)
        .put("completed_at_ms", value.completedAtMs ?: JSONObject.NULL)
        .put("result", resultJson(value.result, observedAtMs = value.completedAtMs))

    fun lastResultJson(value: ChatGptWebObservedState.Snapshot): Any {
        val command = value.lastCommand ?: return JSONObject.NULL
        val request = value.commandRequests.lastOrNull { it.result == command }
        return resultJson(command, request?.id, value.lastCommandObservedAtMs)
    }

    fun recentResultJson(value: ChatGptWebObservedState.Snapshot, action: String): Any {
        val observed = value.recentCommandResults[action] ?: return JSONObject.NULL
        val request = value.commandRequests.lastOrNull { it.result == observed.result }
        return resultJson(observed.result, request?.id, observed.observedAtMs)
    }

    private fun resultJson(
        value: ChatGptWebEvent.CommandResult?,
        requestId: String? = null,
        observedAtMs: Long? = null,
    ): Any {
        if (value == null) return JSONObject.NULL
        return JSONObject()
            .put("action", value.action)
            .put("ok", value.ok)
            .put("detail", value.detail)
            .put("request_id", requestId ?: value.requestId ?: JSONObject.NULL)
            .put("observed_at_ms", observedAtMs ?: JSONObject.NULL)
    }
}
