package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ChatGptWebRegenerationReceiptPolicyTest {
    @Test
    fun newReplyObservationDoesNotClaimFullGenerationCompletion() {
        assertEquals("已开始重新生成回复。", detail(true, "observed"))
    }

    @Test
    fun busyAndIndeterminateWritesDoNotTellTheUserToResend() {
        assertEquals("上一条回复仍在处理中，请稍候。", detail(false, "unknown:busy"))
        for (code in listOf("timeout", "invocation_failed", "completion_failed", "document_changed")) {
            assertEquals("重新生成结果正在核对，请稍候，勿重复提交。", detail(false, "unknown:$code"))
        }
    }

    @Test
    fun rejectionIsExplicitlyUnsent() {
        assertEquals("当前会话暂不能重新生成，本次未提交，请确认会话状态后重试。", detail(false, "rejected:context_changed"))
    }

    @Test
    fun otherActionsAndMalformedReceiptsDoNotGetInterpreted() {
        assertNull(ChatGptWebRegenerationReceiptPolicy.userDetail("send_prompt", true, "official_runtime_v1:regenerate_observed"))
        assertNull(detail(false, "unknown:../../invalid"))
        assertNull(detail(false, "observed"))
        assertNull(detail(true, "unknown:timeout"))
        assertNull(detail(false, "invalid:reason"))
        assertNull(detail(false, "rejected:"))
    }

    private fun detail(ok: Boolean, value: String) = ChatGptWebRegenerationReceiptPolicy.userDetail(
        "regenerate_response", ok, "official_runtime_v1:regenerate_$value",
    )
}
