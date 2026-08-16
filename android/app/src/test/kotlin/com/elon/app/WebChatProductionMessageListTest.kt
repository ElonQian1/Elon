package com.elon.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionMessageListTest {
    @Test
    fun stableMessageIdsRemainTheSameItemWhileContentStreams() {
        val previous = message("assistant:1", "正在")
        val next = message("assistant:1", "正在回复")

        assertTrue(WebChatProductionMessageDiffPolicy.areItemsTheSame(previous, next))
        assertFalse(previous == next)
    }

    @Test
    fun missingOrDifferentIdsAreNeverMerged() {
        assertFalse(WebChatProductionMessageDiffPolicy.areItemsTheSame(
            message(null, "old"),
            message(null, "new"),
        ))
        assertFalse(WebChatProductionMessageDiffPolicy.areItemsTheSame(
            message("assistant:1", "old"),
            message("assistant:2", "new"),
        ))
    }

    @Test
    fun listFollowsOnlyNearTheEndUnlessExplicitlyRequested() {
        assertTrue(WebChatProductionScrollFollowPolicy.shouldFollow(false, 0, -1))
        assertTrue(WebChatProductionScrollFollowPolicy.shouldFollow(false, 10, 7))
        assertFalse(WebChatProductionScrollFollowPolicy.shouldFollow(false, 10, 5))
        assertTrue(WebChatProductionScrollFollowPolicy.shouldFollow(true, 10, 1))
    }

    private fun message(id: String?, content: String) = ChatMessage(
        role = "friend",
        content = content,
        id = id,
    )
}
