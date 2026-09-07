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

    private val project = "g-p-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    private val otherProject = "g-p-bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    private val memberPath = "/g/$project/c/$id"
    private val memberUrl = "https://chatgpt.com/g/$project-fixture/shared/c/$id?owner_user_id=user-synthetic"

    @Test fun shareSelectionEncodesTheConfirmedAudienceWithoutTrustingADisplayName() {
        assertEquals("/c/$id", WebChatConversationSharePolicy.sharePath("/c/$id", null))
        assertEquals(memberPath, WebChatConversationSharePolicy.sharePath("/c/$id", project))
        assertEquals(memberPath, WebChatConversationSharePolicy.sharePath("/g/$project-fixture/c/$id", null))
        assertEquals(memberPath, WebChatConversationSharePolicy.sharePath(memberPath, "$project-fixture"))
        assertNull(WebChatConversationSharePolicy.sharePath(memberPath, otherProject))
        assertNull(WebChatConversationSharePolicy.sharePath("/c/$id", "not-a-project"))
        assertNull(WebChatConversationSharePolicy.sharePath("/g/g-p-unknown/c/$id", null))
        assertNull(WebChatConversationSharePolicy.sharePath("/c/local-draft", project))
        assertTrue(WebChatConversationSharePolicy.membersOnly(memberPath))
        assertFalse(WebChatConversationSharePolicy.membersOnly("/c/$id"))
    }

    @Test fun memberReceiptCannotTurnIntoAPublicShareOrADifferentConversation() {
        assertEquals(memberUrl, WebChatConversationSharePolicy.resultUrl("project_share_link_ready:$memberUrl", memberPath))
        assertNull(WebChatConversationSharePolicy.resultUrl("share_link_ready:$url", memberPath))
        assertNull(WebChatConversationSharePolicy.resultUrl("project_share_link_ready:$memberUrl", "/c/$id"))
        assertNull(WebChatConversationSharePolicy.resultUrl("project_share_link_ready:$memberUrl"))
        for (candidate in listOf(memberUrl.replace(project, otherProject), memberUrl.replace(id, "55555555-5555-4555-8555-555555555555"),
                "$memberUrl&other=1", "$memberUrl#fragment", memberUrl.replace("chatgpt.com", "example.com"),
                memberUrl.replace("user-synthetic", "user-synthetic%26other"), memberUrl.replace("/shared/c/", "/c/"),
                memberUrl.replace("https://", "https://user@"))) {
            assertNull(candidate, WebChatConversationSharePolicy.resultUrl("project_share_link_ready:$candidate", memberPath))
        }
    }

    @Test fun projectContextCanUseCanonicalRouteButCannotSwitchProjectsSilently() {
        assertTrue(WebChatConversationSharePolicy.sameConversation(memberPath, "https://chatgpt.com/c/$id"))
        assertTrue(WebChatConversationSharePolicy.sameConversation(memberPath, "https://chatgpt.com/g/$project-fixture/c/$id"))
        assertFalse(WebChatConversationSharePolicy.sameConversation(memberPath, "https://chatgpt.com/g/$otherProject/c/$id"))
    }
}
