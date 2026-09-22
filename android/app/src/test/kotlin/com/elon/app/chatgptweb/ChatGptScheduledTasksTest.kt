package com.elon.app.chatgptweb

import kotlinx.coroutines.CoroutineStart
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptScheduledTasksTest {
    private fun envelope(id: String, data: Any = JSONObject().put("items", org.json.JSONArray())) =
        JSONObject().put("schema", ChatGptWebProtocol.SCHEMA).put("adapterVersion", 434)
            .put("documentToken", "doc_current").put("event", JSONObject().put("type", "scheduled_tasks")
                .put("requestId", id).put("scope", "a".repeat(64)).put("result", JSONObject()
                    .put("ok", true).put("data", data)))

    @Test fun deliversDedicatedPayloadOnlyToMatchingRequestAndDocument() = runBlocking {
        var id = ""
        val bridge = ChatGptScheduledTasks { _, request -> id = request }
        val pending = async(start = CoroutineStart.UNDISPATCHED) { bridge.request(JSONObject()) }
        assertTrue(bridge.receive(envelope("mcp_other").toString(), 434) { true })
        bridge.receive(envelope(id).toString(), 434) { false }
        assertFalse(pending.isCompleted)
        bridge.receive(envelope(id).toString(), 434) { it == "doc_current" }
        assertEquals("a".repeat(64), pending.await().getString("scope"))
        assertFalse(bridge.receive("{}", 434) { true })
    }

    @Test fun malformedPayloadFailsSafelyAndDoesNotCrashWebMessageListener() = runBlocking {
        var id = ""
        val bridge = ChatGptScheduledTasks { _, request -> id = request }
        val pending = async(start = CoroutineStart.UNDISPATCHED) { runCatching { bridge.request(JSONObject()) } }
        assertTrue(bridge.receive(envelope(id, "invalid").toString(), 434) { true })
        assertEquals("tasks_response_invalid", pending.await().exceptionOrNull()?.message)
    }

    @Test fun closeCancelsWaitAndAllowsNextDocument() = runBlocking {
        var id = ""
        val bridge = ChatGptScheduledTasks { _, request -> id = request }
        val pending = async(start = CoroutineStart.UNDISPATCHED) { runCatching { bridge.request(JSONObject()) } }
        bridge.close(); assertTrue(pending.await().isFailure)
        val next = async(start = CoroutineStart.UNDISPATCHED) { bridge.request(JSONObject()) }
        bridge.receive(envelope(id).toString(), 434) { true }
        assertTrue(next.await().has("data"))
    }
}
