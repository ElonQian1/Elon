package com.elon.app.chatgptweb

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class GroupWebAiFailureTest {
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
