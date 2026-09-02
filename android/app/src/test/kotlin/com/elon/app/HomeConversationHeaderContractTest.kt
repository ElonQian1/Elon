package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class HomeConversationHeaderContractTest {
    @Test
    fun homeHeaderOmitsUnreadFilterAndMarkAllReadControlOnBothSurfaces() {
        val androidHeader = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/HomeConversationHeaderView.kt"
        )
        assertFalse(androidHeader.contains("HomeListFilterMode.Unread to"))
        assertFalse(androidHeader.contains("全部已读"))
        assertTrue(androidHeader.contains("HomeListFilterMode.Friends to"))
        assertTrue(androidHeader.contains("HomeListFilterMode.Projects to"))
        assertTrue(androidHeader.contains("HomeListFilterMode.Conversations to"))
        assertTrue(androidHeader.contains("createFilterTab"))
        assertTrue(androidHeader.contains("if (selected) \"#303536\" else \"#00000000\""))
        assertTrue(androidHeader.contains("LinearLayout.LayoutParams.WRAP_CONTENT, dp(18)"))
        assertTrue(androidHeader.contains("minWidth = dp(20)"))
        assertTrue(androidHeader.contains("roundedWithStroke"))
        assertTrue(androidHeader.contains("\"#C7FAFF\""))

        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertFalse(web.contains("全部已读"))
        assertTrue(web.contains("free of standalone unread-filter and mark-all-read controls"))
        assertTrue(web.contains("className = 'home-filter-tabs'"))
        assertTrue(web.contains("['all', '全部', counts.all]"))
        assertTrue(web.contains("['conversations', '对话', counts.conversations]"))
        assertTrue(web.contains(".home-filter-tab.active { border-color: rgba(125,244,255,.4);"))
        assertTrue(web.contains(".home-filter-count { min-width: 20px; height: 18px;"))
        assertTrue(web.contains("background: #111617"))
    }

    private fun readRepositoryFile(relativePath: String): String {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        val path: Path = generateSequence(cwd) { it.parent }
            .map { it.resolve(relativePath) }
            .take(6)
            .firstOrNull(Files::isRegularFile)
            ?: error("Unable to find $relativePath from $cwd")
        return String(Files.readAllBytes(path), StandardCharsets.UTF_8)
    }
}
