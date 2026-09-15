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
    fun rejectsUnknownOperationsAndInvalidChannels() {
        val decoder = ChatGptWebNativeVoiceJsonDeltaDecoder()

        assertNull(decoder.apply(JSONObject().put("o", "execute").put("p", "")))
        assertNull(
            decoder.apply(
                JSONObject()
                    .put("c", -1)
                    .put("o", "add")
                    .put("p", "")
                    .put("v", "blocked"),
            ),
        )
    }

    @Test
    fun continuesAfterSixteenMessagesAndSearchToolChannels() {
        val decoder = ChatGptWebNativeVoiceJsonDeltaDecoder()
        repeat(40) { channel ->
            val initial = decoder.apply(root(channel, "a")) as JSONObject
            assertEquals("a", text(initial))
            val appended = decoder.apply(
                JSONObject().put("o", "append").put("p", "/message/content/parts/0").put("v", "b"),
            ) as JSONObject
            assertEquals("ab", text(appended))
        }
        decoder.apply(root(10001, "after search"))
        val appended = decoder.apply(
            JSONObject().put("o", "append").put("p", "/message/content/parts/0").put("v", " continues"),
        ) as JSONObject
        assertEquals("after search continues", text(appended))
        assertEquals(10001, decoder.diagnostics().highestChannel)
        assertEquals(16, decoder.diagnostics().cachedChannels)
        assertEquals(0, decoder.diagnostics().rejectedCount)
    }

    @Test
    fun cacheIsBoundedButRecentlyUpdatedChannelsSurvive() {
        val decoder = ChatGptWebNativeVoiceJsonDeltaDecoder()
        repeat(16) { decoder.apply(root(it, "a")) }
        fun append(channel: Int) = decoder.apply(
            JSONObject().put("c", channel).put("o", "append")
                .put("p", "/message/content/parts/0").put("v", "b"),
        )
        append(0)
        decoder.apply(root(16, "new"))
        assertEquals("abb", text(append(0) as JSONObject))
        // Missing bases cannot turn a late tool patch into a partial new message.
        assertNull(append(1))
        assertEquals("missing_channel_base", decoder.diagnostics().lastRejection)
        assertEquals(1, decoder.diagnostics().rejectedCount)
        decoder.apply(root(1, "restored"))
        assertEquals("restoredb", text(append(1) as JSONObject))
    }

    @Test
    fun omittedValueNeverReplaysThePreviousSubtitleChunk() {
        val decoder = ChatGptWebNativeVoiceJsonDeltaDecoder()
        decoder.apply(root(0, "a"))
        decoder.apply(JSONObject().put("o", "append").put("p", "/message/content/parts/0").put("v", "b"))
        assertNull(decoder.apply(JSONObject()))
        assertEquals("abc", text(decoder.apply(JSONObject().put("v", "c")) as JSONObject))
    }

    @Test
    fun rejectsFractionalOverflowAndNonNumericChannelWithoutChangingCurrentChannel() {
        val decoder = ChatGptWebNativeVoiceJsonDeltaDecoder()
        decoder.apply(root(Int.MAX_VALUE, "a"))
        listOf<Any>(0.5, 4294967296L, "1", JSONObject.NULL).forEach { invalid ->
            assertNull(decoder.apply(root(0, "wrong").put("c", invalid)))
        }
        val current = decoder.apply(
            JSONObject().put("o", "append").put("p", "/message/content/parts/0").put("v", "b"),
        ) as JSONObject
        assertEquals("ab", text(current))
    }

    @Test
    fun resetClearsDeltaDiagnostics() {
        val decoder = ChatGptWebNativeVoiceJsonDeltaDecoder()
        decoder.apply(root(100, "private text"))
        decoder.apply(JSONObject().put("c", -1))
        assertEquals(1, decoder.diagnostics().rejectedCount)
        decoder.reset()
        assertEquals(ChatGptWebNativeVoiceDeltaDiagnostics(), decoder.diagnostics())
    }

    private fun root(channel: Int, value: String): JSONObject = JSONObject()
        .put("c", channel).put("o", "add").put("p", "")
        .put("v", JSONObject().put("message", message(value)))

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
