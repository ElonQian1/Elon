package com.elon.app

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionComposerToolsTest {
    @Test
    fun parsesCachedToolsForTheProductionComposer() {
        val navigation = JSONObject().put("composer_sections", JSONObject().put(
            "tools",
            JSONArray()
                .put(tool("search", "搜索", selected = true, selector = "chatgpt-tool:search"))
                .put(tool("study", "学习", selected = false, selector = "chatgpt-tool:study")),
        ))

        val result = WebChatProductionComposerToolParser.parse(navigation)

        assertEquals(listOf("search", "study"), result.map { it.id })
        assertTrue(result.first().selected)
        assertFalse(result.last().selected)
        assertEquals("chatgpt-tool:search", result.first().nativeSelector)
    }

    @Test
    fun ignoresInvalidAndDuplicateOptionsAndKeepsStableFallbackSelector() {
        val navigation = JSONObject().put("composer_sections", JSONObject().put(
            "tools",
            JSONArray()
                .put(tool("canvas", "画布", selected = false, selector = ""))
                .put(tool("canvas", "重复画布", selected = true, selector = "duplicate"))
                .put(JSONObject().put("id", "missing_label")),
        ))

        val result = WebChatProductionComposerToolParser.parse(navigation)

        assertEquals(1, result.size)
        assertEquals("web-chat-composer-tool:canvas", result.single().nativeSelector)
        assertFalse(result.single().selected)
    }

    @Test
    fun returnsEmptyWhenTheOfficialPageHasNoToolSection() {
        assertTrue(WebChatProductionComposerToolParser.parse(JSONObject()).isEmpty())
    }

    private fun tool(id: String, label: String, selected: Boolean, selector: String): JSONObject =
        JSONObject()
            .put("id", id)
            .put("label", label)
            .put("selected", selected)
            .put("native_adb_content_description", selector)
}
