package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionFeatureCompletionTest {
    @Test
    fun waitsForBothSuccessfulReceiptAndSettledFeaturePage() {
        val feature = feature("tasks", "tasks")
        val pendingPage = state("mcp_1", WebChatConsumerCommandStatus.SUCCEEDED, "conversation")
        val settledPage = state("mcp_1", WebChatConsumerCommandStatus.SUCCEEDED, "tasks")

        assertEquals(
            WebChatProductionFeatureCompletionDecision.WAITING,
            WebChatProductionFeatureCompletionPolicy.evaluate(feature, "mcp_1", pendingPage),
        )
        assertEquals(
            WebChatProductionFeatureCompletionDecision.OPEN_OFFICIAL,
            WebChatProductionFeatureCompletionPolicy.evaluate(feature, "mcp_1", settledPage),
        )
    }

    @Test
    fun acceptsKnownOfficialUrlAfterNavigation() {
        val state = state(
            "mcp_2",
            WebChatConsumerCommandStatus.SUCCEEDED,
            pageKind = "unknown",
            pageUrl = "https://chatgpt.com/tasks?ref=native",
        )

        assertTrue(WebChatProductionFeatureCompletionPolicy.pageSettled(feature("tasks", "tasks"), state))
        assertFalse(WebChatProductionFeatureCompletionPolicy.pageSettled(feature("library", "library"), state))
        assertFalse(WebChatProductionFeatureCompletionPolicy.pageSettled(
            feature("tasks", "tasks"),
            state.copy(pageUrl = "https://chatgpt.com/tasks-unrelated"),
        ))
    }

    @Test
    fun acceptsOfficialImageGalleryRoute() {
        val state = state(
            "mcp_images",
            WebChatConsumerCommandStatus.SUCCEEDED,
            pageKind = "images",
            pageUrl = "https://chatgpt.com/images",
        )

        assertTrue(WebChatProductionFeatureCompletionPolicy.pageSettled(
            feature("images", "images"),
            state,
        ))
    }

    @Test
    fun rejectsUntrustedUrlAndFailedCommand() {
        val state = state(
            "mcp_3",
            WebChatConsumerCommandStatus.FAILED,
            pageKind = "tasks",
            pageUrl = "https://example.com/tasks",
        )

        assertEquals(
            WebChatProductionFeatureCompletionDecision.FAILED,
            WebChatProductionFeatureCompletionPolicy.evaluate(feature("tasks", "tasks"), "mcp_3", state),
        )
        assertFalse(WebChatProductionFeatureCompletionPolicy.pageSettled(
            feature("tasks", "tasks"),
            state.copy(pageKind = "unknown"),
        ))
    }

    @Test
    fun futureFeatureUsesPageKindAndOfficialCompletionByDefault() {
        val future = feature("future", "future_workspace")
        val state = state(
            "mcp_4",
            WebChatConsumerCommandStatus.SUCCEEDED,
            pageKind = "future_workspace",
        )

        assertTrue(WebChatProductionFeatureCompletionPolicy.requiresOfficialCompletion(future.kind))
        assertEquals(
            WebChatProductionFeatureCompletionDecision.OPEN_OFFICIAL,
            WebChatProductionFeatureCompletionPolicy.evaluate(future, "mcp_4", state),
        )
    }

    @Test
    fun waitsWhenTheCommandRequestIsMissingOrStillPending() {
        val state = state("mcp_5", WebChatConsumerCommandStatus.PENDING, pageKind = "tasks")

        assertEquals(
            WebChatProductionFeatureCompletionDecision.WAITING,
            WebChatProductionFeatureCompletionPolicy.evaluate(feature("tasks", "tasks"), "mcp_5", state),
        )
        assertEquals(
            WebChatProductionFeatureCompletionDecision.WAITING,
            WebChatProductionFeatureCompletionPolicy.evaluate(feature("tasks", "tasks"), "missing", state),
        )
    }

    private fun feature(id: String, kind: String) = WebChatProductionFeature(
        id = id,
        label = id,
        kind = kind,
        selected = false,
        requiresUserConfirmation = false,
        officialCompletion = true,
        nativeSelector = "web-chat-feature:$id",
    )

    private fun state(
        requestId: String,
        status: WebChatConsumerCommandStatus,
        pageKind: String,
        pageUrl: String = "https://chatgpt.com/",
    ) = WebChatConsumerState(
        streaming = false,
        dictationActive = false,
        composerSections = emptyMap(),
        pageKind = pageKind,
        pageUrl = pageUrl,
        features = emptyList(),
        commandRequests = listOf(WebChatConsumerCommandRequest(requestId, status)),
    )
}
