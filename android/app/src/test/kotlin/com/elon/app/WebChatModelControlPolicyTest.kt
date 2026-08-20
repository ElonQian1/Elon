package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatModelControlPolicyTest {
    @Test
    fun separatesAdvancedNavigationFromTheOfficialLevelScale() {
        val presentation = WebChatModelControlPolicy.resolve(
            options = listOf(
                option("advanced", "高级", opensSubmenu = true),
                option("low", "轻度"),
                option("medium", "标准"),
                option("high", "重度", selected = true),
                option("maximum", "极高"),
            ),
            currentModel = "5.6 Sol 重度",
        )

        assertEquals("advanced", presentation.advanced?.id)
        assertTrue(presentation.usesLevelSlider)
        assertEquals(listOf("low", "medium", "high", "maximum"), presentation.levels.map { it.id })
        assertEquals(2, presentation.selectedLevelIndex)
        assertTrue(presentation.listOptions.isEmpty())
    }

    @Test
    fun keepsLongModelNamesAsAnAdvancedListInsteadOfInventingLevels() {
        val presentation = WebChatModelControlPolicy.resolve(
            options = listOf(
                option("gpt", "GPT-5.6 Thinking"),
                option("sol", "GPT-5.6 Sol Pro"),
            ),
            currentModel = "GPT-5.6 Thinking",
        )

        assertFalse(presentation.usesLevelSlider)
        assertEquals(listOf("gpt", "sol"), presentation.listOptions.map { it.id })
    }

    @Test
    fun compactsObservedModelLabelsForTheComposerPill() {
        assertEquals("低", WebChatModelControlPolicy.compactLabel("5.6 Sol 轻度"))
        assertEquals("中", WebChatModelControlPolicy.compactLabel("5.6 Sol 标准"))
        assertEquals("高", WebChatModelControlPolicy.compactLabel("5.6 Sol 重度"))
        assertEquals("极高", WebChatModelControlPolicy.compactLabel("5.6 Sol 极高"))
        assertEquals("快速", WebChatModelControlPolicy.compactLabel("GPT-5 Fast"))
    }

    private fun option(
        id: String,
        label: String,
        selected: Boolean = false,
        opensSubmenu: Boolean = false,
    ) = WebChatConsumerOption(
        id = id,
        label = label,
        selected = selected,
        semantic = "model",
        opensSubmenu = opensSubmenu,
        nativeSelector = "web-chat-model-option:$id",
    )
}
