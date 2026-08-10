package com.elon.app.mcp

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.atomic.AtomicReference
import kotlin.concurrent.thread

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

    @Test
    fun waitsForTheRequestedActivitySurfaceInsteadOfAnyRegisteredController() {
        val main = controller("main")
        val chatGpt = controller("chatgpt_web")
        val active = AtomicReference<McpNativeControlBridge.Controller>(main)
        val registrar = thread {
            Thread.sleep(40L)
            active.set(chatGpt)
        }

        try {
            assertSame(
                chatGpt,
                McpNativeControlBridge.waitForControllerSurface("chatgpt_web", 1_000L, active::get),
            )
            assertNull(McpNativeControlBridge.waitForControllerSurface("missing", 0L, active::get))
            assertEquals("chatgpt_web", McpNativeControlBridge.targetSurfaceFor("open_chatgpt_web"))
            assertNull(McpNativeControlBridge.targetSurfaceFor("state"))
        } finally {
            registrar.join()
        }
    }

    private fun controller(surfaceId: String) = object : McpNativeControlBridge.Controller {
        override val surfaceId: String = surfaceId
        override fun uiState(): JSONObject = JSONObject().put("surface", surfaceId)
        override fun control(args: JSONObject): JSONObject = uiState().put("control_ok", true)
    }
}
