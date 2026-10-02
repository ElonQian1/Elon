package com.elon.app

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class MessageTimelineWindowTest {
    @Test fun bookmarkPagesMoveBothWaysWithoutFollowingOrReadingSkippedHistory() {
        fun v2(start: Int) = page(start, 50, true).put("schema", "elon.message_timeline.v2").put("has_older", true).put("has_newer", true).also { p ->
            repeat(50) { i -> p.getJSONArray("messages").getJSONObject(i).put("timeline_after_cursor", "a${start + i}") }
        }
        val window = MessageTimelineWindow(maxMessages = 50)
        window.locate(mapOf("bookmark" to "bookmark-test", "resume" to "true"))
        assertTrue(window.path("group:g", "around").contains("/v2?"))
        window.accept(v2(100), "around")
        val forward = window.accept(v2(150).put("has_newer", false), "newer")
        assertEquals("000150", forward.getJSONObject(0).getString("id"))
        assertTrue(window.path("group:g", "older").contains("before=c150"))
        assertTrue(window.path("group:g", "newer").contains("after=a199"))
        window.following = true
        assertFalse(window.following)
        assertNull(window.readReceipt("group:g"))
        window.latest(); assertFalse(window.following)
        window.accept(v2(950).put("has_newer", false), "latest"); assertTrue(window.following)
    }
    private fun row(n: Int) = JSONObject().put("id", n.toString().padStart(6, '0')).put("created_at", "2026-10-01")
        .put("content", "消息").put("timeline_cursor", "c$n")
    private fun page(start: Int, count: Int, more: Boolean = false) = JSONObject().put("schema", "elon.message_timeline.v1")
        .put("messages", JSONArray((start until start + count).map(::row))).put("removed_ids", JSONArray())
        .put("sync", "s${start + count}").put("has_more", more)

    @Test fun expiredCursorRevalidatesTheHistoricalWindowWithoutJumpingToLatest() {
        val window = MessageTimelineWindow()
        window.accept(page(20, 2, true), "latest")
        window.following = false
        window.accept(page(0, 0).put("reset", true), "sync")
        assertEquals("window", window.direction())
        assertEquals(2, window.windowRequest("group:g").getJSONArray("message_ids").length())
        val recovered = window.accept(page(20, 1).put("removed_ids", JSONArray().put("000021")), "window")
        assertEquals("000020", recovered.getJSONObject(0).getString("id"))
        assertEquals(1, recovered.length())
        assertTrue(window.hasOlder)
        assertTrue(window.hasNewer)
        assertNull(window.readReceipt("group:g"))
    }

    @Test fun continuousTrafficIsBoundedAndReceiptDoesNotReadUnloadedHistory() {
        val window = MessageTimelineWindow()
        var rows = JSONArray()
        repeat(200) { n -> rows = window.accept(page(n * 50, 50), if (n == 0) "latest" else "sync") }
        assertEquals(150, rows.length())
        assertEquals("009850", rows.getJSONObject(0).getString("id"))
        assertTrue(window.path("group:g", "older").contains("before=c9850"))
        assertEquals("009999", window.readReceipt("group:g")!!.getString("message_id"))
        window.older()
        assertNull(window.readReceipt("group:g"))
        val history = window.accept(page(9800, 50, true), "older")
        assertEquals("009800", history.getJSONObject(0).getString("id"))
        assertTrue(window.hasNewer)
        val frozen = window.accept(page(10000, 50), "sync")
        assertEquals(history.toString(), frozen.toString())
        window.latest()
        assertEquals(50, window.accept(page(10000, 50), "latest").length())
        assertFalse(window.hasNewer)
    }

    @Test fun duplicateEditRecallDeleteAndResetDoNotReintroduceStaleContent() {
        val window = MessageTimelineWindow()
        window.accept(page(0, 2), "latest")
        val edited = page(0, 2)
        edited.getJSONArray("messages").getJSONObject(0).put("revision", 3).put("content", "edited")
        edited.getJSONArray("messages").getJSONObject(1).put("recalled_at", "now").put("content", "")
        window.accept(edited, "sync")
        val replay = window.accept(page(0, 2), "sync")
        assertEquals(3, replay.getJSONObject(0).getInt("revision"))
        assertEquals("", replay.getJSONObject(1).getString("content"))
        val removed = window.accept(page(0, 0).put("removed_ids", JSONArray().put("000000")), "sync")
        assertEquals(1, removed.length())
        window.accept(page(0, 0).put("reset", true), "sync")
        assertEquals("latest", window.direction())
        assertTrue(window.moreChanges)
    }
}
