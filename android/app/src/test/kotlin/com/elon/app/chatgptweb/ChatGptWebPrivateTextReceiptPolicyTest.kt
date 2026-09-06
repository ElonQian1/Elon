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
    fun runtimeAcceptanceIsNotAnIndependentPrivatePost() {
        val source = event(ok = true, detail = "official_runtime_v1:accepted")
        val value = ChatGptWebPrivateTextReceiptPolicy.resolve(source)
        assertEquals(WebChatSendAuthority.OFFICIAL_PAGE, value.authority)
        assertFalse(value.indeterminate)
        assertEquals("官网已确认提交。", ChatGptWebPrivateTextReceiptPolicy.userDetail(source))
    }

    @Test
    fun runtimeUnknownPreventsRetryAndKeepsOfficialOwnership() {
        val source = event(ok = false, detail = "official_runtime_v1:unknown:timeout")
        val value = ChatGptWebPrivateTextReceiptPolicy.resolve(source)
        assertEquals(WebChatSendAuthority.OFFICIAL_PAGE, value.authority)
        assertTrue(value.indeterminate)
        assertEquals("发送结果正在核对，为避免重复发送，请稍候。", ChatGptWebPrivateTextReceiptPolicy.userDetail(source))
    }

    @Test
    fun runtimeGateRejectionRemainsAnOrdinaryUnsentFailure() {
        val source = event(ok = false, detail = "official_runtime_v1:rejected:not_ready")
        assertFalse(ChatGptWebPrivateTextReceiptPolicy.resolve(source).indeterminate)
        assertEquals("会话暂未准备好，草稿已保留，请稍后重试。", ChatGptWebPrivateTextReceiptPolicy.userDetail(source))
        assertFalse(ChatGptWebPrivateTextReceiptPolicy.resolve(event(
            ok = false, detail = "official_runtime_v1:unknown:../../secret",
        )).indeterminate)
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
