package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class GroupWebAiSendPreparationTest {
    private class Harness {
        var now = 0L
        val tasks = mutableListOf<Pair<Long, () -> Unit>>()
        val ids = mutableListOf<String>()
        val observations = mutableListOf<Map<String, Any?>>()
        var ready = 0
        val preparation = GroupWebAiSendPreparation(ids::add, { delay, read -> tasks += now + delay to read },
            { ready++ }, observations::add)
        fun next() {
            val task = tasks.minBy { it.first }
            tasks.remove(task); now = task.first; task.second()
        }
        fun drain() { repeat(30) { if (tasks.isNotEmpty()) next() } }
        fun result(code: String = "ready", stage: String = "ready", ok: Boolean = code == "ready", id: String = ids.last()) =
            preparation.event(ChatGptWebEvent.CommandResult("private_protocol_probe", ok,
                JSONObject().put("schema", "elon.fresh_text_admission.v1").put("code", code).put("stage", stage).toString(), id))
    }

    @Test fun doesNotAuthorizeOnComposerPresenceAndCoalescesStarts() {
        val h = Harness()
        repeat(5) { h.preparation.start() }
        assertEquals(1, h.ids.size)
        assertTrue(Regex("mcp_[a-z0-9]{1,32}").matches(h.ids.single()))
        assertEquals(0, h.ready)
        h.result()
        h.drain()
        assertEquals(1, h.ready)
        assertEquals(1, h.ids.size)
        assertEquals("private_send_prepared", h.observations.last()["stage"])
    }

    @Test fun coldOwnerIsReadAgainWithoutWritingOrReusingCommandIds() {
        val h = Harness(); h.preparation.start()
        h.result("context_unavailable", "base_owner")
        assertEquals(0, h.ready)
        h.next()
        assertEquals(500L, h.now)
        assertEquals(2, h.ids.distinct().size)
        h.result()
        h.drain()
        assertEquals(1, h.ready)
    }

    @Test fun absentReceiptsHaveBoundedDeadlineAndRetainOfficialFallback() {
        val h = Harness(); h.preparation.start(); h.drain()
        assertEquals(5, h.ids.size)
        assertEquals(1, h.ready)
        assertTrue(h.now <= 35_000)
        assertEquals("official_send_fallback", h.observations.last()["stage"])
    }

    @Test fun staleWrongActionAndRepeatedReceiptsCannotReleaseNewAttempt() {
        val h = Harness(); h.preparation.start()
        val old = h.ids.single()
        h.next(); h.next()
        h.result(id = old)
        h.preparation.event(ChatGptWebEvent.CommandResult("send_prompt", true, "", h.ids.last()))
        assertEquals(0, h.ready)
        h.result(); h.result(); h.drain()
        assertEquals(1, h.ready)
    }

    @Test fun cancellationStopsQueuedAndLateReads() {
        val h = Harness(); h.preparation.start()
        h.result("runtime_unavailable", "base_context")
        h.preparation.close(); h.result(); h.drain(); h.preparation.start()
        assertEquals(1, h.ids.size)
        assertEquals(0, h.ready)
    }

    @Test fun contradictoryAndUnsanitizedResponsesNeverClaimPrivateReadiness() {
        val h = Harness(); h.preparation.start()
        h.result(ok = false)
        h.next()
        h.result(stage = "base_owner")
        h.next()
        h.result(code = "private_user_content", stage = "private_user_content", ok = true)
        assertEquals(0, h.ready)
        assertFalse(h.observations.toString().contains("private_user_content"))
        assertEquals("unavailable", h.observations.last()["code"])
    }

    @Test fun actualBaseModelStageRemainsObservable() {
        val h = Harness(); h.preparation.start()
        h.result("context_unavailable", "base_model")
        assertEquals("base_model", h.observations.last()["admission_stage"])
        assertEquals(0, h.ready)
    }
}
