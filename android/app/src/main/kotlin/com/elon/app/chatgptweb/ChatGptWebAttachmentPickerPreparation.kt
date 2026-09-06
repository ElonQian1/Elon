package com.elon.app.chatgptweb

import com.elon.app.PendingAttachment
import com.elon.app.WebBridgeDocumentSession
import com.elon.app.WebChatAttachmentSelection
import com.elon.app.WebChatAttachmentSelectionKind
import java.util.UUID
import org.json.JSONObject

internal class ChatGptWebAttachmentPickerPreparation(
    private val document: () -> WebBridgeDocumentSession.Snapshot,
    private val href: () -> String?,
    private val available: () -> Boolean,
    private val dispatch: (String) -> Unit,
    private val schedule: (Runnable, Long) -> Unit,
    private val remove: (Runnable) -> Unit,
    private val now: () -> Long,
) {
    private var pending: Selection? = null

    private inner class Selection(
        val id: String,
        val state: WebBridgeDocumentSession.Snapshot,
        val url: String,
        val expiresAt: Long,
    ) : WebChatAttachmentSelection {
        var file: PendingAttachment? = null
        var size = 0L
        var modified = 0L
        val expiry = Runnable { cancel() }

        override fun selected(attachments: List<PendingAttachment>) {
            if (pending !== this) return
            if (file != null || attachments.size != 1 || !current(this)) return cancel()
            val selected = attachments.single()
            if (!selected.file.isFile) return cancel()
            file = selected
            size = selected.file.length()
            modified = selected.file.lastModified()
        }

        override fun cancel() {
            if (pending !== this) return
            pending = null
            remove(expiry)
            execute(this, "cancelSelection", JSONObject.quote(id))
            file = null
        }
    }

    fun begin(kind: WebChatAttachmentSelectionKind): WebChatAttachmentSelection? {
        cancel()
        val state = document()
        val url = href() ?: return null
        if (!available() || !state.adapterCurrent || !url.startsWith("https://chatgpt.com/")) return null
        val selection = Selection("selection_${UUID.randomUUID().toString().replace("-", "")}",
            state, url, now() + 120_000L)
        pending = selection
        schedule(selection.expiry, 120_000L)
        val intent = JSONObject().put("id", selection.id).put("kind", kind.wireValue)
            .put("documentToken", state.documentToken).put("href", url).toString()
        execute(selection, "beginSelection", JSONObject.quote(intent))
        return selection
    }

    fun take(file: PendingAttachment): String? {
        val selection = pending ?: return null
        // Only the exact native selection can consume this preparation, not a later file at a reused path.
        if (!current(selection) || selection.file !== file || file.file.length() != selection.size ||
            file.file.lastModified() != selection.modified
        ) {
            selection.cancel()
            return null
        }
        pending = null
        remove(selection.expiry)
        selection.file = null
        return selection.id
    }

    fun cancel() = pending?.cancel() ?: Unit

    private fun current(selection: Selection): Boolean {
        val state = document()
        return available() && now() < selection.expiresAt && href() == selection.url &&
            state.documentToken == selection.state.documentToken && state.pageGeneration == selection.state.pageGeneration
    }

    private fun execute(selection: Selection, method: String, argument: String) {
        val token = JSONObject.quote(selection.state.documentToken)
        runCatching { dispatch("if(window.__elonChatGptDocumentToken===$token)" +
            "window.__elonChatGptPrivateAttachmentSend?.$method($argument);") }
    }
}
