package com.elon.app.chatgptweb

import com.elon.app.WebChatConversationFile
import com.elon.app.WebChatConversationFileIndex
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.time.LocalDate

class ChatGptConversationDeletionTest {
    private val target = ChatGptWebConversation("target-chat", "Synthetic fixture", "/c/target-chat", false)
    private fun snapshot(path: String) = ChatGptWebSnapshot(
        title = "Synthetic fixture", url = "https://chatgpt.com$path", draft = "", messages = emptyList(),
        authenticated = true, composerReady = true, streaming = false, currentModel = "auto",
        attachments = emptyList(), dictationActive = false, capabilities = ChatGptWebCapabilities(emptySet()),
    )
    private fun deleted() = ChatGptWebEvent.ConversationList(emptyList(),
        removedConversationIds = setOf(target.id), deletedConversationIds = setOf(target.id))

    @Test fun protocolKeepsDeletionDistinctFromReversibleArchive() {
        val event = JSONObject().put("type", "conversation_snapshot").put("conversations", org.json.JSONArray())
            .put("removedConversationIds", org.json.JSONArray(listOf("archived-chat", target.id)))
            .put("deletedConversationIds", org.json.JSONArray(listOf(target.id, "../invalid", "https://evil.invalid")))
        val parsed = ChatGptWebProtocol.parse(JSONObject().put("schema", ChatGptWebProtocol.SCHEMA)
            .put("event", event).toString()) as ChatGptWebEvent.ConversationList
        assertEquals(setOf(target.id), parsed.deletedConversationIds)
        assertEquals(setOf("archived-chat", target.id), parsed.removedConversationIds)
    }

    @Test fun directoryCannotRecreateDeletedRowFromLateOfficialListOrSnapshot() {
        val directory = ChatGptConversationDirectory(null)
        val stale = ChatGptWebEvent.ConversationList(listOf(target))
        directory.accept(stale)
        directory.accept(deleted())
        directory.accept(stale)
        directory.observeCurrent(snapshot(target.path), LocalDate.of(2026, 9, 6))
        assertTrue(directory.index().conversations.isEmpty())
        directory.clear()
        directory.accept(stale)
        assertEquals(listOf(target), directory.index().conversations)
    }

    @Test fun observedStateEvictsFileIndexAndRejectsLateFileResponse() {
        val observed = ChatGptWebObservedState()
        observed.accept(ChatGptWebEvent.ConversationList(listOf(target)))
        val request = observed.beginConversationFilesCommand(target.path)
        val index = WebChatConversationFileIndex(target.path, request.id, listOf(
            WebChatConversationFile("message:0", "message", "fixture.txt", "file", "user", "text/plain")), false)
        observed.accept(ChatGptWebEvent.ConversationFiles(index))
        assertEquals(1, observed.snapshot().conversationFiles.size)
        observed.accept(deleted())
        observed.accept(ChatGptWebEvent.ConversationFiles(index))
        observed.accept(ChatGptWebEvent.ConversationList(listOf(target)))
        assertTrue(observed.snapshot().conversationFiles.isEmpty())
        assertTrue(observed.snapshot().conversations.isEmpty())
    }

    @Test fun historyRemovalEvictsExactCacheAndCannotBeUndoneByLateSave() {
        val values = mutableMapOf(target.path to snapshot(target.path), "/c/keep-chat" to snapshot("/c/keep-chat"))
        val removed = mutableListOf<String>()
        val repository = object : WebChatConversationSnapshotRepository {
            override fun restore(path: String) = values[path]
            override fun save(path: String, snapshot: ChatGptWebSnapshot) { values[path] = snapshot }
            override fun remove(path: String) { removed += path; values.remove(path) }
        }
        val navigation = ChatGptConversationNavigationCoordinator(repository)
        navigation.forget(setOf(target.id))
        navigation.save(target.path, snapshot(target.path))
        assertFalse(navigation.shouldAccept(snapshot("/g/g-p-fixture/c/${target.id}")))
        assertTrue(navigation.shouldAccept(snapshot("/c/keep-chat")))
        assertEquals(listOf(target.path), removed)
        assertEquals(setOf("/c/keep-chat"), values.keys)
        navigation.complete()
        assertFalse(navigation.shouldAccept(snapshot(target.path)))
        navigation.resetDeletedHistory()
        assertTrue(navigation.shouldAccept(snapshot(target.path)))
    }

    @Test fun onlyConfirmedCachedNoncurrentSelectionDispatchesDeletion() {
        val sent = mutableListOf<String>()
        val commands = object : ChatGptWebMcpCommandPort by ChatGptWebMcpTestCommandPort() {
            override fun deleteConversation(path: String, requestId: String) { sent += "$path:$requestId" }
        }
        val args = JSONObject().put("action", "chatgpt_delete_conversation").put("conversation_path", target.path)
        fun run(current: String = "/c/other-chat", rows: List<ChatGptWebConversation> = listOf(target)) =
            ChatGptWebConversationMutationMcpAction.dispatch(args, commands, snapshot(current), rows) { _, block -> block("mcp_test") }
        assertEquals("user_confirmation_required", run())
        args.put("user_confirmed", true)
        assertEquals("delete_current_conversation_active", run("/g/g-p-fixture/c/${target.id}"))
        assertEquals("delete_selection_expired", run(rows = emptyList()))
        assertNull(run())
        assertEquals(listOf("${target.path}:mcp_test"), sent)
    }

    @Test fun deletionMarkersAreBoundedAndCanonical() {
        val deleted = ChatGptDeletedConversations()
        deleted.remember((1..250).map { "fixture-$it" }.toSet() + "../invalid")
        assertEquals(200, deleted.ids().size)
        assertFalse(deleted.containsPath("/c/fixture-1"))
        assertTrue(deleted.containsPath("/g/g-p-fixture/c/fixture-250"))
        assertFalse(deleted.containsUrl("https://evil.invalid/c/fixture-250"))
    }

    @Test fun startupResumeCacheIsClearedOnlyIfItStillReferencesTheDeletedIdentity() {
        var cached = "https://chatgpt.com/g/g-p-fixture/c/${target.id}"
        var restored = "https://chatgpt.com/c/keep-chat"
        var cachedCleared = 0
        var restoredCleared = 0
        val forgotten = mutableListOf<Set<String>>()
        val caches = ChatGptConversationDeletionCaches(
            { forgotten += it }, { cached }, { cachedCleared += 1 },
            { restored }, { restoredCleared += 1 },
        )
        caches.accept(setOf(target.id))
        assertEquals(1, cachedCleared)
        assertEquals(0, restoredCleared)
        cached = "https://chatgpt.com/c/keep-chat"
        restored = "https://chatgpt.com/c/${target.id}"
        caches.accept(setOf(target.id))
        assertEquals(1, cachedCleared)
        assertEquals(1, restoredCleared)
        assertEquals(listOf(setOf(target.id), setOf(target.id)), forgotten)
        caches.accept(emptySet())
        assertEquals(2, forgotten.size)
    }
}
