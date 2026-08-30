package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatTemporaryChatIntentQueueTest {
    private val queue = WebChatTemporaryChatIntentQueue()

    @Test
    fun waitsForALiveControlThenEmitsOneMutation() {
        assertTrue(queue.begin(desiredSelected = true))
        assertFalse(queue.begin(desiredSelected = true))
        assertEquals(
            WebChatTemporaryChatIntentDecision.AwaitingControl,
            queue.evaluate(control = null),
        )

        assertEquals(
            WebChatTemporaryChatIntentDecision.Apply("temporary", true),
            queue.evaluate(control(selected = false)),
        )
        assertEquals(
            WebChatTemporaryChatIntentDecision.AwaitingConfirmation,
            queue.evaluate(control(selected = false)),
        )
        assertEquals(
            WebChatTemporaryChatIntentDecision.Confirmed(true),
            queue.evaluate(control(selected = true)),
        )
    }

    @Test
    fun rejectedMutationCanBeResolvedAgainstAFreshControlId() {
        assertTrue(queue.begin(desiredSelected = true))
        assertEquals(
            WebChatTemporaryChatIntentDecision.Apply("temporary", true),
            queue.evaluate(control(selected = false)),
        )

        queue.mutationRejected("temporary")

        assertEquals(
            WebChatTemporaryChatIntentDecision.AwaitingControl,
            queue.evaluate(control(selected = false)),
        )

        assertEquals(
            WebChatTemporaryChatIntentDecision.Apply("temporary-new", true),
            queue.evaluate(control(selected = false, id = "temporary-new")),
        )
    }

    private fun control(
        selected: Boolean,
        id: String = "temporary",
    ) = ChatGptWebUiControl(
        id = id,
        label = "临时聊天",
        semantic = "temporary_chat",
        region = "header",
        role = "button",
        enabled = true,
        selected = selected,
        stateSettable = true,
    )
}
