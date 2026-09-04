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

        assertTrue(WebChatConversationMutationPolicy.desiredPinned(unknown))
        assertEquals("置顶", WebChatConversationMutationPolicy.pinnedActionTitle(unknown))
        assertFalse(WebChatConversationMutationPolicy.desiredPinned(pinned))
        assertEquals("取消置顶", WebChatConversationMutationPolicy.pinnedActionTitle(pinned))
    }

    @Test
    fun pinnedMutationRequiresACompletedReceiptBeforeShowingSuccess() {
        assertEquals(
            WebChatConversationMutationProgress.WAITING,
            WebChatConversationMutationPolicy.progress(null),
        )
        assertEquals(
            WebChatConversationMutationProgress.WAITING,
            WebChatConversationMutationPolicy.progress(WebChatConsumerCommandStatus.PENDING),
        )
        assertEquals(
            WebChatConversationMutationProgress.SUCCEEDED,
            WebChatConversationMutationPolicy.progress(WebChatConsumerCommandStatus.SUCCEEDED),
        )
        assertEquals(
            WebChatConversationMutationProgress.NEEDS_OFFICIAL_CONFIRMATION,
            WebChatConversationMutationPolicy.progress(WebChatConsumerCommandStatus.FAILED),
        )
    }

    @Test
    fun renameTitlesAreNormalizedAndBoundedBeforeDispatch() {
        assertEquals(
            "新的 会话标题",
            WebChatConversationMutationPolicy.normalizedTitle("  新的   会话标题  "),
        )
        assertEquals(null, WebChatConversationMutationPolicy.normalizedTitle("   "))
        assertEquals(
            null,
            WebChatConversationMutationPolicy.normalizedTitle("x".repeat(161)),
        )
    }

    @Test
    fun projectMoveUsesTheDestinationInProgressAndCompletionCopy() {
        val intent = WebChatConversationMutationIntent.Moved(
            projectId = "g-p-demo",
            projectTitle = "家庭成员健康",
        )

        assertEquals(
            "正在移动到“家庭成员健康”",
            WebChatConversationMutationPolicy.progressTitle(intent),
        )
        assertEquals(
            "已移动到“家庭成员健康”",
            WebChatConversationMutationPolicy.completedMessage(intent),
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
