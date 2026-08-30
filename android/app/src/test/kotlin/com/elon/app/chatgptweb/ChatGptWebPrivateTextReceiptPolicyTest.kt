package com.elon.app.chatgptweb

import com.elon.app.WebChatSendAuthority
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebPrivateTextReceiptPolicyTest {
    @Test
    fun exactAcceptedReceiptMarksSameOriginAuthority() {
        val value = ChatGptWebPrivateTextReceiptPolicy.resolve(
            event(ok = true, detail = "private_text_v1:accepted"),
        )

        assertEquals(WebChatSendAuthority.SAME_ORIGIN_PRIVATE, value.authority)
        assertFalse(value.indeterminate)
    }

    @Test
    fun boundedUnknownReceiptRequiresReadOnlyReconciliation() {
        val source = event(ok = false, detail = "private_text_v1:unknown:network")
        val value = ChatGptWebPrivateTextReceiptPolicy.resolve(source)

        assertEquals(WebChatSendAuthority.SAME_ORIGIN_PRIVATE, value.authority)
        assertTrue(value.indeterminate)
        assertEquals(
            "发送结果正在核对，为避免重复发送，请稍候。",
            ChatGptWebPrivateTextReceiptPolicy.userDetail(source),
        )
    }

    @Test
    fun malformedOrOfficialDetailsCannotClaimPrivateAuthority() {
        listOf(
            event(ok = true, detail = "官网发送请求已提交。"),
            event(ok = false, detail = "private_text_v1:unknown:../../secret"),
            event(ok = false, detail = "private_text_v1:accepted"),
        ).forEach { source ->
            val value = ChatGptWebPrivateTextReceiptPolicy.resolve(source)
            assertEquals(WebChatSendAuthority.OFFICIAL_PAGE, value.authority)
            assertFalse(value.indeterminate)
        }
    }

    private fun event(ok: Boolean, detail: String) = ChatGptWebEvent.CommandResult(
        action = "send_prompt",
        ok = ok,
        detail = detail,
        requestId = "mcp_private1",
    )
}
