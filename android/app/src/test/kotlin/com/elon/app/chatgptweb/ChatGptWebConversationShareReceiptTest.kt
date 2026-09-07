package com.elon.app.chatgptweb

import com.elon.app.WebChatConversationSharePolicy
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebConversationShareReceiptTest {
    private val id = "44444444-4444-4444-8444-444444444444"
    private val project = "g-p-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    private val path = "/g/$project/c/$id"
    private val url = "https://chatgpt.com/g/$project-${"s".repeat(124)}/shared/c/$id?owner_user_id=${"u".repeat(160)}"

    @Test fun validatedMemberUrlSurvivesTheActualProtocolDetailBoundary() {
        val raw = "project_share_link_ready:$url"
        assertTrue(raw.length > 160)
        val detail = ChatGptWebPrivateProtocolEvidence.detail("share_conversation", raw)
        assertEquals(raw, detail)
        assertEquals(url, WebChatConversationSharePolicy.resultUrl(detail, path))
    }

    @Test fun receiptBoundaryRejectsUnvalidatedUrlsAndExtraData() {
        for (value in listOf("project_share_link_ready:$url&token=secret", "project_share_link_ready:${url}x",
                "project_share_link_ready:" + url.replace("chatgpt.com", "example.com"),
                "share_link_ready:javascript:alert(1)", "share_link_ready:https://chatgpt.com/share/$id?other=1",
                "share_link_ready:" + "x".repeat(600))) {
            assertEquals(value, "share_result_unconfirmed", ChatGptWebPrivateProtocolEvidence.detail("share_conversation", value))
        }
    }

    @Test fun publicReceiptsAndErrorsAreKeptWhileOtherActionsRetainTheirLimit() {
        val raw = "share_link_ready:https://chatgpt.com/share/$id"
        assertEquals(raw, ChatGptWebPrivateProtocolEvidence.detail("share_conversation", raw))
        assertEquals("share_http_403", ChatGptWebPrivateProtocolEvidence.detail("share_conversation", "share_http_403"))
        assertEquals("x".repeat(160), ChatGptWebPrivateProtocolEvidence.detail("send_prompt", "x".repeat(500)))
    }
}
