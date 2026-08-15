package com.elon.app

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionContextPagerTest {
    @Test
    fun pagesProviderNeutralProductionMessagesWithStableCursors() {
        val messages = listOf(message("one", "user"), message("two", "friend"), message("three", "friend"))
        val first = page(messages, JSONObject().put("message_limit", 2))

        assertTrue(first.getBoolean("control_ok"))
        assertEquals(WebChatProductionContextPager.SCHEMA, first.getString("schema"))
        assertEquals("google_web", first.getString("provider_id"))
        assertEquals(3, first.getInt("message_count"))
        assertEquals(2, first.getJSONArray("messages").length())
        assertTrue(first.getBoolean("has_more"))

        val second = page(messages, JSONObject()
            .put("message_limit", 2)
            .put("message_cursor", first.getString("next_message_cursor")))
        assertEquals(2, second.getInt("message_offset"))
        assertEquals("three", second.getJSONArray("messages").getJSONObject(0).getString("content"))
        assertFalse(second.getBoolean("has_more"))
    }

    @Test
    fun rejectsMalformedAndStaleCursorsWithoutLeakingConversationText() {
        val original = listOf(message("private original", "user"))
        val cursor = page(original, JSONObject()).getString("message_cursor")

        val invalid = page(original, JSONObject().put("message_cursor", "not-a-cursor"))
        assertFalse(invalid.getBoolean("control_ok"))
        assertEquals("invalid_message_cursor", invalid.getString("error"))
        assertFalse(invalid.toString().contains("private original"))

        val stale = page(listOf(message("changed", "user")), JSONObject().put("message_cursor", cursor))
        assertFalse(stale.getBoolean("control_ok"))
        assertEquals("stale_message_cursor", stale.getString("error"))
        assertEquals(0, stale.getInt("retry_from_message_offset"))
    }

    @Test
    fun returnsBoundedRichMetadataWithoutLocalPathsOrRemoteUrls() {
        val attachment = ChatAttachment(
            kind = "file",
            displayName = "notes.txt",
            mimeType = "text/plain",
            url = "https://private.example/file",
            localPath = "C:/private/notes.txt",
            sizeBytes = 42,
        )
        val message = message("answer", "friend").copy(
            attachments = listOf(attachment),
            webChatMessage = WebChatProductionMessage(
                providerWireValue = "google_web",
                sourceMessageId = "source-1",
                actions = setOf(WebChatMessageAction.COPY),
                renderMarkdown = true,
                contentParts = listOf(WebChatProductionContentPart("citation", "Source")),
            ),
        )

        val encoded = page(listOf(message), JSONObject()).getJSONArray("messages").getJSONObject(0)
        assertEquals(1, encoded.getInt("part_count"))
        assertEquals(1, encoded.getInt("attachment_count"))
        assertEquals("notes.txt", encoded.getJSONArray("attachments").getJSONObject(0).getString("display_name"))
        assertFalse(encoded.toString().contains("private.example"))
        assertFalse(encoded.toString().contains("C:/private"))
    }

    private fun page(messages: List<ChatMessage>, args: JSONObject): JSONObject =
        WebChatProductionContextPager.page(
            providerId = WebChatProviderId.GOOGLE_WEB,
            conversationPath = "/google-ai-mode/conversation/demo",
            model = "AI Mode",
            state = "ready",
            streaming = false,
            messages = messages,
            args = args,
        )

    private fun message(content: String, role: String) = ChatMessage(
        role = role,
        content = content,
        id = "id-$content",
        createdAtMs = 1_000L,
    )
}
