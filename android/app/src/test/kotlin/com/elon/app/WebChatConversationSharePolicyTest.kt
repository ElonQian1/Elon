package com.elon.app

import org.junit.Assert.*
import org.junit.Test

class WebChatConversationSharePolicyTest {
    private val id = "44444444-4444-4444-8444-444444444444"
    private val url = "https://chatgpt.com/share/$id"

    @Test fun acceptsOnlyAnAcknowledgedOfficialPublicUrl() {
        assertEquals(url, WebChatConversationSharePolicy.resultUrl("share_link_ready:$url"))
        for (value in listOf(null, url, "share_result_unconfirmed", "share_link_ready:javascript:alert(1)",
                "share_link_ready:$url?token=secret", "share_link_ready:$url#other", "share_link_ready:$url/",
                "share_link_ready:https://user@chatgpt.com/share/$id", "share_link_ready:https://example.com/share/$id",
                "share_link_ready:https://chatgpt.com/c/$id")) {
            assertNull(value, WebChatConversationSharePolicy.resultUrl(value))
        }
    }

    @Test fun pendingOrFailedPublicationDoesNotEncourageAutomaticReplay() {
        for (code in listOf("share_result_unconfirmed", "share_cooldown")) {
            val text = WebChatConversationSharePolicy.errorMessage(code)
            assertTrue(text.contains("可能已经创建"))
            assertTrue(text.contains("暂不重复创建"))
        }
        assertTrue(WebChatConversationSharePolicy.errorMessage("share_scope_unconfirmed").contains("尚未确认"))
        assertFalse(WebChatConversationSharePolicy.errorMessage("share_scope_unconfirmed").contains("没有功能"))
    }

    @Test fun conversationIdentityCannotCrossProviders() {
        assertTrue(WebChatConversationSharePolicy.sameConversation("/c/$id", "https://chatgpt.com/c/$id"))
        assertFalse(WebChatConversationSharePolicy.sameConversation("/c/$id", "https://example.com/c/$id"))
        assertFalse(WebChatConversationSharePolicy.sameConversation("/c/$id", "https://chatgpt.com/"))
        for (value in listOf("http://chatgpt.com/c/$id", "https://chatgpt.com.evil/c/$id",
                "https://user@chatgpt.com/c/$id", "https://chatgpt.com:444/c/$id", "/c/$id")) {
            assertFalse(value, WebChatConversationSharePolicy.sameConversation("/c/$id", value))
        }
        assertTrue(WebChatConversationSharePolicy.sameConversation("/c/$id", "https://chatgpt.com/g/g-p-fixture/c/$id"))
    }
}
