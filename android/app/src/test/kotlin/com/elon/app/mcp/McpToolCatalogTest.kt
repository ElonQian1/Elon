package com.elon.app.mcp

import org.json.JSONObject
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class McpToolCatalogTest {
    @Test
    fun uiControlDeclaresTheChatGptContextCursorInItsStrictSchema() {
        val schema = toolSchema("ui_control")
        val properties = schema.getJSONObject("properties")

        assertFalse(schema.getBoolean("additionalProperties"))
        assertTrue(properties.has("message_cursor"))
        assertTrue(
            properties.getJSONObject("message_cursor")
                .getString("description")
                .contains("chatgpt_get_context"),
        )
    }

    private fun toolSchema(name: String): JSONObject {
        val tools = mcpToolsListResult().getJSONArray("tools")
        for (index in 0 until tools.length()) {
            val tool = tools.getJSONObject(index)
            if (tool.getString("name") == name) return tool.getJSONObject("inputSchema")
        }
        error("MCP tool not found: $name")
    }
}
