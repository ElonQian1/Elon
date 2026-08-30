package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebNativeVoiceJsonDeltaDecoderTest {
    @Test
    fun reconstructsCompactInheritedAppendAndReplaceOperations() {
        val decoder = ChatGptWebNativeVoiceJsonDeltaDecoder()
        val initial = decoder.apply(
            JSONObject()
                .put("c", 0)
                .put("o", "add")
                .put("p", "")
                .put("v", JSONObject().put("message", message("你"))),
        ) as JSONObject
        val appended = decoder.apply(
            JSONObject()
                .put("o", "append")
                .put("p", "/message/content/parts/0")
                .put("v", "好"),
        ) as JSONObject
        val replaced = decoder.apply(
            JSONObject()
                .put("o", "replace")
                .put("p", "/message/status")
                .put("v", "finished_successfully"),
        ) as JSONObject

        assertEquals("你", text(initial))
        assertEquals("你好", text(appended))
        assertEquals("finished_successfully", replaced.getJSONObject("message").getString("status"))
    }

    @Test
    fun appliesPatchRemoveAndTruncateWithoutEscapingBounds() {
        val decoder = ChatGptWebNativeVoiceJsonDeltaDecoder()
        decoder.apply(
            JSONObject()
                .put("o", "add")
                .put("p", "")
                .put(
                    "v",
                    JSONObject()
                        .put("text", "abcdef")
                        .put("unused", true)
                        .put("items", JSONArray().put("a").put("b")),
                ),
        )
        val value = decoder.apply(
            JSONObject()
                .put("o", "patch")
                .put("p", "")
                .put(
                    "v",
                    JSONArray()
                        .put(JSONObject().put("o", "truncate").put("p", "/text").put("v", 3))
                        .put(JSONObject().put("o", "remove").put("p", "/unused")),
                ),
        ) as JSONObject

        assertEquals("abc", value.getString("text"))
        assertNull(value.opt("unused"))
        assertEquals(2, value.getJSONArray("items").length())
    }

    @Test
    fun rejectsUnknownOperationsAndOutOfRangeChannels() {
        val decoder = ChatGptWebNativeVoiceJsonDeltaDecoder()

        assertNull(decoder.apply(JSONObject().put("o", "execute").put("p", "")))
        assertNull(
            decoder.apply(
                JSONObject()
                    .put("c", 99)
                    .put("o", "add")
                    .put("p", "")
                    .put("v", "blocked"),
            ),
        )
    }

    private fun message(text: String): JSONObject = JSONObject()
        .put("id", "message_1")
        .put("author", JSONObject().put("role", "assistant"))
        .put("status", "in_progress")
        .put("content", JSONObject().put("parts", JSONArray().put(text)))

    private fun text(value: JSONObject): String = value
        .getJSONObject("message")
        .getJSONObject("content")
        .getJSONArray("parts")
        .getString(0)
}
