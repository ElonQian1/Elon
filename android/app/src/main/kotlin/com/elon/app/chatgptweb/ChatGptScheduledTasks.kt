package com.elon.app.chatgptweb

import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONObject
import java.util.UUID

internal suspend fun ChatGptBackgroundSession.scheduledTaskRequest(input: JSONObject): JSONObject {
    prepareScheduledTasks()
    return withTimeout(30_000) {
        while (scheduledTaskClient() == null) kotlinx.coroutines.delay(150)
        requireNotNull(scheduledTaskClient()).request(input)
    }
}

/** Private payloads never enter the MCP observed-state or command-detail ledger. */
internal class ChatGptScheduledTasks(private val send: (String, String) -> Unit) {
    private val pending = mutableMapOf<String, CompletableDeferred<JSONObject>>()
    private val serial = Mutex()
    suspend fun request(input: JSONObject): JSONObject = serial.withLock {
        requestOnce(input)
    }
    private suspend fun requestOnce(input: JSONObject): JSONObject {
        val id = "mcp_" + UUID.randomUUID().toString().replace("-", "")
        val result = CompletableDeferred<JSONObject>()
        pending[id] = result
        try { send(input.toString(), id); return withTimeout(18_000) { result.await() } }
        finally { pending.remove(id) }
    }
    fun receive(raw: String, version: Int, accept: (String) -> Boolean): Boolean {
        val value = runCatching { JSONObject(raw) }.getOrNull() ?: return false
        val event = value.optJSONObject("event") ?: return false
        if (event.optString("type") != "scheduled_tasks") return false
        val task = pending[event.optString("requestId")] ?: return true
        if (raw.length > 1_100_000 || value.optInt("adapterVersion") < version ||
            value.optString("schema") != ChatGptWebProtocol.SCHEMA || !accept(value.optString("documentToken"))) return true
        if (!Regex("[a-f0-9]{64}").matches(event.optString("scope"))) return true
        val response = event.optJSONObject("result") ?: return true
        if (!response.optBoolean("ok")) task.completeExceptionally(IllegalStateException(response.optString("code")))
        else runCatching { JSONObject().put("scope", event.getString("scope")).put("data", response.getJSONObject("data")) }
            .onSuccess { task.complete(it) }
            .onFailure { task.completeExceptionally(IllegalStateException("tasks_response_invalid")) }
        return true
    }
    fun failure(event: ChatGptWebEvent) {
        if (event is ChatGptWebEvent.CommandResult && event.action == "scheduled_tasks" && !event.ok)
            pending[event.requestId]?.completeExceptionally(IllegalStateException(event.detail))
    }
    fun close() { pending.values.forEach { it.cancel() }; pending.clear() }
}
