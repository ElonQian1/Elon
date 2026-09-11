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
            listOf("创建图片", "网页搜索", "学习与研究", "画布"),
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
            )?.id,
        )
        assertEquals(
            "tool-b",
            WebChatProductionQuickComposerActionResolver.find(
                WebChatProductionQuickComposerAction.WEB_SEARCH,
                tools,
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
            )?.id,
        )
        assertEquals(
            "browse",
            WebChatProductionQuickComposerActionResolver.find(
                WebChatProductionQuickComposerAction.WEB_SEARCH,
                tools,
            )?.id,
        )
        assertNull(
            WebChatProductionQuickComposerActionResolver.find(
                WebChatProductionQuickComposerAction.IMAGE_GENERATION,
                WebChatProductionComposerToolParser.parse(options.take(1)),
            ),
        )
    }

    @Test fun confirmedDisabledCatalogClearsAnOldSelectedChip() {
        val tools = WebChatProductionComposerToolParser.parse(listOf(
            option("image", "Image", "image_generation"),
            option("search", "Search", "web_search"),
        ))
        assertNull(WebChatProductionQuickComposerActionResolver.selection(
            tools, WebChatProductionQuickComposerAction.IMAGE_GENERATION,
        ))
        assertNull(WebChatProductionQuickComposerActionResolver.selection(
            tools, WebChatProductionQuickComposerAction.WEB_SEARCH,
        ))
    }

    @Test fun missingCatalogKeepsLastObservedChoiceInsteadOfAssumingDisabled() {
        assertEquals(WebChatProductionQuickComposerAction.IMAGE_GENERATION,
            WebChatProductionQuickComposerActionResolver.selection(
                emptyList(), WebChatProductionQuickComposerAction.IMAGE_GENERATION,
            ))
        assertNull(WebChatProductionQuickComposerActionResolver.selection(emptyList(), null))
    }

    @Test fun aNewSelectedToolReplacesThePreviouslyDisplayedTool() {
        val tools = WebChatProductionComposerToolParser.parse(listOf(
            option("image", "Image", "image_generation"),
            option("search", "Search", "web_search").copy(selected = true),
        ))
        assertEquals(WebChatProductionQuickComposerAction.WEB_SEARCH,
            WebChatProductionQuickComposerActionResolver.selection(
                tools, WebChatProductionQuickComposerAction.IMAGE_GENERATION,
            ))
    }

    @Test fun aCatalogWithOnlyOtherToolsDoesNotKeepAnUnavailableQuickSelection() {
        val tools = WebChatProductionComposerToolParser.parse(listOf(
            option("upload", "Photos", "attachment_photos").copy(selected = true),
        ))
        assertNull(WebChatProductionQuickComposerActionResolver.selection(
            tools, WebChatProductionQuickComposerAction.IMAGE_GENERATION,
        ))
    }

    @Test fun extendedToolsUseCanonicalSemanticsAndKeepOneSelectedChip() {
        for (action in listOf(WebChatProductionQuickComposerAction.STUDY, WebChatProductionQuickComposerAction.CANVAS)) {
            val tools = WebChatProductionComposerToolParser.parse(listOf(
                option("selected", "Arbitrary official label", action.semantic).copy(selected = true),
            ))
            assertEquals(action, WebChatProductionQuickComposerActionResolver.selection(tools, null))
            assertEquals("selected", WebChatProductionQuickComposerActionResolver.find(action, tools)?.id)
            assertEquals(action, WebChatProductionQuickComposerActionResolver.selection(emptyList(), action))
        }
    }

    @Test fun toolLabelsCannotOverrideAttachmentOrResearchSemantics() {
        for ((label, semantic) in listOf("Canvas" to "attachment_file", "学习与研究" to "deep_research",
            "Search the web" to "attachment_photos", "Create image" to "agent")) {
            val tool = WebChatProductionComposerToolParser.parse(listOf(option("other", label, semantic))).single()
            assertNull(WebChatProductionQuickComposerActionResolver.actionFor(tool))
        }
    }

    @Test fun extendedLocalizedLabelsAreExactInsteadOfMatchingFileNames() {
        val tools = WebChatProductionComposerToolParser.parse(listOf(
            option("learn", "Study and learn", "tool"), option("canvas", "画布", "tool"),
            option("file", "Canvas notes.pdf", "tool"),
        ))
        assertEquals(WebChatProductionQuickComposerAction.STUDY, WebChatProductionQuickComposerActionResolver.actionFor(tools[0]))
        assertEquals(WebChatProductionQuickComposerAction.CANVAS, WebChatProductionQuickComposerActionResolver.actionFor(tools[1]))
        assertNull(WebChatProductionQuickComposerActionResolver.actionFor(tools[2]))
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
