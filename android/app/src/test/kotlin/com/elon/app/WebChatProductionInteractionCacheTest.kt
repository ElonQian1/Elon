package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionInteractionCacheTest {
    private val cache = WebChatProductionInteractionCache()

    @Test
    fun keepsComposerChoicesDuringARefreshWithoutMixingProvidersOrSections() {
        cache.composerOptions(WebChatProviderId.CHATGPT_WEB, "tools", listOf(option("search")))
        cache.composerOptions(WebChatProviderId.CHATGPT_WEB, "model", listOf(option("fast")))

        assertEquals(
            listOf("search"),
            cache.composerOptions(WebChatProviderId.CHATGPT_WEB, "tools", emptyList()).map { it.id },
        )
        assertEquals(
            listOf("fast"),
            cache.composerOptions(WebChatProviderId.CHATGPT_WEB, "MODEL", emptyList()).map { it.id },
        )
        assertTrue(cache.composerOptions(WebChatProviderId.GOOGLE_WEB, "tools", emptyList()).isEmpty())
    }

    @Test
    fun replacesCachedManifestOnlyWhenThePageReturnsAUsableSnapshot() {
        cache.features(WebChatProviderId.CHATGPT_WEB, listOf(feature("projects")))
        cache.controls(WebChatProviderId.CHATGPT_WEB, listOf(control("profile")))

        assertEquals(
            "projects",
            cache.features(WebChatProviderId.CHATGPT_WEB, emptyList()).single().id,
        )
        assertEquals(
            "profile",
            cache.controls(WebChatProviderId.CHATGPT_WEB, emptyList()).single().control.id,
        )

        assertEquals(
            "tasks",
            cache.features(WebChatProviderId.CHATGPT_WEB, listOf(feature("tasks"))).single().id,
        )
        assertEquals(
            "more",
            cache.controls(WebChatProviderId.CHATGPT_WEB, listOf(control("more"))).single().control.id,
        )
    }

    @Test
    fun clearingOneProviderDoesNotDiscardAnotherProvidersSnapshot() {
        cache.composerOptions(WebChatProviderId.CHATGPT_WEB, "tools", listOf(option("search")))
        cache.composerOptions(WebChatProviderId.GOOGLE_WEB, "tools", listOf(option("lens")))

        cache.clear(WebChatProviderId.CHATGPT_WEB)

        assertTrue(cache.composerOptions(WebChatProviderId.CHATGPT_WEB, "tools", emptyList()).isEmpty())
        assertEquals(
            "lens",
            cache.composerOptions(WebChatProviderId.GOOGLE_WEB, "tools", emptyList()).single().id,
        )
    }

    @Test
    fun capturesEveryInteractionSurfaceDuringBackgroundPrewarm() {
        cache.capture(
            WebChatProviderId.CHATGPT_WEB,
            WebChatConsumerState(
                streaming = false,
                dictationActive = false,
                composerSections = mapOf(
                    "model" to listOf(option("fast")),
                    "tools" to listOf(option("search")),
                ),
                pageKind = "conversation",
                pageUrl = "https://example.invalid/",
                features = listOf(feature("projects")),
                controls = listOf(control("more")),
                commandRequests = emptyList(),
            ),
        )

        assertEquals("fast", cache.composerOptions(
            WebChatProviderId.CHATGPT_WEB,
            "model",
            emptyList(),
        ).single().id)
        assertEquals("search", cache.composerOptions(
            WebChatProviderId.CHATGPT_WEB,
            "tools",
            emptyList(),
        ).single().id)
        assertEquals("projects", cache.features(
            WebChatProviderId.CHATGPT_WEB,
            emptyList(),
        ).single().id)
        assertEquals("more", cache.controls(
            WebChatProviderId.CHATGPT_WEB,
            emptyList(),
        ).single().control.id)
    }

    private fun option(id: String) = WebChatConsumerOption(
        id = id,
        label = id,
        selected = false,
        semantic = id,
        opensSubmenu = false,
        nativeSelector = "option:$id",
    )

    private fun feature(id: String) = WebChatConsumerFeature(
        id = id,
        label = id,
        kind = id,
        selected = false,
        requiresUserConfirmation = false,
        nativeSelector = "feature:$id",
    )

    private fun control(id: String) = WebChatConsumerControlDescriptor(
        control = ChatGptWebUiControl(
            id = id,
            label = id,
            semantic = id,
            region = "header",
            role = "button",
            enabled = true,
            selected = false,
        ),
        requiresUserConfirmation = false,
        presentation = WebChatConsumerControlPresentation.DIRECT,
        nativeSelector = "control:$id",
    )
}
