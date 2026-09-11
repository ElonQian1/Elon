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

    @Test fun failuresKeepCachedRowsAndOnlyRenderKnownSafeMessages() {
        val expected = mapOf(
            "files_read_timeout" to "读取超时，可重试",
            "files_read_network" to "网络连接中断，可重试",
            "files_read_rate_limit" to "请求过于频繁，请稍后重试",
            "files_read_cooldown" to "请稍后重试",
            "files_identity_unavailable" to "登录状态需要确认",
            "files_read_parse" to "附件数据暂时无法解析",
            "files_read_http" to "官网读取失败，请稍后重试",
            "untrusted-server-detail" to "读取失败，可重试",
        )
        for ((detail, title) in expected) {
            val rows = WebChatConversationFilesPresentation.rows(index, false, true, detail)
            assertEquals(title, rows[0].title)
            assertEquals("web-chat-conversation-files-status", rows[0].contentDescription)
            assertEquals("sample.pdf", rows[1].title)
            assertTrue(rows[1].enabled)
            assertFalse(rows[0].enabled)
        }
    }

    @Test fun refreshKeepsItsSheetAndPendingReadWhileOtherFooterActionsStillDismiss() {
        assertTrue(WebChatActionSheetFooterAction("Open", "open") {}.dismissOnClick)
        assertFalse(WebChatActionSheetFooterAction("Refresh", "refresh", dismissOnClick = false) {}.dismissOnClick)
        val root = generateSequence(java.nio.file.Paths.get("").toAbsolutePath()) { it.parent }
            .first { java.nio.file.Files.isDirectory(it.resolve("android/app/src/main/kotlin/com/elon/app")) }
            .resolve("android/app/src/main/kotlin/com/elon/app")
        val coordinator = String(java.nio.file.Files.readAllBytes(root.resolve("WebChatConversationFilesCoordinator.kt")), Charsets.UTF_8)
        val sheet = String(java.nio.file.Files.readAllBytes(root.resolve("WebChatActionSheet.kt")), Charsets.UTF_8)
        assertTrue(sheet.contains("if (action.dismissOnClick) dialog.dismiss()"))
        assertTrue(coordinator.contains("\"web-chat-conversation-files-refresh\", dismissOnClick = false"))
        val refresh = coordinator.substringAfter("fun refresh() {").substringBefore("sheet = WebChatActionSheet")
        assertTrue(refresh.contains("pollTask != null) return"))
        assertTrue(refresh.contains("currentEpoch != epoch || consumerPort() !== owner"))
        assertFalse(refresh.contains("show(conversation"))
        assertFalse(refresh.contains("cancel()"))
        assertTrue(refresh.contains("sheet?.updateItems"))
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
