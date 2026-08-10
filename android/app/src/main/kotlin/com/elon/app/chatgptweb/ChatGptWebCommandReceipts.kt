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
        .put("result", resultJson(value.result))

    fun lastResultJson(value: ChatGptWebObservedState.Snapshot): Any {
        val command = value.lastCommand ?: return JSONObject.NULL
        val request = value.commandRequests.lastOrNull { it.result == command }
        return resultJson(command, request?.id)
    }

    private fun resultJson(
        value: ChatGptWebEvent.CommandResult?,
        requestId: String? = null,
    ): Any {
        if (value == null) return JSONObject.NULL
        return JSONObject()
            .put("action", value.action)
            .put("ok", value.ok)
            .put("detail", value.detail)
            .put("request_id", requestId ?: JSONObject.NULL)
    }
}
