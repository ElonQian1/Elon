package com.elon.app.chatgptweb

import com.elon.app.WebChatConversationFile
import org.json.JSONObject

internal data class ChatGptWebCanvasExport(
    val documentId: String, val documentVersion: Long, val format: String, val file: WebChatConversationFile,
) {
    override fun toString() = "CanvasExport(format=$format,version=$documentVersion)"
}

internal object ChatGptWebCanvasExportProtocol {
    private val types = mapOf("pdf" to "application/pdf",
        "docx" to "application/vnd.openxmlformats-officedocument.wordprocessingml.document")

    fun parse(value: JSONObject, documents: List<ChatGptWebCanvasDocument>): ChatGptWebCanvasExport {
        require(value.keys().asSequence().toSet() == setOf("documentId", "documentVersion", "format", "file"))
        val document = documents.single { it.id == value.getString("documentId") }
        require(document.documentType == "document" && value.opt("documentVersion") is Number &&
            value.getDouble("documentVersion") == document.documentVersion.toDouble())
        val format = value.getString("format")
        val mime = types[format] ?: error("export_format")
        val row = value.getJSONObject("file")
        require(row.keys().asSequence().toSet() == setOf("id", "name", "mediaType", "downloadHandle"))
        val name = row.getString("name")
        val handle = row.getString("downloadHandle")
        val id = "canvas-export-${document.id}-$format"
        require(row.opt("id") == id && row.opt("mediaType") == mime && ChatGptWebFileDownloadPolicy.HANDLE.matches(handle))
        require(name.isNotBlank() && name.length <= 200 && name.endsWith(".$format") &&
            name.none { it.code < 32 || it.code == 127 || it in "\\/:*?\"<>|" })
        return ChatGptWebCanvasExport(document.id, document.documentVersion, format,
            WebChatConversationFile(id, "", name, "document", "assistant", mime, handle))
    }

    fun selected(value: ChatGptWebCanvasDocuments?, path: String, id: String, handle: String): WebChatConversationFile? {
        val export = value?.exportFile ?: return null
        return export.file.takeIf { !value.unconfirmedWrite && value.path == path && it.id == id && it.downloadHandle == handle &&
            value.documents.any { doc -> doc.id == export.documentId && doc.documentVersion == export.documentVersion } }
    }
}
