package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebCanvasComment
import com.elon.app.chatgptweb.ChatGptWebCanvasDocument
import com.elon.app.chatgptweb.ChatGptWebCanvasDocumentProtocol
import com.elon.app.chatgptweb.ChatGptWebCanvasDocuments
import org.json.JSONArray
import org.json.JSONObject

internal class WebChatCanvasDraft(val path: String, val scope: String, document: ChatGptWebCanvasDocument) {
    var base = document
        private set
    var content = document.content
        private set
    var comments = document.comments.toList()
        private set
    private val brokenAnchors = mutableSetOf<String>()
    private var pendingCommentDismissal: String? = null
    val needsRepair: Set<String> get() = brokenAnchors.toSet()
    val changed: Boolean get() = content != base.content || comments != base.comments || brokenAnchors.isNotEmpty()

    fun replace(start: Int, count: Int, inserted: String) {
        require(start >= 0 && count >= 0 && start <= content.length - count)
        val end = start + count
        val next = content.replaceRange(start, end, inserted)
        require(next.length <= ChatGptWebCanvasDocumentProtocol.MAX_CONTENT)
        val delta = inserted.length - count
        // Android selection offsets use UTF-16; the transport converts them to server code points.
        comments = comments.map { comment ->
            val updated = when {
                end <= comment.start -> comment.copy(start = comment.start + delta, end = comment.end + delta)
                start >= comment.end -> comment
                else -> comment.copy(
                    start = if (comment.start >= start) start else comment.start,
                    end = if (comment.end <= end) start + inserted.length else comment.end + delta,
                )
            }
            if ((comment.start < comment.end && updated.start == updated.end) ||
                !ChatGptWebCanvasDocumentProtocol.boundary(next, updated.start) ||
                !ChatGptWebCanvasDocumentProtocol.boundary(next, updated.end)) brokenAnchors += comment.id
            updated
        }
        content = next
        if (content == base.content && comments == base.comments) brokenAnchors.clear()
    }

    fun reanchor(id: String, start: Int, end: Int): Boolean {
        if (start >= end || !ChatGptWebCanvasDocumentProtocol.boundary(content, start) ||
            !ChatGptWebCanvasDocumentProtocol.boundary(content, end) || comments.none { it.id == id }) return false
        comments = comments.map { if (it.id == id) it.copy(start = start, end = end) else it }
        brokenAnchors -= id
        return true
    }

    fun matches(index: ChatGptWebCanvasDocuments): Boolean = index.path == path && index.scope == scope &&
        index.documents.firstOrNull { it.id == base.id } == base

    fun saveRequest(index: ChatGptWebCanvasDocuments): JSONObject? {
        if (!matches(index) || index.unconfirmedWrite || brokenAnchors.isNotEmpty()) return null
        return selection(index, "save").put("content", content)
            .put("comments", JSONArray(comments.map(ChatGptWebCanvasComment::json)))
    }

    fun renameRequest(index: ChatGptWebCanvasDocuments, title: String): JSONObject? {
        if (!matches(index) || index.unconfirmedWrite || !ChatGptWebCanvasDocumentProtocol.validTitle(title)) return null
        return selection(index, "rename").put("title", title)
    }

    fun acceptRename(document: ChatGptWebCanvasDocument): Boolean {
        if (document.title == base.title || document.documentVersion < base.documentVersion ||
            document.copy(title = base.title, documentVersion = base.documentVersion) != base) return false
        // A title-only server change must not replace unsaved text or repaired comment anchors.
        base = document
        return true
    }

    fun dismissCommentRequest(index: ChatGptWebCanvasDocuments, id: String): JSONObject? {
        if (!matches(index) || index.unconfirmedWrite || base.comments.none { it.id == id }) return null
        pendingCommentDismissal = id
        return selection(index, "dismiss_comment").put("commentId", id)
    }

    fun acceptCommentDismissal(document: ChatGptWebCanvasDocument): Boolean {
        val id = pendingCommentDismissal ?: return false
        if (document.documentVersion <= base.documentVersion ||
            document.copy(comments = base.comments, documentVersion = base.documentVersion) != base ||
            document.comments != base.comments.filterNot { it.id == id }) return false
        // Only the explicitly dismissed comment is removed from the unsaved native draft.
        base = document
        comments = comments.filterNot { it.id == id }
        brokenAnchors -= id
        pendingCommentDismissal = null
        return true
    }

    fun selection(index: ChatGptWebCanvasDocuments, operation: String) = JSONObject()
        .put("operation", operation).put("path", path).put("scope", scope)
        .put("ticket", index.ticket).put("id", base.id)

    fun generationRequest(index: ChatGptWebCanvasDocuments, prompt: String, start: Int, end: Int): JSONObject? {
        if (changed || !matches(index) || index.unconfirmedWrite || start > end ||
            !ChatGptWebCanvasDocumentProtocol.boundary(content, start) || !ChatGptWebCanvasDocumentProtocol.boundary(content, end)) return null
        return ChatGptWebCanvasDocumentProtocol.request(selection(index, "generate")
            .put("prompt", prompt).put("start", start).put("end", end))
    }

    fun adopt(document: ChatGptWebCanvasDocument) {
        require(document.id == base.id)
        base = document
        content = document.content
        comments = document.comments.toList()
        brokenAnchors.clear()
        pendingCommentDismissal = null
    }

    fun rebase(document: ChatGptWebCanvasDocument) {
        require(document.id == base.id)
        val previous = base
        base = document
        pendingCommentDismissal = null
        if (document == previous) return
        // A newer server comment may describe different text. Require an explicit new selection.
        comments = document.comments.map { it.copy(start = 0, end = 0) }
        brokenAnchors.clear()
        brokenAnchors += comments.map { it.id }
        if (content == document.content) adopt(document)
    }

    override fun toString() = "CanvasDraft(changed=$changed,repair=${brokenAnchors.size})"
}
