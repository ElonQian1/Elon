package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebCanvasSharedLinksTest {
    private val ticket = "sl_" + "17".repeat(16) + "_1"
    private val path = "/c/11111111-1111-4111-8111-111111111111"
    private fun row(id: Any = "canvas_fixture-A1", source: Any = JSONObject.NULL) = JSONObject()
        .put("id", id).put("path", source).put("createdAt", "2026-09-11T12:00:00Z")
    private fun payload(rows: JSONArray = JSONArray().put(row())) = JSONObject()
        .put("schema", "elon.canvas_shares.v1").put("ticket", ticket).put("complete", true)
        .put("offset", 0).put("nextOffset", JSONObject.NULL).put("items", rows)

    @Test fun canvasReceiptKeepsResourceAndUrlWithoutRequiringSourceConversation() {
        val value = payload(JSONArray().put(row()).put(row("second", path))).toString()
        val index = requireNotNull(ChatGptWebSharedLinks.parseAccount(value))
        assertEquals(ChatGptWebSharedLinks.Resource.CANVAS, index.resource)
        assertNull(index.items[0].path)
        assertEquals(path, index.items[1].path)
        assertEquals("https://chatgpt.com/canvas/shared/canvas_fixture-A1", index.items[0].url)
        assertEquals(value, ChatGptWebPrivateProtocolEvidence.detail("share_conversation", value))
        assertNull(ChatGptWebSharedLinks.parse(value))
        assertNull(ChatGptWebConversationShareReceipt.parse(value))
        assertNull(ChatGptWebSharedLinks.parseAccount(payload().put("schema", "elon.account_shares.v1").toString()))
    }

    @Test fun rejectsUnsafeOrCoercedIdsAndMetadata() {
        for (id in listOf<Any>(123, JSONObject.NULL, "", "a/b", "../id", "id?x", "a%2Fb", "a".repeat(129))) {
            assertNull(ChatGptWebSharedLinks.parseAccount(payload(JSONArray().put(row(id))).toString()))
        }
        for (source in listOf<Any>(123, "/c/fixture", "https://chatgpt.com$path", "/c/11111111-1111-4111-8111-111111111111?x=1")) {
            assertNull(ChatGptWebSharedLinks.parseAccount(payload(JSONArray().put(row(source = source))).toString()))
        }
        assertNull(ChatGptWebSharedLinks.parseAccount(payload(JSONArray().put(row()).put(row())).toString()))
        assertNull(ChatGptWebSharedLinks.parseAccount(payload(JSONArray().put(row().put("name", "fixture"))).toString()))
        assertNull(ChatGptWebSharedLinks.parseAccount(payload(JSONArray().put(row().put("createdAt", "invalid"))).toString()))
        assertNull(ChatGptWebSharedLinks.parseAccount(payload().put("nextOffset", 100).toString()))
    }

    @Test fun fullCanvasPageSurvivesTheActualCommandReceiptBoundary() {
        val rows = JSONArray()
        repeat(100) { rows.put(row(it.toString().padStart(128, 'a'), path)) }
        val value = payload(rows).put("nextOffset", 100).toString()
        assertTrue(value.length > 18000)
        assertTrue(value.length < 32000)
        assertEquals(value, ChatGptWebPrivateProtocolEvidence.detail("share_conversation", value))
        val index = requireNotNull(ChatGptWebSharedLinks.parseAccount(value))
        assertEquals(100, index.items.size)
        assertEquals(100, index.nextOffset)
        assertNull(ChatGptWebSharedLinks.parseAccount(payload(rows.put(row("extra"))).toString()))
        assertEquals("share_list_unconfirmed", ChatGptWebSharedLinks.detail(value + " ".repeat(32000)))
    }

    @Test fun canvasCommandsUseTheirOwnResourceAndDoNotInventAConversationPath() {
        val list = requireNotNull(ChatGptWebSharedLinks.accountRequest(0, null, canvas = true))
        assertEquals("canvas", list.getString("resource"))
        assertFalse(list.has("path"))
        assertFalse(ChatGptWebSharedLinks.accountRequest(0, null)!!.has("resource"))
        assertNull(ChatGptWebSharedLinks.accountRequest(100, null, canvas = true))
        assertNotNull(ChatGptWebSharedLinks.accountRequest(100, ticket, canvas = true))
        val revoke = requireNotNull(ChatGptWebSharedLinks.canvasRevokeRequest("fixture", ticket))
        assertEquals("revoke_account", revoke.getString("operation"))
        assertEquals("canvas", revoke.getString("resource"))
        assertFalse(revoke.has("path"))
        assertNull(ChatGptWebSharedLinks.canvasRevokeRequest("../fixture", ticket))
        assertNull(ChatGptWebSharedLinks.canvasRevokeRequest("fixture", "expired"))
    }
}
