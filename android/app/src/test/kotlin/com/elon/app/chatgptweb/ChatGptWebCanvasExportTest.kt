package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test

class ChatGptWebCanvasExportTest {
    private val doc = ChatGptWebCanvasDocument("synthetic", "Synthetic", "body", "document", 4, emptyList())
    private val path = "/c/11111111-1111-4111-8111-111111111111"
    private val ticket = "cd_${"a".repeat(32)}_1"
    private fun raw() = JSONObject().put("documentId", doc.id).put("documentVersion", 4).put("format", "pdf")
        .put("file", JSONObject().put("id", "canvas-export-synthetic-pdf").put("name", "Synthetic.pdf")
            .put("mediaType", "application/pdf").put("downloadHandle", "download_${"b".repeat(32)}"))

    @Test fun exportIsBoundToTheOriginalDocumentAndNativeDownloadHandle() {
        val export = ChatGptWebCanvasExportProtocol.parse(raw(), listOf(doc))
        val value = ChatGptWebCanvasDocuments("mcp_test", path, ticket, ticket, listOf(doc), false, exportFile = export)
        assertEquals(export.file, ChatGptWebCanvasExportProtocol.selected(value, path, export.file.id, export.file.downloadHandle))
        assertNull(ChatGptWebCanvasExportProtocol.selected(value, "/", export.file.id, export.file.downloadHandle))
        assertNull(ChatGptWebCanvasExportProtocol.selected(value, path, export.file.id, "download_${"c".repeat(32)}"))
        assertNull(ChatGptWebCanvasExportProtocol.selected(value.copy(unconfirmedWrite = true), path, export.file.id, export.file.downloadHandle))
        assertNull(ChatGptWebCanvasExportProtocol.selected(value.copy(documents = listOf(doc.copy(documentVersion = 5))),
            path, export.file.id, export.file.downloadHandle))
        assertFalse(export.toString().contains("Synthetic"))
    }

    @Test fun invalidMetadataCannotChooseAnArbitraryDownload() {
        for ((key, input) in listOf("documentId" to "foreign", "documentVersion" to 5, "documentVersion" to "4",
            "documentVersion" to 4.5, "format" to "html", "url" to "https://example.com")) {
            assertTrue(runCatching { ChatGptWebCanvasExportProtocol.parse(raw().put(key, input), listOf(doc)) }.isFailure)
        }
        for ((key, input) in listOf("id" to "file-foreign", "mediaType" to "text/html", "name" to "../secret.pdf",
            "name" to "wrong.docx", "downloadHandle" to "expired", "url" to "https://example.com")) {
            val row = raw(); row.getJSONObject("file").put(key, input)
            assertTrue(runCatching { ChatGptWebCanvasExportProtocol.parse(row, listOf(doc)) }.isFailure)
        }
        assertTrue(runCatching { ChatGptWebCanvasExportProtocol.parse(raw(), listOf(doc.copy(documentType = "code/python"))) }.isFailure)
    }

    @Test fun requestIsOnlyAnExplicitFormatAndExistingSelection() {
        fun request() = JSONObject().put("operation", "prepare_export").put("path", path).put("scope", ticket)
            .put("ticket", ticket).put("id", doc.id).put("format", "pdf")
        assertNotNull(ChatGptWebCanvasDocumentProtocol.request(request()))
        assertNotNull(ChatGptWebCanvasDocumentProtocol.request(request().put("format", "docx")))
        for ((key, input) in listOf("format" to "html", "format" to 1, "body" to "changed", "url" to "https://example.com"))
            assertNull(ChatGptWebCanvasDocumentProtocol.request(request().put(key, input)))
    }
}
