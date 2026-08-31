package com.elon.app.chatgptweb

import com.elon.app.WebChatProductionRichCard
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebMessageJsonTest {
    @Test
    fun emitsBoundedStructuredMetadataWithoutCredentialTargets() {
        val part = ChatGptWebMessagePart(
            type = "file",
            label = "分析结果.csv",
            metadata = ChatGptWebMessagePartMetadata(
                kind = "download",
                mediaType = "text/csv",
                targetKind = "external",
                targetHost = "files.example.com",
            ),
        )
        val message = ChatGptWebMessageJson.encode(
            messages = listOf(ChatGptWebMessage(
                id = "a0",
                role = "assistant",
                content = "result",
                state = "completed",
                parts = listOf(part),
            )),
            startIndex = 0,
            maxChars = 30_000,
        ).getJSONObject(0)
        val encodedPart = message.getJSONArray("parts").getJSONObject(0)
        val metadata = encodedPart.getJSONObject("metadata")

        assertEquals(ChatGptWebMessageJson.METADATA_SCHEMA, metadata.getString("schema"))
        assertEquals("download", metadata.getString("kind"))
        assertEquals("text/csv", metadata.getString("media_type"))
        assertEquals("external", metadata.getString("target_kind"))
        assertEquals("files.example.com", metadata.getString("target_host"))
        assertFalse(metadata.has("url"))
        assertFalse(metadata.has("target_path"))
        assertEquals("chatgpt-message-part:a0:0:file", encodedPart.getString("native_adb_content_description"))
    }

    @Test
    fun keepsPartAndLabelBoundsVisibleToCallers() {
        val parts = (1..20).map { index ->
            ChatGptWebMessagePart("file", "x".repeat(200) + index)
        }
        val message = ChatGptWebMessageJson.encode(
            listOf(ChatGptWebMessage("a0", "assistant", "result", "completed", parts)),
            0,
            30_000,
        ).getJSONObject(0)

        assertEquals(20, message.getInt("part_count"))
        assertTrue(message.getBoolean("parts_truncated"))
        assertEquals(16, message.getJSONArray("parts").length())
        assertEquals(180, message.getJSONArray("parts").getJSONObject(0).getString("label").length)
        assertTrue(message.getJSONArray("parts").getJSONObject(0).getBoolean("label_truncated"))
    }

    @Test
    fun doesNotExportRichCardPayloadThroughTheDiagnosticMessageJson() {
        val card = WebChatProductionRichCard(
            kind = WebChatProductionRichCard.Kind.FINANCE,
            title = "private title",
            primaryValue = "private value",
        )
        val message = ChatGptWebMessageJson.encode(
            listOf(ChatGptWebMessage(
                "a0",
                "assistant",
                "result",
                "completed",
                listOf(ChatGptWebMessagePart("rich_card", "summary", richCard = card)),
            )),
            0,
            30_000,
        ).getJSONObject(0)
        val part = message.getJSONArray("parts").getJSONObject(0)

        assertEquals("rich_card", part.getString("type"))
        assertFalse(part.has("rich_content"))
        assertFalse(part.has("payload"))
    }
}
