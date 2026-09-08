package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebAttachmentProtocolTest {
    private val upload = "private_attachment_00000000-0000-4000-8000-000000000001"
    private val library = "private_attachment_mcp_library1"

    private fun parse(vararg ids: String): List<ChatGptWebAttachment> {
        val event = JSONObject().put("type", "message_snapshot")
            .put("url", "https://chatgpt.com/")
            .put("attachments", JSONArray(ids.map { id ->
                JSONObject().put("id", id).put("name", "fixture.txt")
                    .put("state", "ready").put("removable", true)
            }))
        val message = JSONObject().put("schema", "yilong.ai.ui.v1").put("event", event)
        return (ChatGptWebProtocol.parse(message.toString()) as ChatGptWebEvent.Snapshot).value.attachments
    }

    @Test fun retainsPrivateUploadIdentifiersAndMetadata() {
        val item = parse(upload).single()
        assertEquals(upload, item.id)
        assertEquals("fixture.txt", item.name)
        assertTrue(item.removable)
    }

    @Test fun retainsLibraryRequestIdentifiersAndLegacyDomEntries() {
        assertEquals(listOf(library, "attachment_ab12"), parse(library, "attachment_ab12").map { it.id })
    }

    @Test fun acceptsMaximumLengthLibraryRequestWithoutChangingItsId() {
        val id = "private_attachment_mcp_" + "a".repeat(32)
        assertEquals(id, parse(id).single().id)
    }

    @Test fun rejectsUnknownMalformedAndOversizedIdentifiersInsteadOfTruncating() {
        val invalid = arrayOf("../attachment_ab12", "private_attachment_other", upload + "a",
            "private_attachment_mcp_", "private_attachment_mcp_" + "a".repeat(33),
            "private_attachment_" + "-".repeat(36), "attachment_" + "a".repeat(90),
            "private_attachment_mcp_a/other", "private_attachment_mcp_a\n")
        assertTrue(parse(*invalid).isEmpty())
    }
}
