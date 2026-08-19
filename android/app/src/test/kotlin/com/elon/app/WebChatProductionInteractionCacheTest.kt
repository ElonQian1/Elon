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
    fun authoritativeEmptyComposerSnapshotClearsAStaleSection() {
        cache.composerOptions(WebChatProviderId.CHATGPT_WEB, "model", listOf(option("stale")))

        assertTrue(cache.replaceComposerOptions(
            WebChatProviderId.CHATGPT_WEB,
            "MODEL",
            emptyList(),
        ).isEmpty())
        assertTrue(cache.composerOptions(
            WebChatProviderId.CHATGPT_WEB,
            "model",
            emptyList(),
        ).isEmpty())
        assertTrue(cache.hasComposerSnapshot(WebChatProviderId.CHATGPT_WEB, "MODEL"))
    }

    @Test
    fun replacesCachedManifestOnlyWhenThePageReturnsAUsableSnapshot() {
        val conversationState = state("conversation", "https://chatgpt.com/c/one")
        cache.features(WebChatProviderId.CHATGPT_WEB, listOf(feature("projects")))
        cache.controls(
            WebChatProviderId.CHATGPT_WEB,
            conversationState.copy(controls = listOf(control("profile"))),
        )

        assertEquals(
            "projects",
            cache.features(WebChatProviderId.CHATGPT_WEB, emptyList()).single().id,
        )
        assertEquals(
            "profile",
            cache.controls(WebChatProviderId.CHATGPT_WEB, conversationState).single().control.id,
        )
        assertTrue(cache.hasFeatureSnapshot(WebChatProviderId.CHATGPT_WEB))
        assertTrue(cache.hasControlSnapshot(WebChatProviderId.CHATGPT_WEB, conversationState))

        assertEquals(
            "tasks",
            cache.features(WebChatProviderId.CHATGPT_WEB, listOf(feature("tasks"))).single().id,
        )
        assertEquals(
            "more",
            cache.controls(
                WebChatProviderId.CHATGPT_WEB,
                conversationState.copy(controls = listOf(control("more"))),
            ).single().control.id,
        )
    }

    @Test
    fun controlSnapshotsNeverLeakAcrossPagesOrConversations() {
        val firstConversation = state("conversation", "https://chatgpt.com/c/first")
        val secondConversation = state("conversation", "https://chatgpt.com/c/second")
        val featurePage = state("feature", "https://chatgpt.com/images")

        cache.controls(
            WebChatProviderId.CHATGPT_WEB,
            firstConversation.copy(controls = listOf(control("first-actions"))),
        )

        assertTrue(cache.controls(WebChatProviderId.CHATGPT_WEB, secondConversation).isEmpty())
        assertTrue(cache.controls(WebChatProviderId.CHATGPT_WEB, featurePage).isEmpty())
        assertEquals(
            "first-actions",
            cache.controls(WebChatProviderId.CHATGPT_WEB, firstConversation).single().control.id,
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
            state("conversation", "https://example.invalid/"),
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

    private fun state(pageKind: String, pageUrl: String) = WebChatConsumerState(
        streaming = false,
        dictationActive = false,
        composerSections = emptyMap(),
        pageKind = pageKind,
        pageUrl = pageUrl,
        features = emptyList(),
        controls = emptyList(),
        commandRequests = emptyList(),
    )
}
