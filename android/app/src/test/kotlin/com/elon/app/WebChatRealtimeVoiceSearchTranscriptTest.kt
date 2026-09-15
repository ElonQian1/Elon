package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCapabilities
import com.elon.app.chatgptweb.ChatGptWebNativeVoiceTranscriptDecoder
import com.elon.app.chatgptweb.ChatGptWebSnapshot
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceSearchTranscriptTest {
    @Test
    fun highChannelSearchResultContinuesIntoNativeBubbleWithoutHistoryRefresh() {
        val decoder = ChatGptWebNativeVoiceTranscriptDecoder()
        val continuity = WebChatRealtimeVoiceTranscriptContinuity()
        continuity.begin(ChatGptWebSnapshot(
            title = "Synthetic voice fixture", url = "https://chatgpt.com/c/fixture",
            draft = "", messages = emptyList(), authenticated = true, composerReady = true,
            streaming = false, currentModel = "", attachments = emptyList(), dictationActive = false,
            capabilities = ChatGptWebCapabilities.EMPTY, pageKind = "conversation",
        ))
        repeat(15) { decoder.decode(root(it, "assistant", "earlier")) }
        continuity.applyLive(requireNotNull(decoder.decode(root(15, "assistant", "Searching."))))
        assertNull(decoder.decode(root(16, "tool", "Internal search result")))
        var snapshot = continuity.applyLive(requireNotNull(decoder.decode(root(17, "assistant", "Result"))))
        assertEquals("Result", snapshot?.messages?.last()?.content)
        snapshot = continuity.applyLive(requireNotNull(decoder.decode(event(
            JSONObject().put("o", "append").put("p", "/message/content/parts/0").put("v", " continues"),
        ))))
        assertEquals("Result continues", snapshot?.messages?.last()?.content)
        assertTrue(snapshot?.streaming == true)
        snapshot = continuity.applyLive(requireNotNull(decoder.decode(event(
            JSONObject().put("o", "replace").put("p", "/message/status").put("v", "finished_successfully"),
        ))))
        assertEquals("completed", snapshot?.messages?.last()?.state)
        assertEquals(listOf("Searching.", "Result continues"), snapshot?.messages?.map { it.content })
        assertFalse(snapshot?.messages?.any { it.role == "tool" } == true)
    }

    private fun root(channel: Int, role: String, text: String): String = event(
        JSONObject().put("c", channel).put("o", "add").put("p", "").put("v",
            JSONObject().put("message", JSONObject()
                .put("id", "message-$channel")
                .put("author", JSONObject().put("role", role))
                .put("status", if (channel == 15) "finished_successfully" else "in_progress")
                .put("content", JSONObject().put("content_type", "text").put("parts", JSONArray().put(text))),
            ),
        ),
    )

    private fun event(delta: JSONObject): String = JSONObject()
        .put("type", "chat_message_delta")
        .put("payload", JSONObject().put("delta", delta)).toString()
}
