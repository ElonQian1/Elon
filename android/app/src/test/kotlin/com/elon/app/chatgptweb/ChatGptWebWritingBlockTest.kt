package com.elon.app.chatgptweb

import com.elon.app.WebChatTextBlock
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebWritingBlockTest {
    private val path = "/c/11111111-1111-4111-8111-111111111111"
    private val ticket = "wb_" + "a".repeat(32)
    private fun request() = JSONObject().put("operation", "prepare").put("path", path)
        .put("id", "block-a").put("messageId", "message-a").put("content", "  Original\r\n")
    private fun event(id: String) = JSONObject().put("type", "writing_block").put("version", 1)
        .put("requestId", id).put("path", path).put("ticket", ticket).put("id", "block-a")
        .put("messageId", "message-a").put("pending", false)

    @Test fun originalIdentitySurvivesCacheButIsNotASaveTicket() {
        val source = WebChatTextBlock("block-a", "writing", "Example", "", "\r\n", true, "message-a")
        assertEquals(source, WebChatTextBlock.parse(source.toJson()))
        assertFalse(source.toJson().has("ticket"))
        assertNull(WebChatTextBlock.parse(source.copy(kind = "code").toJson())?.sourceMessageId)
        assertNull(WebChatTextBlock.parse(source.toJson().put("sourceMessageId", "../other"))?.sourceMessageId)
    }

    @Test fun requestHasAnExactSchemaAndPreservesWhitespace() {
        assertEquals("  Original\r\n", ChatGptWebWritingBlockProtocol.request(request())?.getString("content"))
        assertNotNull(ChatGptWebWritingBlockProtocol.request(request().put("content", "")))
        assertNull(ChatGptWebWritingBlockProtocol.request(request().put("scope", "other")))
        assertNull(ChatGptWebWritingBlockProtocol.request(request().put("content", "\uD800")))
        assertNull(ChatGptWebWritingBlockProtocol.request(request().put("path", "/g/project" + path)))
        val save = JSONObject().put("operation", "save").put("path", path).put("ticket", ticket).put("content", "updated")
        assertNotNull(ChatGptWebWritingBlockProtocol.request(save))
        assertNull(ChatGptWebWritingBlockProtocol.request(save.put("ticket", "cached")))
    }

    @Test fun receiptHasNoBodyAndMustMatchAPendingCommand() {
        val state = ChatGptWebObservedState()
        val request = state.beginCommand(ChatGptWebWritingBlockProtocol.ACTION)
        val value = ChatGptWebWritingBlockProtocol.parse(event(request.id))!!
        assertNull(ChatGptWebWritingBlockProtocol.parse(event(request.id).put("content", "unexpected")))
        state.accept(ChatGptWebEvent.WritingBlock(value.copy(requestId = "mcp_old")))
        assertNull(state.snapshot().writingBlock)
        state.accept(ChatGptWebEvent.WritingBlock(value))
        assertEquals(value, state.snapshot().writingBlock)
        state.clearConversationHistory()
        assertNull(state.snapshot().writingBlock)
        state.accept(ChatGptWebEvent.WritingBlock(value))
        assertNull(state.snapshot().writingBlock)
    }

    @Test fun savingDoesNotUseTheComposerAdmissionGroup() {
        assertEquals(ChatGptWebOperationReadiness.Requirement.ACCOUNT_READ,
            ChatGptWebOperationReadiness.requirement("chatgpt_writing_block"))
        assertNull(ChatGptWebOperationReadiness.rejection("chatgpt_writing_block", null, true, false))
    }
}
