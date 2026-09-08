package com.elon.app.chatgptweb

import com.elon.app.WebChatLibraryPresentation
import com.elon.app.WebChatProductionBuiltInCatalog
import com.elon.app.WebChatProductionFeatureCompletionPolicy
import com.elon.app.WebChatProviderId
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.lang.reflect.Proxy

class ChatGptWebLibraryTest {
    private val handle = "library_" + "a".repeat(32)
    private val download = "download_" + "b".repeat(32)
    private fun payload(id: String = "mcp_test") = JSONObject().put("version", 1).put("requestId", id)
        .put("directoryHandle", "").put("query", "").put("breadcrumbs", JSONArray())
        .put("items", JSONArray().put(JSONObject().put("handle", handle).put("kind", "file")
            .put("name", "fixture.txt").put("mediaType", "text/plain").put("sizeBytes", 7).put("downloadHandle", download)))
        .put("hasMore", false).put("partial", false).put("stale", false)

    @Test fun parsesNativeFileRowsAndExportsOnlyOpaqueSelectors() {
        val state = ChatGptWebLibraryProtocol.parse(payload())!!
        assertEquals("fixture.txt", state.items.single().name)
        assertEquals(7L, state.items.single().sizeBytes)
        val json = ChatGptWebLibraryProtocol.json(state) as JSONObject
        assertEquals(handle, json.getJSONArray("items").getJSONObject(0).getString("handle"))
        assertFalse(json.has("cursor"))
        assertFalse(json.has("conversation_path"))
    }

    @Test fun attachesOnlyExplicitlySelectedObservedFilesWithBooleanCapability() {
        val value = payload()
        val row = value.getJSONArray("items").getJSONObject(0)
        row.put("canAttach", "true")
        assertFalse(ChatGptWebLibraryProtocol.parse(value)!!.items.single().canAttach)
        row.put("canAttach", true)
        val state = ChatGptWebObservedState.Snapshot.EMPTY.copy(libraryFiles = ChatGptWebLibraryProtocol.parse(value))
        val calls = mutableListOf<String>()
        val commands = Proxy.newProxyInstance(javaClass.classLoader, arrayOf(ChatGptWebMcpCommandPort::class.java)) { _, method, args ->
            calls += method.name
            assertEquals(handle, args!![0])
            null
        } as ChatGptWebMcpCommandPort
        val request = JSONObject().put("action", "chatgpt_attach_library_file").put("file_handle", handle)
        val dispatch: (String, (String) -> Unit) -> Unit = { action, run ->
            assertEquals("attach_library_file", action)
            run("mcp_attach")
        }
        assertEquals("user_confirmation_required", ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
        request.put("confirmed", true).put("file_handle", "library_" + "c".repeat(32))
        assertEquals("library_selection_expired", ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
        request.put("file_handle", handle)
        assertNull(ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
        assertEquals(listOf("attachLibraryFile"), calls)
        val json = ChatGptWebLibraryProtocol.json(state.libraryFiles) as JSONObject
        assertTrue(json.getJSONArray("items").getJSONObject(0).getBoolean("can_attach"))
    }

    @Test fun invalidRowsArePartialRatherThanEmptySuccess() {
        val value = payload()
        value.getJSONArray("items").put(JSONObject().put("handle", "raw-library-id").put("name", "other"))
        val state = ChatGptWebLibraryProtocol.parse(value)!!
        assertEquals(1, state.items.size)
        assertTrue(state.partial)
        assertTrue(WebChatLibraryPresentation.status(state, false, false).contains("部分"))
    }

    @Test fun rejectsForgedBreadcrumbsAndVersions() {
        assertNull(ChatGptWebLibraryProtocol.parse(payload().put("version", 2)))
        assertNull(ChatGptWebLibraryProtocol.parse(payload().put("directoryHandle", handle)))
        assertNull(ChatGptWebLibraryProtocol.parse(payload().put("query", "x".repeat(201))))
        assertNull(ChatGptWebLibraryProtocol.parse(payload().put("requestId", "not-a-request")))
    }

    @Test fun acceptsOnlyLatestPendingLibrarySnapshotAndPreservesItOnRefreshFailure() {
        val state = ChatGptWebObservedState(nowMs = { 1000L })
        val old = state.beginCommand(ChatGptWebLibraryProtocol.ACTION)
        val fresh = state.beginCommand(ChatGptWebLibraryProtocol.ACTION)
        state.accept(ChatGptWebEvent.LibraryFiles(ChatGptWebLibraryProtocol.parse(payload(old.id))!!))
        assertNull(state.snapshot().libraryFiles)
        state.accept(ChatGptWebEvent.LibraryFiles(ChatGptWebLibraryProtocol.parse(payload(fresh.id))!!))
        state.accept(ChatGptWebEvent.CommandResult(ChatGptWebLibraryProtocol.ACTION, false, "library_read_failed", fresh.id))
        assertEquals(fresh.id, state.snapshot().libraryFiles?.requestId)
        val late = payload(fresh.id).put("items", JSONArray())
        state.accept(ChatGptWebEvent.LibraryFiles(ChatGptWebLibraryProtocol.parse(late)!!))
        assertEquals(1, state.snapshot().libraryFiles?.items?.size)
        val pending = state.beginCommand(ChatGptWebLibraryProtocol.ACTION)
        state.clearConversationHistory()
        state.accept(ChatGptWebEvent.LibraryFiles(ChatGptWebLibraryProtocol.parse(payload(pending.id))!!))
        assertNull(state.snapshot().libraryFiles)
    }

    @Test fun dispatchesDownloadsOnlyForAnObservedMatchingFileHandle() {
        val calls = mutableListOf<String>()
        val commands = Proxy.newProxyInstance(javaClass.classLoader, arrayOf(ChatGptWebMcpCommandPort::class.java)) { _, method, _ ->
            calls += method.name
            null
        } as ChatGptWebMcpCommandPort
        val state = ChatGptWebObservedState.Snapshot.EMPTY.copy(libraryFiles = ChatGptWebLibraryProtocol.parse(payload()))
        val request = JSONObject().put("action", "chatgpt_download_library_file")
            .put("file_handle", handle).put("download_handle", "download_" + "c".repeat(32))
        val dispatch: (String, (String) -> Unit) -> Unit = { action, run ->
            assertEquals("download_library_file", action)
            run("mcp_download")
        }
        assertEquals("download_selection_expired", ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
        assertTrue(calls.isEmpty())
        request.put("download_handle", download)
        assertNull(ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
        assertEquals(listOf("downloadLibraryFile"), calls)
        var now = 1000L
        val timing = ChatGptWebObservedState(nowMs = { now })
        timing.beginCommand("download_conversation_file")
        timing.beginCommand("download_library_file")
        now += 60_000L
        assertTrue(timing.snapshot().commandRequests.all { it.status == ChatGptWebObservedState.CommandRequest.PENDING })
        now += 135_000L
        assertTrue(timing.snapshot().commandRequests.all { it.status == ChatGptWebObservedState.CommandRequest.TIMED_OUT })
    }

    @Test fun nativePresetDoesNotRequireAReadOfTheOfficialNavigationMenu() {
        val library = WebChatProductionBuiltInCatalog.features(WebChatProviderId.CHATGPT_WEB).single { it.kind == "library" }
        assertEquals("web-chat-feature:library", library.nativeSelector)
        assertFalse(WebChatProductionFeatureCompletionPolicy.requiresOfficialCompletion(library.kind))
        assertTrue(WebChatProductionBuiltInCatalog.features(WebChatProviderId.GOOGLE_WEB).isEmpty())
    }

    @Test fun failedRefreshReportsFailureWithoutReplacingVisibleFiles() {
        val page = ChatGptWebLibraryProtocol.parse(payload())!!
        assertTrue(WebChatLibraryPresentation.status(page, false, true).contains("保留已有"))
        assertFalse(WebChatLibraryPresentation.status(null, false, true).contains("为空"))
        assertEquals("text/plain · 7 B", WebChatLibraryPresentation.subtitle(page.items.single()))
    }

    @Test fun mutationCapabilitiesRequireBooleanFileEvidenceAndExplicitConfirmation() {
        val value = payload()
        val row = value.getJSONArray("items").getJSONObject(0)
        row.put("canRename", "true").put("canTrash", true)
        val entry = ChatGptWebLibraryProtocol.parse(value)!!.items.single()
        assertFalse(entry.canRename)
        assertTrue(entry.canTrash)
        val calls = mutableListOf<String>()
        val commands = Proxy.newProxyInstance(javaClass.classLoader, arrayOf(ChatGptWebMcpCommandPort::class.java)) { _, method, _ ->
            calls += method.name
            null
        } as ChatGptWebMcpCommandPort
        val state = ChatGptWebObservedState.Snapshot.EMPTY.copy(libraryFiles = ChatGptWebLibraryProtocol.parse(value))
        val request = JSONObject().put("action", "chatgpt_mutate_library_file").put("file_handle", handle).put("operation", "trash")
        val dispatch: (String, (String) -> Unit) -> Unit = { action, run ->
            assertEquals("mutate_library_file", action)
            run("mcp_mutate")
        }
        assertEquals("user_confirmation_required", ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
        request.put("confirmed", "true")
        assertEquals("user_confirmation_required", ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
        request.put("confirmed", true)
        assertNull(ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
        assertEquals(listOf("mutateLibraryFile"), calls)
        request.put("operation", "rename").put("name", "renamed.txt")
        assertEquals("library_mutation_unsupported", ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
        request.put("operation", "delete_permanently")
        assertEquals("library_mutation_unsupported", ChatGptWebLibraryCommands.control(request, state, commands, dispatch))
    }

    @Test fun renameValidationAndReceiptDeadlineDoNotPermitUnsafeOrPrematureWrites() {
        assertTrue(ChatGptWebLibraryCommands.validName("fixture v2.txt"))
        listOf("", "..", "a/b", "a\\b", "a\nb", " a", "a".repeat(181)).forEach {
            assertFalse(ChatGptWebLibraryCommands.validName(it))
        }
        var now = 1000L
        val state = ChatGptWebObservedState(nowMs = { now })
        state.beginCommand("mutate_library_file")
        now += 25_000
        assertEquals(ChatGptWebObservedState.CommandRequest.PENDING, state.snapshot().commandRequests.single().status)
        now += 11_000
        assertEquals(ChatGptWebObservedState.CommandRequest.TIMED_OUT, state.snapshot().commandRequests.single().status)
    }
}
