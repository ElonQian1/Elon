package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GroupWebAiFailureTest {
    @Test fun onlyExactPreClickReceiptCanRetireTheAuthorization() {
        val rejected = ChatGptWebEvent.CommandResult("send_prompt", false,
            "官方输入框未接受文本，请返回官网重试。", "group_test")
        assertTrue(rejected.isConfirmedPreSendRejection())
        assertTrue(rejected.copy(detail = rejected.detail + " [runtime_fallback:runtime_unavailable]").isConfirmedPreSendRejection())
        assertFalse(rejected.copy(detail = rejected.detail + " arbitrary suffix").isConfirmedPreSendRejection())
        assertFalse(rejected.copy(ok = true).isConfirmedPreSendRejection())
        assertFalse(rejected.copy(action = "set_draft").isConfirmedPreSendRejection())
        for (detail in listOf("", "official_runtime_v1:unknown:timeout",
            "private_text_v1:unknown:timeout", "官方网页未确认发送，请稍候。",
            "official_runtime_v1:rejected:previous_send_unresolved")) {
            assertFalse(rejected.copy(detail = detail).isConfirmedPreSendRejection())
        }
        val failure = GroupWebAiFailure(false, GroupWebAiFailureReason.COMPOSER, rejectedBeforeSend = true)
        assertTrue(failure.rejectedBeforeSend)
        assertTrue(failure.canRetry)
        assertFalse(failure.message.contains("请求可能已发送"))
    }

    @Test fun preAuthorizationFailureCanReuseReservedSelection() {
        for (reason in listOf(GroupWebAiFailureReason.PAGE, GroupWebAiFailureReason.TIMEOUT,
            GroupWebAiFailureReason.LOGIN, GroupWebAiFailureReason.MODEL, GroupWebAiFailureReason.PREPARATION)) {
            val failure = GroupWebAiFailure(false, reason)
            assertTrue(failure.canRetry)
            assertTrue(failure.message.contains("尚未发送"))
        }
    }

    @Test fun unknownDispatchNeverOffersRetryOrClaimsTheAnswerWillArrive() {
        for (reason in GroupWebAiFailureReason.entries) {
            val failure = GroupWebAiFailure(true, reason)
            assertFalse(failure.canRetry)
            assertTrue(failure.message.contains("尚未生成群回复"))
            assertFalse(failure.message.contains("请稍后查看群消息"))
        }
    }

    @Test fun explicitCancellationDoesNotOfferRetry() {
        assertFalse(GroupWebAiFailure(false, GroupWebAiFailureReason.CANCELLED).canRetry)
    }
}
