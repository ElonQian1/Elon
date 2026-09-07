package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebPrivateRuntimeAssetsTest {
    private fun root(names: List<Any> = listOf("shared-abc123.js")) = JSONObject()
        .put("schema", ChatGptWebPrivateRuntimeAssets.SCHEMA)
        .put("assets", JSONArray(names)).put("truncated", false)

    private fun detail(value: JSONObject) = ChatGptWebPrivateProtocolEvidence.detail(
        "private_protocol_probe", value.toString(),
    )

    @Test fun acceptsOnlyBoundedPublicFilenamesThroughProductionReceipt() {
        assertTrue("runtime_assets" in ChatGptWebPrivateProtocolEvidence.MODES)
        val result = JSONObject(detail(root()))
        assertEquals("shared-abc123.js", result.getJSONArray("assets").getString(0))
        assertFalse(result.getBoolean("truncated"))
        assertTrue(JSONObject(detail(root().put("truncated", true))).getBoolean("truncated"))
        assertEquals(0, JSONObject(detail(root(emptyList()))).getJSONArray("assets").length())
    }

    @Test fun rejectsValuesUrlsUnknownKeysAndInvalidTypes() {
        val invalid = listOf(
            root(listOf("https://chatgpt.com/cdn/assets/shared.js")),
            root(listOf("shared.js?token=secret")), root(listOf("shared.js#secret")),
            root(listOf("../shared.js")), root(listOf("bad\nname.js")),
            root(listOf("a".repeat(97) + ".js")), root(listOf(123)),
            root(listOf("shared.js", "shared.js")), root(List(97) { "chunk-$it.js" }),
            root().put("headers", "secret"), root().put("truncated", "false"),
        )
        invalid.forEach { assertEquals("invalid_protocol_evidence", detail(it)) }
    }
}
