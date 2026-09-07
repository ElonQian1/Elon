package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebSharedLinksTest {
    private val id = "44444444-4444-4444-8444-444444444444"
    private val path = "/c/11111111-1111-4111-8111-111111111111"
    private val ticket = "sl_" + "17".repeat(16) + "_1"
    private fun payload() = JSONObject().put("schema", "elon.conversation_shares.v1")
        .put("path", path).put("ticket", ticket).put("complete", true)
        .put("items", JSONArray().put(JSONObject().put("id", id).put("createdAt", "2026-09-01T10:00:00Z")))

    @Test fun resultSurvivesTheActualShareReceiptSanitizer() {
        val raw = payload().toString()
        assertTrue(raw.length > 160)
        assertEquals(raw, ChatGptWebConversationShareReceipt.detail(raw))
        val parsed = requireNotNull(ChatGptWebSharedLinks.parse(raw))
        assertEquals(path, parsed.path)
        assertEquals("https://chatgpt.com/share/$id", parsed.items.single().url)
        assertTrue(parsed.complete)
        assertEquals("share_link_revoked", ChatGptWebConversationShareReceipt.detail("share_link_revoked"))
        assertEquals("share_revoke_unconfirmed", ChatGptWebConversationShareReceipt.detail("share_revoke_unconfirmed"))
    }

    @Test fun partialAndEmptyStayDistinct() {
        val data = payload().put("items", JSONArray()).put("complete", false)
        val result = requireNotNull(ChatGptWebSharedLinks.parse(data.toString()))
        assertFalse(result.complete)
        assertTrue(result.items.isEmpty())
        assertNull(ChatGptWebSharedLinks.parse("{}"))
    }

    @Test fun rejectsUnknownKeysUrlsDuplicateIdsAndBadDates() {
        val invalid = listOf(
            payload().put("headers", "not allowed"),
            payload().put("path", "https://example.com$path"),
            payload().put("ticket", "unbound"),
            payload().put("complete", "true"),
            payload().apply { getJSONArray("items").put(getJSONArray("items").getJSONObject(0)) },
            payload().apply { getJSONArray("items").getJSONObject(0).put("id", "../all") },
            payload().apply { getJSONArray("items").getJSONObject(0).put("createdAt", "yesterday") },
            payload().apply { getJSONArray("items").getJSONObject(0).put("url", "https://example.com") },
        )
        invalid.forEach {
            assertNull(ChatGptWebSharedLinks.parse(it.toString()))
            assertEquals("share_list_unconfirmed", ChatGptWebConversationShareReceipt.detail(it.toString()))
        }
    }

    @Test fun commandKeepsListReadOnlyAndBindsOneSelectedLink() {
        val list = requireNotNull(ChatGptWebSharedLinks.request(path, "list", null, null))
        assertEquals(setOf("operation", "path"), list.keys().asSequence().toSet())
        val revoke = requireNotNull(ChatGptWebSharedLinks.request(path, "revoke", id, ticket))
        assertEquals(id, revoke.getString("id"))
        assertEquals(ticket, revoke.getString("ticket"))
        assertNull(ChatGptWebSharedLinks.request(path, "revoke", id, null))
        assertNull(ChatGptWebSharedLinks.request(path, "revoke", "all", ticket))
        assertNull(ChatGptWebSharedLinks.request(path, "delete_all", null, null))
        assertEquals(path, ChatGptWebSharedLinks.path("/g/g-p-${"a".repeat(32)}$path"))
    }
}
