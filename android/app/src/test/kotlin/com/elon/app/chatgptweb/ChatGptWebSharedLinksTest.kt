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
    private fun accountPayload() = payload().apply {
        put("schema", "elon.account_shares.v1").put("offset", 0).put("nextOffset", JSONObject.NULL)
        remove("path")
        getJSONArray("items").getJSONObject(0).put("path", path)
    }

    @Test fun accountReceiptRetainsOnlySourceBoundRowsAndDistinctSchema() {
        val raw = accountPayload().toString()
        assertEquals(raw, ChatGptWebConversationShareReceipt.detail(raw))
        val page = requireNotNull(ChatGptWebSharedLinks.parseAccount(raw))
        assertEquals(path, page.items.single().path)
        assertEquals("https://chatgpt.com/share/$id", page.items.single().url)
        assertNull(page.nextOffset)
        assertNull(ChatGptWebSharedLinks.parse(raw))
        assertNull(ChatGptWebSharedLinks.parseAccount(payload().toString()))
    }

    @Test fun accountCursorNeedsAnExistingSelectionAndStrictPageBoundaries() {
        assertNotNull(ChatGptWebSharedLinks.accountRequest(0, null))
        assertNotNull(ChatGptWebSharedLinks.accountRequest(100, ticket))
        assertNotNull(ChatGptWebSharedLinks.accountRequest(0, ticket))
        for (offset in listOf(-100, 1, 1000)) assertNull(ChatGptWebSharedLinks.accountRequest(offset, ticket))
        assertNull(ChatGptWebSharedLinks.accountRequest(100, null))
        assertNull(ChatGptWebSharedLinks.accountRequest(0, "unbound"))
        val rows = JSONArray()
        repeat(100) { position -> rows.put(JSONObject().put("id", "aaaaaaaa-aaaa-4aaa-8aaa-%012x".format(position))
            .put("path", path).put("createdAt", JSONObject.NULL)) }
        val raw = accountPayload().put("items", rows).put("nextOffset", 100).toString()
        assertEquals(100, requireNotNull(ChatGptWebSharedLinks.parseAccount(raw)).nextOffset)
        assertTrue(raw.length <= 18000)
    }

    @Test fun accountReceiptRejectsForeignPathsMetadataAndInventedPagination() {
        val invalid = listOf(
            accountPayload().put("offset", "0"), accountPayload().put("offset", 1),
            accountPayload().put("offset", 1000), accountPayload().put("nextOffset", 100),
            accountPayload().put("offset", 100).put("items", JSONArray()),
            accountPayload().apply { getJSONArray("items").getJSONObject(0).put("path", "https://example.com$path") },
            accountPayload().apply { getJSONArray("items").getJSONObject(0).put("title", "not a receipt field") },
            accountPayload().put("workspace_id", "other"),
        )
        invalid.forEach {
            assertNull(ChatGptWebSharedLinks.parseAccount(it.toString()))
            assertEquals("share_list_unconfirmed", ChatGptWebSharedLinks.detail(it.toString()))
        }
    }

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
