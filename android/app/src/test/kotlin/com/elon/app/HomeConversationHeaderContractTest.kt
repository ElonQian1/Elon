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
        assertTrue(androidHeader.contains("if (selected) \"#2A2A2A\" else \"#00000000\""))
        assertTrue(androidHeader.contains("LinearLayout.LayoutParams.WRAP_CONTENT, dp(18)"))
        assertTrue(androidHeader.contains("minWidth = dp(20)"))
        assertTrue(androidHeader.contains("roundedWithStroke"))
        assertTrue(androidHeader.contains("\"#DBFCFF\""))
        assertTrue(androidHeader.contains("HorizontalScrollView(activity)"))
        assertTrue(androidHeader.contains("dp(192)"))
        assertTrue(androidHeader.contains("dp(95)"))
        assertTrue(androidHeader.contains("SummaryHeaderLayout"))
        assertTrue(androidHeader.contains("setShadowLayer(30f * density"))
        assertTrue(androidHeader.contains("Color.argb(102, 26, 26, 26)"))
        assertTrue(androidHeader.contains("Color.argb(38, 0, 240, 255)"))
        assertTrue(androidHeader.contains("RadialGradient"))
        assertTrue(androidHeader.contains("textSize = 18f"))

        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertFalse(web.contains("全部已读"))
        assertTrue(web.contains("free of standalone unread-filter and mark-all-read controls"))
        assertTrue(web.contains("className = 'home-filter-tabs'"))
        assertTrue(web.contains("['all', '全部', counts.all]"))
        assertTrue(web.contains("['conversations', '对话', counts.conversations]"))
        assertTrue(web.contains(".home-filter-tab.active { border-color: rgba(219,252,255,.3);"))
        assertTrue(web.contains(".home-filter-count { min-width: 20px; height: 18px;"))
        assertTrue(web.contains("min-height:192px"))
        assertTrue(web.contains(".home-filter-tabs { height: 95px; padding: 24px 16px 33px;"))
        assertTrue(web.contains("background: rgba(26,26,26,.4)"))
        assertTrue(web.contains("box-shadow:0 0 30px rgba(0,240,255,.15)"))
        assertTrue(web.contains("border-top: 1px solid rgba(255,255,255,.1)"))
        assertTrue(web.contains(".work-summary-entry::after"))
        assertTrue(web.contains("work-summary-entry-beta"))
        assertTrue(web.contains("appView.classList.toggle('stitch-home-active', isChatHome)"))
        assertTrue(web.contains("const showMenu = options.showMenu === true"))
        assertTrue(web.contains("searchBtn.style.display = isChatHome ? '' : 'none'"))
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
