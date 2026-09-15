package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebEvent
import com.elon.app.chatgptweb.ChatGptWebProtocol
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatPendingWriteRecoveryTest {
    private fun parse(state: String?): String {
        val event = JSONObject().put("type", "message_snapshot")
        if (state != null) event.put("privateSendRecoveryState", state)
        val wire = JSONObject().put("schema", "yilong.ai.ui.v1").put("event", event)
        return (ChatGptWebProtocol.parse(wire.toString()) as ChatGptWebEvent.Snapshot)
            .value.privateSendRecoveryState
    }

    @Test
    fun snapshotOnlyAcceptsKnownAggregateStatesAndOldAdaptersStayHidden() {
        for (state in listOf("disabled", "idle", "checking", "clear", "recovered", "unconfirmed", "unavailable")) {
            assertEquals(state, parse(state))
        }
        for (state in listOf(null, "", "unknown", "checking-current-account", "x".repeat(1000))) {
            assertEquals("disabled", parse(state))
        }
    }

    @Test
    fun emptyOrCompletedRecoveryDoesNotAddLoadingChrome() {
        for (state in listOf("disabled", "idle", "clear", "recovered", "unknown")) {
            assertNull(WebChatPendingWriteRecoveryPresentation.resolve(state))
        }
    }

    @Test
    fun checkingCannotOfferAnAccidentalResend() {
        val state = requireNotNull(WebChatPendingWriteRecoveryPresentation.resolve("checking"))
        assertTrue(state.visible)
        assertEquals("正在核对上次发送结果", state.message)
        assertFalse(state.retryVisible)
        assertFalse(state.officialVisible)
    }

    @Test
    fun uncertainWritesStayExplicitAndOfferReadOnlyInspection() {
        for (phase in listOf("unconfirmed", "unavailable")) {
            val state = requireNotNull(WebChatPendingWriteRecoveryPresentation.resolve(phase))
            assertTrue(state.visible)
            assertFalse(state.retryVisible)
            assertTrue(state.officialVisible)
            assertEquals("查看", state.officialLabel)
        }
    }

    @Test
    fun productionBannerIncludesRecoveryWithoutChangingConnectionOrLoginHandling() {
        val provider = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        assertEquals("正在核对上次发送结果", WebChatConsumerRecoveryPolicy.resolve(
            provider, "ready", pendingWriteRecovery = parse("checking"),
        ).message)
        assertTrue(WebChatConsumerRecoveryPolicy.resolve(
            provider, "error", pendingWriteRecovery = "unconfirmed",
        ).retryVisible)
        assertEquals("登录", WebChatConsumerRecoveryPolicy.resolve(
            provider, "login_required", pendingWriteRecovery = "unconfirmed",
        ).officialLabel)
    }
}
