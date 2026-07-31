package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class BottomNavigationInsetsContractTest {
    @Test
    fun bottomMenuUsesOpaqueBaseAndFadeOnBothSurfaces() {
        val dimens = readRepositoryFile("android/app/src/main/res/values/dimens.xml")
        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        val fade = readRepositoryFile("android/app/src/main/res/drawable/bg_bottom_nav_fade.xml")
        assertTrue(dimens.contains("name=\"main_bottom_menu_fade_height\">24dp</dimen>"))
        assertTrue(
            Regex(
                """android:id="@\+id/pageTabs"[^>]*android:background="@color/elon_bg_app"""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(layout)
        )
        assertTrue(
            Regex(
                """android:id="@\+id/bottomMenuFade"[^>]*android:layout_height="@dimen/main_bottom_menu_fade_height"[^>]*android:layout_marginTop="-24dp"[^>]*android:background="@drawable/bg_bottom_nav_fade"""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(layout)
        )
        assertTrue(fade.contains("android:startColor=\"@android:color/transparent\""))
        assertTrue(fade.contains("android:centerColor=\"#99000000\""))
        assertTrue(fade.contains("android:endColor=\"@color/elon_bg_app\""))

        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(
            Regex(
                """\.tabs-bar\s*\{[^}]*background:\s*var\(--bg\);""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(web)
        )
        assertTrue(
            Regex(
                """\.tabs-bar::before\s*\{[^}]*top:\s*-24px;[^}]*height:\s*24px;[^}]*background:\s*linear-gradient\([^}]*var\(--bg\)\s+100%""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(web)
        )
    }

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

    @Test
    fun scrollPagesReserveBottomBarExactlyOnceAcrossAndroidAndWeb() {
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainBottomNavigationController.kt"
        )
        assertTrue(controller.contains("R.dimen.main_bottom_menu_outer_height"))
        listOf("projectScrollView", "profilePage", "marketplacePage").forEach { id ->
            assertTrue(controller.contains("binding.$id"))
        }

        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        assertTrue(
            Regex(
                """android:id="@\+id/profilePageContent"[^>]*android:paddingBottom="16dp"""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(layout)
        )
        assertTrue(
            Regex(
                """android:id="@\+id/marketplaceListContainer"[^>]*android:paddingBottom="16dp"""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(layout)
        )

        val marketplace = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        assertTrue(!marketplace.contains("setPadding(0, 0, 0, dp(124))"))

        val web = readRepositoryFile("server/src/assets/web_page.html")
        val sharedBottomInset =
            """calc(var(--bottom-menu-height) + 16px + env(safe-area-inset-bottom))"""
        assertTrue(
            Regex(
                """#profilePage\s*\{[^}]*padding:\s*14px\s+20px\s+${Regex.escape(sharedBottomInset)};""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(web)
        )
        assertTrue(
            Regex(
                """\.project-plaza-inline-root\s*\{[^}]*padding-bottom:\s*${Regex.escape(sharedBottomInset)};""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(web)
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
