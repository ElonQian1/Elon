package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionQuickComposerActionsTest {
    @Test
    fun exposesFamiliarQuickActionsOnlyForProvidersWithComposerTools() {
        val chatGpt = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        val google = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB)

        assertEquals(
            listOf("创建图片", "网页搜索"),
            WebChatProductionQuickComposerActionCatalog.availableFor(chatGpt).map { it.label },
        )
        assertTrue(WebChatProductionQuickComposerActionCatalog.availableFor(google).isEmpty())
    }

    @Test
    fun resolvesQuickActionsFromStableSemanticValues() {
        val options = listOf(
            option("tool-a", "Make something", "image_generation"),
            option("tool-b", "Search", "web_search"),
        )
        val tools = WebChatProductionComposerToolParser.parse(options)

        assertEquals(
            "tool-a",
            WebChatProductionQuickComposerActionResolver.find(
                WebChatProductionQuickComposerAction.IMAGE_GENERATION,
                tools,
                options,
            )?.id,
        )
        assertEquals(
            "tool-b",
            WebChatProductionQuickComposerActionResolver.find(
                WebChatProductionQuickComposerAction.WEB_SEARCH,
                tools,
                options,
            )?.id,
        )
    }

    @Test
    fun resolvesLocalizedLabelsWithoutConfusingPhotoUploadWithImageCreation() {
        val options = listOf(
            option("upload", "照片", "attachment_photos"),
            option("draw", "生成图像", "tool"),
            option("browse", "联网搜索", "tool"),
        )
        val tools = WebChatProductionComposerToolParser.parse(options)

        assertEquals(
            "draw",
            WebChatProductionQuickComposerActionResolver.find(
                WebChatProductionQuickComposerAction.IMAGE_GENERATION,
                tools,
                options,
            )?.id,
        )
        assertEquals(
            "browse",
            WebChatProductionQuickComposerActionResolver.find(
                WebChatProductionQuickComposerAction.WEB_SEARCH,
                tools,
                options,
            )?.id,
        )
        assertNull(
            WebChatProductionQuickComposerActionResolver.find(
                WebChatProductionQuickComposerAction.IMAGE_GENERATION,
                WebChatProductionComposerToolParser.parse(options.take(1)),
                options.take(1),
            ),
        )
    }

    private fun option(id: String, label: String, semantic: String) = WebChatConsumerOption(
        id = id,
        label = label,
        selected = false,
        semantic = semantic,
        opensSubmenu = false,
        nativeSelector = "web-chat-composer-tool:$id",
    )
}
