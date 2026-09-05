package com.elon.app

import org.junit.Assert.*
import org.junit.Test

class WebChatConversationFilesPresentationTest {
    private val index = WebChatConversationFileIndex("/c/test", "mcp_test", listOf(
        WebChatConversationFile("m:0", "m", "sample.pdf", "file", "user", "application/pdf")), false, 1_000)

    @Test fun cachedFilesStayVisibleDuringRefreshAndFailure() {
        for ((loading, failed) in listOf(true to false, false to true)) {
            val rows = WebChatConversationFilesPresentation.rows(index, loading, failed)
            assertEquals(2, rows.size)
            assertFalse(rows[0].enabled)
            assertEquals("sample.pdf", rows[1].title)
            assertEquals("web-chat-conversation-file-0", rows[1].contentDescription)
            assertTrue(rows[1].enabled)
        }
    }

    @Test fun loadingUnknownPartialAndConfirmedEmptyRemainDistinct() {
        val unknown = WebChatConversationFilesPresentation.rows(null, false, false).single()
        val loading = WebChatConversationFilesPresentation.rows(null, true, false).single()
        val empty = WebChatConversationFilesPresentation.rows(index.copy(files = emptyList()), false, false).single()
        val partial = WebChatConversationFilesPresentation.rows(index.copy(files = emptyList(), truncated = true), false, false).single()
        assertEquals(4, setOf(unknown.title, loading.title, empty.title, partial.title).size)
        assertEquals("此会话暂无附件", empty.title)
        assertEquals("部分附件", partial.title)
    }

    @Test fun productionLifecycleClosesConversationSheetsWithoutInstantiatingInactiveFeatures() {
        val sourcePath = generateSequence(java.nio.file.Paths.get("").toAbsolutePath()) { it.parent }
            .map { it.resolve("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt") }
            .first { java.nio.file.Files.isRegularFile(it) }
        val source = String(java.nio.file.Files.readAllBytes(sourcePath), Charsets.UTF_8)
        assertTrue(source.contains("private val productionConversationActions by productionConversationActionsDelegate"))
        assertTrue(source.contains("if (productionConversationActionsDelegate.isInitialized()) productionConversationActions.cancelPending()"))
        for (method in listOf("fun destroy()", "private fun deactivateChatProvider(", "private fun activateChatProvider(")) {
            val body = source.substringAfter(method).substringBefore("\n    private fun ")
            assertTrue(method, body.contains("cancelProductionActionSheets()"))
        }
    }
}
