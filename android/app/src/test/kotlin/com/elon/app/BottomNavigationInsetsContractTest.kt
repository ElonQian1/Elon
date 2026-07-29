package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class BottomNavigationInsetsContractTest {
    @Test
    fun androidEdgeSelectionsKeepFiveDpInsets() {
        val dimens = readRepositoryFile("android/app/src/main/res/values/dimens.xml")
        assertTrue(dimens.contains("name=\"main_bottom_menu_content_width\">320dp</dimen>"))
        assertTrue(dimens.contains("name=\"main_bottom_menu_selection_inset\">5dp</dimen>"))
        assertTrue(dimens.contains("name=\"main_bottom_menu_selection_width\">58dp</dimen>"))
        assertTrue(dimens.contains("name=\"main_bottom_menu_selection_height\">46dp</dimen>"))

        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        val content = linearLayoutBlock(layout, "bottomNavContent")
        assertTrue(content.contains("android:layout_width=\"@dimen/main_bottom_menu_content_width\""))
        assertTrue(content.contains("android:layout_gravity=\"center_horizontal\""))
        assertTrue(!content.contains("android:layout_width=\"match_parent\""))
        listOf(
            "tabChatSelection",
            "tabProjectSelection",
            "tabProfileSelection"
        ).forEach { id ->
            val block = imageViewBlock(layout, id)
            assertTrue(block.contains("android:layout_width=\"@dimen/main_bottom_menu_selection_width\""))
            assertTrue(block.contains("android:layout_height=\"@dimen/main_bottom_menu_selection_height\""))
        }

        assertTrue(
            imageViewBlock(layout, "tabChatSelection")
                .contains("android:layout_marginStart=\"@dimen/main_bottom_menu_selection_inset\"")
        )
        assertTrue(!layout.contains("android:id=\"@+id/bottomMenuSelection\""))
    }

    @Test
    fun webMirrorKeepsTheSameFivePixelGeometry() {
        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(
            Regex("""\.tabs-bar\s*\{[^}]*max-width:\s*360px;""", RegexOption.DOT_MATCHES_ALL)
                .containsMatchIn(web)
        )
        assertTrue(
            Regex(
                """\.tabs-bar\s*\{[^}]*padding:\s*8px\s+max\(0px,\s*calc\(50%\s*-\s*160px\)\)""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(web)
        )
        assertTrue(
            Regex("""\.tab-selection\s*\{[^}]*width:\s*58px;[^}]*height:\s*46px;""", RegexOption.DOT_MATCHES_ALL)
                .containsMatchIn(web)
        )
        assertTrue(
            Regex("""\.tabs-panel\s*>\s*\.tab:first-child\s+\.tab-selection\s*\{[^}]*left:\s*5px;""", RegexOption.DOT_MATCHES_ALL)
                .containsMatchIn(web)
        )
        val menuButton = Regex(
            """<button class="bottom-menu-tab"[^>]*>.*?</button>""",
            RegexOption.DOT_MATCHES_ALL
        ).find(web)?.value ?: error("Missing bottom menu button")
        assertTrue(!web.contains(".bottom-menu-tab.active .tab-selection"))
        assertTrue(!menuButton.contains("tab-selection"))
    }

    private fun imageViewBlock(layout: String, id: String): String {
        val marker = "android:id=\"@+id/$id\""
        val start = layout.indexOf(marker)
        require(start >= 0) { "Missing $id" }
        val end = layout.indexOf("/>", start)
        require(end >= 0) { "Unclosed $id" }
        return layout.substring(start, end)
    }

    private fun linearLayoutBlock(layout: String, id: String): String {
        val marker = "android:id=\"@+id/$id\""
        val start = layout.lastIndexOf("<LinearLayout", layout.indexOf(marker))
        require(start >= 0) { "Missing $id" }
        val end = layout.indexOf(">", layout.indexOf(marker))
        require(end >= 0) { "Unclosed $id" }
        return layout.substring(start, end)
    }

    private fun readRepositoryFile(relativePath: String): String {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        val candidates = generateSequence(cwd) { it.parent }
            .map { it.resolve(relativePath) }
            .take(6)
            .toList()
        val path: Path = candidates.firstOrNull(Files::isRegularFile)
            ?: error("Unable to find $relativePath from $cwd")
        return String(Files.readAllBytes(path), StandardCharsets.UTF_8)
    }
}
