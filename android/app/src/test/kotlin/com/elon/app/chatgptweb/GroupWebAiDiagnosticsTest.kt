package com.elon.app.chatgptweb

import com.elon.app.WebChatProviderId
import org.junit.Assert.*
import org.junit.Test

class GroupWebAiDiagnosticsTest {
    @Test fun sendReceiptKeepsFailureClassNotUntrustedDetail() {
        val records = mutableListOf<Map<String, Any?>>()
        val diagnostics = GroupWebAiDiagnostics(WebChatProviderId.CHATGPT_WEB) { _, values -> records += values }
        diagnostics.sendReceipt(ChatGptWebEvent.CommandResult("send_prompt", false,
            "发送按钮尚未就绪，请返回官网重试。 [private-user-content]"))
        assertEquals("send_button_not_ready", records.last()["reason"])
        diagnostics.sendReceipt(ChatGptWebEvent.CommandResult("send_prompt", false, "private-user-content"))
        assertEquals("unknown", records.last()["reason"])
        diagnostics.sendReceipt(ChatGptWebEvent.CommandResult("send_prompt", true, "private_text_v1:accepted"))
        assertEquals("same_origin_private", records.last()["authority"])
        assertEquals(true, records.last()["ok"])
        assertFalse(records.toString().contains("private-user-content"))
    }

    private fun snapshot() = ChatGptWebSnapshot(
        title = "private-title", url = "https://chatgpt.com/?temporary-chat=true&secret=private-query",
        draft = "private-draft", messages = listOf(ChatGptWebMessage("private-id", "user", "private-text", "completed", emptyList())),
        authenticated = true, composerReady = true, streaming = false, currentModel = "private-model",
        attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities.EMPTY,
    )

    @Test fun diagnosticsContainOnlyFlagsNotContentOrUrls() {
        val records = mutableListOf<Map<String, Any?>>()
        val diagnostics = GroupWebAiDiagnostics(WebChatProviderId.CHATGPT_WEB) { phase, values ->
            assertEquals("group_web_ai", phase); records += values
        }
        diagnostics.snapshot(snapshot())
        val value = records.single()
        assertEquals(true, value["temporary_route"])
        assertEquals(true, value["has_draft"])
        assertEquals(true, value["has_messages"])
        assertEquals(false, value["ready"])
        assertFalse(value.values.any { it.toString().contains("private-") || it.toString().contains("https:") })
    }

    @Test fun repeatedSnapshotsAreDeduplicatedAndTransitionsAreBounded() {
        val records = mutableListOf<Map<String, Any?>>()
        val diagnostics = GroupWebAiDiagnostics(WebChatProviderId.CHATGPT_WEB) { _, values -> records += values }
        repeat(100) { diagnostics.snapshot(snapshot()) }
        assertEquals(1, records.size)
        repeat(100) { diagnostics.snapshot(snapshot().copy(streaming = it % 2 == 0)) }
        assertEquals(24, records.size)
        diagnostics.stage("prepare_timeout")
        diagnostics.stage("failed_before_authorize")
        assertEquals("failed_before_authorize", records.last()["stage"])
        assertEquals(26, records.size)
    }

    @Test fun readinessUsesLiveDocumentWhileDiagnosticsRemainRedacted() {
        val records = mutableListOf<Map<String, Any?>>()
        val diagnostics = GroupWebAiDiagnostics(WebChatProviderId.CHATGPT_WEB) { _, values -> records += values }
        diagnostics.snapshot(snapshot().copy(url = "https://chatgpt.com/", draft = "", messages = emptyList()),
            "https://chatgpt.com/?temporary-chat=true")
        assertEquals(true, records.single()["ready"])
        assertEquals(true, records.single()["temporary_route"])
        assertFalse(records.single().values.any { it.toString().contains("https:") })
    }
}
