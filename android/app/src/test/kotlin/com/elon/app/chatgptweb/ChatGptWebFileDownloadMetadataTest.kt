package com.elon.app.chatgptweb

import org.json.JSONObject
import org.junit.Assert.*
import org.junit.Test
import java.io.File

class ChatGptWebFileDownloadMetadataTest {
    private fun lease() = requireNotNull(ChatGptWebFileDownloadLease().begin(
        "doc_fixture", 7, "https://chatgpt.com/c/fixture", "Cloud document",
        "application/vnd.google-apps.document", 1_000))

    private fun packet(name: Any = "Cloud document.docx", mediaType: Any = DOCX) = JSONObject().put(
        "resolvedFile", JSONObject().put("version", 1).put("name", name).put("mediaType", mediaType))

    @Test fun oldPacketsPreserveTheExactOriginalLease() {
        val original = lease()
        assertSame(original, ChatGptWebFileDownloadMetadata.resolve(original, JSONObject()))
    }

    @Test fun materializedMetadataChangesOnlyStorageNameAndType() {
        val original = lease()
        val resolved = requireNotNull(ChatGptWebFileDownloadMetadata.resolve(original, packet()))
        assertEquals("Cloud document.docx", resolved.name)
        assertEquals(DOCX, resolved.mediaType)
        assertEquals(original, resolved.copy(name = original.name, mediaType = original.mediaType))
        assertEquals("Cloud document", original.name)
    }

    @Test fun onlyTheConsumedOriginalDocumentRouteAndTimeLeaseCanReachTheResolver() {
        val leases = ChatGptWebFileDownloadLease()
        val original = requireNotNull(leases.begin("doc_a", 1, "https://chatgpt.com/c/a", "source", "", 0))
        for ((token, generation, href, time) in listOf(
            listOf("doc_b", 1L, original.href, 1L), listOf("doc_a", 2L, original.href, 1L),
            listOf("doc_a", 1L, "https://chatgpt.com/c/b", 1L), listOf("doc_a", 1L, original.href, 25_000L))) {
            assertNull(leases.consume(original.id, token as String, generation as Long, href as String, time as Long))
        }
        val consumed = requireNotNull(leases.consume(original.id, "doc_a", 1, original.href, 1))
        assertNotNull(ChatGptWebFileDownloadMetadata.resolve(consumed, packet()))
        assertNull(leases.consume(original.id, "doc_a", 1, original.href, 2))
    }

    @Test fun unknownVersionsPartialFieldsAndWrongTypesFailClosed() {
        for (bad in listOf<Any>(JSONObject.NULL, "text", 1, JSONObject(),
            JSONObject().put("version", 2).put("name", "a.docx").put("mediaType", DOCX),
            JSONObject().put("version", "1").put("name", "a.docx").put("mediaType", DOCX),
            JSONObject().put("version", 1.0).put("name", "a.docx").put("mediaType", DOCX),
            JSONObject().put("version", 1).put("name", "a.docx"),
            JSONObject().put("version", 1).put("name", "a.docx").put("mediaType", DOCX).put("url", "https://example.test"))) {
            assertNull(ChatGptWebFileDownloadMetadata.resolve(lease(), JSONObject().put("resolvedFile", bad)))
        }
        assertNull(ChatGptWebFileDownloadMetadata.resolve(lease(), packet(12)))
        assertNull(ChatGptWebFileDownloadMetadata.resolve(lease(), packet(mediaType = 12)))
    }

    @Test fun malformedNamesAndMimeTypesCannotStartStorage() {
        for (name in listOf("", " ", "bad\u0000.docx", "bad\n.docx", "bad\u007f.docx", "a".repeat(1025))) {
            assertNull(name, ChatGptWebFileDownloadMetadata.resolve(lease(), packet(name)))
        }
        for (type in listOf("text/plain; charset=utf-8", "text/\nplain", "image", "*/plain", " /text", "a".repeat(64) + "/b")) {
            assertNull(type, ChatGptWebFileDownloadMetadata.resolve(lease(), packet(mediaType = type)))
        }
        assertEquals("", ChatGptWebFileDownloadMetadata.resolve(lease(), packet(mediaType = ""))?.mediaType)
        assertEquals("text/plain", ChatGptWebFileDownloadMetadata.resolve(lease(), packet(mediaType = "TEXT/PLAIN"))?.mediaType)
    }

    @Test fun exportedNamesCannotEscapeStorageOrLoseTheirSuffix() {
        val escaped = requireNotNull(ChatGptWebFileDownloadMetadata.resolve(lease(), packet("../../draft\\copy:report.docx")))
        assertFalse(escaped.name.contains('/'))
        assertFalse(escaped.name.contains('\\'))
        assertFalse(escaped.name.contains(':'))
        val long = requireNotNull(ChatGptWebFileDownloadMetadata.resolve(lease(), packet("a".repeat(300) + ".xlsx")))
        assertEquals(150, long.name.length)
        assertTrue(long.name.endsWith(".xlsx"))
        assertEquals("download.bin", ChatGptWebFileDownloadMetadata.resolve(lease(), packet("..."))?.name)
    }

    @Test fun bothProductionDownloadRoutesResolveBeforeCreatingStorage() {
        val source = listOf(File("src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebFileDownloadGateway.kt"),
            File("android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebFileDownloadGateway.kt"))
            .first { it.isFile }.readText()
        assertTrue(source.contains("put(\"resolvedFileVersion\", ChatGptWebFileDownloadMetadata.VERSION)"))
        assertEquals(2, Regex("consumeResolved\\(id, value, state\\)").findAll(source).count())
        assertTrue(source.contains("ChatGptWebFileDownloadMetadata.resolve(lease, value)"))
        assertTrue(source.indexOf("bytes.accept(value, lease)") > source.indexOf("consumeResolved(id, value, state)"))
    }

    @Test fun unicodeExportTitlesFitTheNativeDestinationByteBudget() {
        for (stem in listOf("\u6587".repeat(160), "\ud83d\udcc4".repeat(160), "a".repeat(144) + "\ud83d\udcc4" + "b".repeat(20))) {
            val result = requireNotNull(ChatGptWebFileDownloadMetadata.resolve(lease(), packet(stem + ".docx")))
            val destination = "elon-${result.id}-${result.name}"
            assertTrue(destination.toByteArray(Charsets.UTF_8).size <= 255)
            assertTrue(result.name.endsWith(".docx"))
            assertEquals(result.name, result.name.toByteArray(Charsets.UTF_8).toString(Charsets.UTF_8))
        }
    }

    private companion object {
        const val DOCX = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    }
}
