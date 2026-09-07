package com.elon.app.chatgptweb

import org.junit.Assert.*
import org.junit.Test

class ChatGptNewConversationConfirmationStateTest {
    private var now = 100L
    private val state = ChatGptNewConversationConfirmationState { now }
    private val ticket = "a".repeat(32)
    private fun detail(id: String = ticket) =
        "请确认。 [runtime_new_chat:confirmation_required] [confirmation_id:$id]"

    @Test fun acceptsOneExactDecisionAndNeverReopensAConsumedReceipt() {
        assertEquals(ticket, state.offer("new_conversation", false, detail()))
        assertTrue(state.consume(ticket))
        assertFalse(state.consume(ticket))
        assertNull(state.offer("new_conversation", false, detail()))
    }

    @Test fun rejectsOtherActionsAndSuccessfulReceipts() {
        assertNull(state.offer("send_prompt", false, detail()))
        assertNull(state.offer("new_conversation", true, detail()))
        assertFalse(state.consume(ticket))
    }

    @Test fun rejectsMalformedOrUnboundTickets() {
        for (value in listOf("", detail("a"), detail("A".repeat(32)), detail() + " extra",
            detail().replace("confirmation_required", "ready"), "x".repeat(513) + detail())) {
            assertNull(state.offer("new_conversation", false, value))
        }
        assertFalse(state.consume(ticket))
    }

    @Test fun doesNotReplaceAnOpenConfirmationWithAnotherReceipt() {
        assertEquals(ticket, state.offer("new_conversation", false, detail()))
        assertNull(state.offer("new_conversation", false, detail("b".repeat(32))))
        assertFalse(state.consume("b".repeat(32)))
        assertTrue(state.consume(ticket))
    }

    @Test fun expiryCannotTurnIntoApproval() {
        state.offer("new_conversation", false, detail())
        now += ChatGptNewConversationConfirmationState.TIMEOUT_MS
        assertFalse(state.consume(ticket))
        assertNull(state.offer("new_conversation", false, detail()))
        assertEquals("b".repeat(32), state.offer("new_conversation", false, detail("b".repeat(32))))
    }

    @Test fun preservesLastInstantBeforeExpiry() {
        state.offer("new_conversation", false, detail())
        now += ChatGptNewConversationConfirmationState.TIMEOUT_MS - 1
        assertTrue(state.consume(ticket))
    }

    @Test fun expiredDialogCanCancelItsPageLeaseButCannotApprove() {
        state.offer("new_conversation", false, detail())
        now += ChatGptNewConversationConfirmationState.TIMEOUT_MS
        assertTrue(state.consume(ticket, requireFresh = false))
        assertFalse(state.consume(ticket))
        assertFalse(state.consume(ticket, requireFresh = false))
        assertEquals("b".repeat(32), state.offer("new_conversation", false, detail("b".repeat(32))))
    }

    @Test fun cancellationCannotConsumeAnotherDialogsTicket() {
        state.offer("new_conversation", false, detail())
        assertFalse(state.consume("b".repeat(32), requireFresh = false))
        assertTrue(state.consume(ticket))
    }

    @Test fun userFacingDetailsDoNotExposeProtocolTagsOrTicket() {
        val message = ChatGptNewConversationConfirmationState.userDetail("new_conversation", detail())!!
        assertFalse(message.contains(ticket))
        assertFalse(message.contains("runtime_new_chat"))
        assertNull(ChatGptNewConversationConfirmationState.userDetail("other", detail()))
        assertNull(ChatGptNewConversationConfirmationState.userDetail("new_conversation", "unchanged error"))
    }
}
