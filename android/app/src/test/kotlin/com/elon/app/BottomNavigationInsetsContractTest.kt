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
        assertTrue(dimens.contains("name=\"main_bottom_menu_selection_inset\">5dp</dimen>"))
        assertTrue(dimens.contains("name=\"main_bottom_menu_selection_width\">58dp</dimen>"))
        assertTrue(dimens.contains("name=\"main_bottom_menu_selection_height\">46dp</dimen>"))

        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        listOf(
            "tabChatSelection",
            "tabProjectSelection",
            "tabProfileSelection",
            "bottomMenuSelection"
        ).forEach { id ->
            val block = imageViewBlock(layout, id)
            assertTrue(block.contains("android:layout_width=\"@dimen/main_bottom_menu_selection_width\""))
            assertTrue(block.contains("android:layout_height=\"@dimen/main_bottom_menu_selection_height\""))
        }

        assertTrue(
            imageViewBlock(layout, "tabChatSelection")
                .contains("android:layout_marginStart=\"@dimen/main_bottom_menu_selection_inset\"")
        )
        assertTrue(
            imageViewBlock(layout, "bottomMenuSelection")
                .contains("android:layout_marginEnd=\"@dimen/main_bottom_menu_selection_inset\"")
        )
    }

    @Test
    fun webMirrorKeepsTheSameFivePixelGeometry() {
        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(
            Regex("""\.tab-selection\s*\{[^}]*width:\s*58px;[^}]*height:\s*46px;""", RegexOption.DOT_MATCHES_ALL)
                .containsMatchIn(web)
        )
        assertTrue(
            Regex("""\.tabs-panel\s*>\s*\.tab:first-child\s+\.tab-selection\s*\{[^}]*left:\s*5px;""", RegexOption.DOT_MATCHES_ALL)
                .containsMatchIn(web)
        )
        assertTrue(
            Regex("""\.tabs-panel\s*>\s*\.bottom-menu-tab:last-child\s+\.tab-selection\s*\{[^}]*right:\s*5px;""", RegexOption.DOT_MATCHES_ALL)
                .containsMatchIn(web)
        )
    }

    private fun imageViewBlock(layout: String, id: String): String {
        val marker = "android:id=\"@+id/$id\""
        val start = layout.indexOf(marker)
        require(start >= 0) { "Missing $id" }
        val end = layout.indexOf("/>", start)
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
