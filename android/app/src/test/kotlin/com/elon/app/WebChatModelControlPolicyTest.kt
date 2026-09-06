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
    fun restoresTheParentModelRowFromNestedOfficialLevels() {
        val presentation = WebChatModelControlPolicy.resolve(
            options = listOf(
                option("low", "轻度", parentId = "advanced", parentLabel = "高级"),
                option("medium", "标准", parentId = "advanced", parentLabel = "高级"),
                option(
                    "high",
                    "重度",
                    selected = true,
                    parentId = "advanced",
                    parentLabel = "高级",
                ),
                option("maximum", "极高", parentId = "advanced", parentLabel = "高级"),
            ),
            currentModel = "5.6 Sol 重度",
        )

        assertEquals("advanced", presentation.advanced?.id)
        assertEquals("高级", presentation.advanced?.label)
        assertTrue(presentation.advanced?.opensSubmenu == true)
        assertTrue(presentation.usesLevelSlider)
        assertEquals(2, presentation.selectedLevelIndex)
    }

    @Test
    fun compactsObservedModelLabelsForTheComposerPill() {
        assertEquals("低", WebChatModelControlPolicy.compactLabel("5.6 Sol 轻度"))
        assertEquals("中", WebChatModelControlPolicy.compactLabel("5.6 Sol 标准"))
        assertEquals("高", WebChatModelControlPolicy.compactLabel("5.6 Sol 重度"))
        assertEquals("极高", WebChatModelControlPolicy.compactLabel("5.6 Sol 极高"))
        assertEquals("快速", WebChatModelControlPolicy.compactLabel("GPT-5 Fast"))
    }

    @Test
    fun versionAndSpeedChoicesStayDiscreteAndKeepBothNavigationEntries() {
        val versions = listOf(
            option("back", "返回档位", opensSubmenu = true),
            option("v1", "5.1").copy(semantic = "model_version"),
            option("v2", "5.2", selected = true).copy(semantic = "model_version"),
            option("speed", "快速").copy(semantic = "service_tier"),
            option("other", "其他官网模型", opensSubmenu = true),
        )
        val presentation = WebChatModelControlPolicy.resolve(versions, "快速")
        assertFalse(presentation.usesLevelSlider)
        assertEquals("back", presentation.advanced?.id)
        assertEquals(listOf("v1", "v2", "speed", "other"), presentation.listOptions.map { it.id })
        assertFalse(WebChatModelControlPolicy.isSelected(versions[3], "快速"))
        assertTrue(WebChatModelControlPolicy.isSelected(versions[2], "快速"))
        assertFalse(WebChatModelControlPolicy.resolve(versions.subList(1, 3), "5.2").usesLevelSlider)
    }

    private fun option(
        id: String,
        label: String,
        selected: Boolean = false,
        opensSubmenu: Boolean = false,
        parentId: String? = null,
        parentLabel: String? = null,
    ) = WebChatConsumerOption(
        id = id,
        label = label,
        selected = selected,
        semantic = "model",
        opensSubmenu = opensSubmenu,
        nativeSelector = "web-chat-model-option:$id",
        parentId = parentId,
        parentLabel = parentLabel,
    )
}
