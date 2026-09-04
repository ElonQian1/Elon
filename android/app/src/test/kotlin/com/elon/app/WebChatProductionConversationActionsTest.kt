package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionConversationActionsTest {
    @Test
    fun pinnedActionUsesObservedStateAndNeverAssumesUnknownItemsArePinned() {
        val unknown = com.elon.app.chatgptweb.ChatGptWebConversation(
            id = "unknown",
            title = "未知状态",
            path = "/c/unknown",
            active = false,
        )
        val pinned = unknown.copy(pinned = true)

        assertTrue(WebChatConversationPinnedMutationPolicy.desiredPinned(unknown))
        assertEquals("置顶", WebChatConversationPinnedMutationPolicy.actionTitle(unknown))
        assertFalse(WebChatConversationPinnedMutationPolicy.desiredPinned(pinned))
        assertEquals("取消置顶", WebChatConversationPinnedMutationPolicy.actionTitle(pinned))
    }

    @Test
    fun pinnedMutationRequiresACompletedReceiptBeforeShowingSuccess() {
        assertEquals(
            WebChatConversationPinnedMutationProgress.WAITING,
            WebChatConversationPinnedMutationPolicy.progress(null),
        )
        assertEquals(
            WebChatConversationPinnedMutationProgress.WAITING,
            WebChatConversationPinnedMutationPolicy.progress(WebChatConsumerCommandStatus.PENDING),
        )
        assertEquals(
            WebChatConversationPinnedMutationProgress.SUCCEEDED,
            WebChatConversationPinnedMutationPolicy.progress(WebChatConsumerCommandStatus.SUCCEEDED),
        )
        assertEquals(
            WebChatConversationPinnedMutationProgress.NEEDS_OFFICIAL_CONFIRMATION,
            WebChatConversationPinnedMutationPolicy.progress(WebChatConsumerCommandStatus.FAILED),
        )
    }

    @Test
    fun readyTargetConversationShowsActionsImmediately() {
        assertEquals(
            WebChatConversationActionReadiness.SHOW,
            WebChatProductionConversationActionPolicy.evaluate(
                WebChatProviderId.CHATGPT_WEB,
                targetPath = "/c/target",
                currentPath = "/c/target",
                state = "ready",
            ),
        )
    }

    @Test
    fun projectAndCanonicalPathsForTheSameConversationAreEquivalent() {
        assertEquals(
            WebChatConversationActionReadiness.SHOW,
            WebChatProductionConversationActionPolicy.evaluate(
                WebChatProviderId.CHATGPT_WEB,
                targetPath = "/g/g-p-project/c/target",
                currentPath = "/c/target",
                state = "ready",
            ),
        )
    }

    @Test
    fun anotherOrLoadingConversationWaitsForNavigation() {
        listOf("loading" to "/c/target", "ready" to "/c/other").forEach { (state, current) ->
            assertEquals(
                WebChatConversationActionReadiness.WAIT,
                WebChatProductionConversationActionPolicy.evaluate(
                    WebChatProviderId.CHATGPT_WEB,
                    targetPath = "/c/target",
                    currentPath = current,
                    state = state,
                ),
            )
        }
    }

    @Test
    fun nonChatGptProviderCancelsRemoteConversationActions() {
        assertEquals(
            WebChatConversationActionReadiness.CANCEL,
            WebChatProductionConversationActionPolicy.evaluate(
                WebChatProviderId.GOOGLE_WEB,
                targetPath = "/c/target",
                currentPath = "/c/target",
                state = "ready",
            ),
        )
    }

    @Test
    fun pendingDraftOnlyBlocksNavigationToAnotherConversation() {
        assertTrue(WebChatConversationDraftNavigation.blocks(
            targetPath = "/c/target",
            currentPath = "/c/current",
            draftPresent = true,
        ))
        assertFalse(WebChatConversationDraftNavigation.blocks(
            targetPath = "/g/g-p-project/c/current",
            currentPath = "/c/current",
            draftPresent = true,
        ))
        assertFalse(WebChatConversationDraftNavigation.blocks(
            targetPath = "/c/target",
            currentPath = "/c/current",
            draftPresent = false,
        ))
    }
}
