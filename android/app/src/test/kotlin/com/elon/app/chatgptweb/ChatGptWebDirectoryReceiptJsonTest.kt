package com.elon.app.chatgptweb

import com.elon.app.WebBridgeDocumentSession
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebDirectoryReceiptJsonTest {
    @Test fun unrelatedPageCommandCannotHideNativeDirectoryCompletion() {
        var now = 100L
        val state = ChatGptWebObservedState(nowMs = { now })
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_fixture"))
        state.accept(ChatGptWebEvent.CommandResult("list_conversations", true, "directory_ready"))
        now = 200L
        state.accept(ChatGptWebEvent.CommandResult("set_skin_mode", true, ""))
        val receipt = ChatGptWebMcpSnapshotJson.navigation(state.snapshot()).getJSONObject("last_directory_refresh")
        assertEquals("set_skin_mode", state.snapshot().lastCommand?.action)
        assertTrue(receipt.getBoolean("ok"))
        assertEquals("directory_ready", receipt.getString("code"))
        assertEquals(100L, receipt.getLong("observed_at_ms"))
        assertEquals("native", receipt.getString("source"))
    }

    @Test fun missingOrReplacedDocumentCannotExposeOldCompletion() {
        val state = ChatGptWebObservedState()
        assertTrue(ChatGptWebMcpSnapshotJson.navigation(state.snapshot()).isNull("last_directory_refresh"))
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_fixture"))
        assertTrue(ChatGptWebMcpSnapshotJson.navigation(state.snapshot()).isNull("last_directory_refresh"))
        state.accept(ChatGptWebEvent.CommandResult("list_conversations", true, "directory_ready"))
        state.updateDocument(WebBridgeDocumentSession.Snapshot(2, 0, "doc_replacement"))
        assertTrue(ChatGptWebMcpSnapshotJson.navigation(state.snapshot()).isNull("last_directory_refresh"))
    }

    @Test fun failedMcpReadIsDistinguishedAndArbitraryDetailIsNotExported() {
        val state = ChatGptWebObservedState()
        state.updateDocument(WebBridgeDocumentSession.Snapshot(1, 1, "doc_fixture"))
        state.accept(ChatGptWebEvent.CommandResult("list_conversations", false, "untrusted response text", "mcp_fixture"))
        val receipt = ChatGptWebMcpSnapshotJson.navigation(state.snapshot()).getJSONObject("last_directory_refresh")
        assertFalse(receipt.getBoolean("ok"))
        assertEquals("", receipt.getString("code"))
        assertEquals("mcp", receipt.getString("source"))
        assertEquals(setOf("ok", "code", "observed_at_ms", "source"), receipt.keys().asSequence().toSet())
    }
}
