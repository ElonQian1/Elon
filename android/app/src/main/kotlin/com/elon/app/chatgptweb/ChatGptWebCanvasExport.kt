package com.elon.app.chatgptweb

import com.elon.app.WebChatConversationFile
import org.json.JSONObject

internal data class ChatGptWebCanvasExport(
    val documentId: String, val documentVersion: Long, val format: String, val file: WebChatConversationFile,
) {
    override fun toString() = "CanvasExport(format=$format,version=$documentVersion)"
}

internal object ChatGptWebCanvasExportProtocol {
    fun parse(value: JSONObject, documents: List<ChatGptWebCanvasDocument>): ChatGptWebCanvasExport {
        require(value.keys().asSequence().toSet() == setOf("documentId", "documentVersion", "format", "file"))
        val document = documents.single { it.id == value.getString("documentId") }
        require(value.opt("documentVersion") is Number &&
            value.getDouble("documentVersion") == document.documentVersion.toDouble())
        val format = value.getString("format")
        val type = ChatGptWebCanvasExportFormats.find(document.documentType, format) ?: error("export_format")
        val mime = type.mediaType
        val row = value.getJSONObject("file")
        require(row.keys().asSequence().toSet() == setOf("id", "name", "mediaType", "downloadHandle"))
        val name = row.getString("name")
        val handle = row.getString("downloadHandle")
        val id = "canvas-export-${document.id}-$format"
        require(row.opt("id") == id && row.opt("mediaType") == mime && ChatGptWebFileDownloadPolicy.HANDLE.matches(handle))
        require(name.isNotBlank() && name.length <= 150 && name.endsWith(".${type.extension}") &&
            name.none { it.code < 32 || it.code in 127..159 || it.code in 0x202a..0x202e ||
                it.code in 0x2066..0x2069 || it in "\\/:*?\"<>|" })
        return ChatGptWebCanvasExport(document.id, document.documentVersion, format,
            WebChatConversationFile(id, "", name, "document", "assistant", mime, handle))
    }

    fun selected(value: ChatGptWebCanvasDocuments?, path: String, id: String, handle: String): WebChatConversationFile? {
        val export = value?.exportFile ?: return null
        return export.file.takeIf { !value.unconfirmedWrite && value.path == path && it.id == id && it.downloadHandle == handle &&
            value.documents.any { doc -> doc.id == export.documentId && doc.documentVersion == export.documentVersion } }
    }
}
