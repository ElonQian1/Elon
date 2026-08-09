package com.elon.app.mcp

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class McpNativeControlBridgeTest {
    @Test
    fun preservesPayloadSchemaAndAddsEnvelopeSchema() {
        val result = McpNativeControlBridge.decorateControlResult(
            JSONObject().put("schema", "elon.chatgpt_web.capability_matrix.v2"),
            "chatgpt_get_capability_matrix",
        )

        assertEquals("elon.chatgpt_web.capability_matrix.v2", result.getString("schema"))
        assertEquals("elon.apk.native_mcp_control_result.v1", result.getString("envelope_schema"))
        assertEquals("chatgpt_get_capability_matrix", result.getString("action"))
        assertTrue(result.getBoolean("activity_bound"))
    }

    @Test
    fun suppliesEnvelopeSchemaWhenPayloadHasNoSchema() {
        val result = McpNativeControlBridge.decorateControlResult(
            JSONObject().put("control_ok", true),
            "state",
        )

        assertEquals("elon.apk.native_mcp_control_result.v1", result.getString("schema"))
        assertFalse(result.has("envelope_schema"))
    }
}
