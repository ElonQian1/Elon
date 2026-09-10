package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebDocumentStateEvidenceTest {
    private fun shape() = JSONObject().put("tag", "div").put("connected", true).put("editable", true)
        .put("width", 320).put("height", 48).put("display", "block").put("visibility", "visible")

    private fun root() = JSONObject().put("schema", "elon.document_state.v1")
        .put("ready", "complete").put("visibility", "visible").put("focused", false).put("skin", false)
        .put("viewport_width", 360).put("viewport_height", 780).put("body", shape().put("tag", "body"))
        .put("main_count", 1).put("form_count", 1).put("editable_count", 1)
        .put("prompt_count", 1).put("visible_prompt_count", 1).put("prompts", JSONArray().put(shape()))
        .put("incomplete", false)

    private fun detail(value: JSONObject) = ChatGptWebPrivateProtocolEvidence.detail("private_protocol_probe", value.toString())

    @Test fun acceptsObservedAndMissingComposerShapesWithoutContent() {
        assertTrue("document_state" in ChatGptWebPrivateProtocolEvidence.MODES)
        val value = JSONObject(detail(root()))
        assertEquals(1, value.getInt("prompt_count"))
        val missing = root().put("body", JSONObject.NULL).put("prompt_count", 0)
            .put("visible_prompt_count", 0).put("prompts", JSONArray())
        assertEquals(0, JSONObject(detail(missing)).getInt("prompt_count"))
        assertTrue(JSONObject(detail(root().put("incomplete", true))).getBoolean("incomplete"))
    }

    @Test fun rejectsUnknownTextCredentialsAndOutOfRangeOrAmbiguousTypes() {
        val invalid = listOf(root().put("cookie", "secret"), root().put("ready", "raw error"),
            root().put("viewport_width", -1), root().put("viewport_height", 20001),
            root().put("focused", "false"), root().put("main_count", 257),
            root().put("prompt_count", 17), root().put("visible_prompt_count", 2),
            root().put("prompt_count", 0), root().put("editable_count", 1.1),
            root().put("prompts", JSONArray().put(shape().put("text", "secret"))),
            root().put("prompts", JSONArray().put(shape().put("display", "raw style"))),
            root().put("prompts", JSONArray().put(shape().put("width", "320"))),
            root().put("body", shape().put("tag", "private title")))
        invalid.forEach { assertEquals("invalid_protocol_evidence", detail(it)) }
    }
}
