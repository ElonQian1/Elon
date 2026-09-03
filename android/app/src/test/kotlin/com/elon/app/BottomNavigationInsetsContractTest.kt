package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class BottomNavigationInsetsContractTest {
    @Test
    fun bottomMenuHasNoFullWidthBackgroundBoardOnEitherSurface() {
        val dimens = readRepositoryFile("android/app/src/main/res/values/dimens.xml")
        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        assertTrue(dimens.contains("name=\"main_bottom_menu_fade_height\">4dp</dimen>"))
        assertTrue(dimens.contains("name=\"main_bottom_menu_container_height\">88dp</dimen>"))
        assertTrue(
            Regex(
                """android:id="@\+id/pageTabs"[^>]*android:layout_height="@dimen/main_bottom_menu_container_height"[^>]*android:background="@android:color/transparent"""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(layout)
        )
        assertTrue(!layout.contains("android:id=\"@+id/bottomMenuBase\""))
        assertTrue(!layout.contains("android:id=\"@+id/bottomMenuFade\""))

        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(web.contains("background: rgba(32,31,31,.8);"))
        assertTrue(!web.contains(".tabs-bar::before"))
    }

    @Test
    fun androidBottomMenuMatchesStitchGeometryWhileKeepingExistingIcons() {
        val dimens = readRepositoryFile("android/app/src/main/res/values/dimens.xml")
        assertTrue(dimens.contains("name=\"main_bottom_menu_content_width\">326dp</dimen>"))
        assertTrue(dimens.contains("name=\"main_bottom_menu_edge_gap\">24dp</dimen>"))
        assertTrue(dimens.contains("name=\"main_bottom_menu_selection_width\">56dp</dimen>"))
        assertTrue(dimens.contains("name=\"main_bottom_menu_selection_height\">48dp</dimen>"))

        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        val content = linearLayoutBlock(layout, "bottomNavContent")
        assertTrue(content.contains("android:layout_width=\"@dimen/main_bottom_menu_content_width\""))
        assertTrue(content.contains("android:layout_gravity=\"bottom|center_horizontal\""))
        assertTrue(content.contains("android:layout_height=\"@dimen/main_bottom_menu_outer_height\""))
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
        assertTrue(layout.contains("@drawable/ic_bottom_nav_chat"))
        assertTrue(layout.contains("@drawable/ic_bottom_nav_project_selector"))
        assertTrue(layout.contains("@drawable/ic_bottom_nav_profile_selector"))
        assertTrue(layout.contains("@drawable/ic_bottom_nav_menu_selector"))
    }

    @Test
    fun webMirrorUsesTheSameStitchGeometry() {
        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(
            Regex("""\.tabs-bar\s*\{[^}]*width:\s*326px;""", RegexOption.DOT_MATCHES_ALL)
                .containsMatchIn(web)
        )
        assertTrue(
            Regex(
                """\.tabs-bar\s*\{[^}]*padding:\s*8px\s+16px;""",
                RegexOption.DOT_MATCHES_ALL
            ).containsMatchIn(web)
        )
        assertTrue(
            Regex("""\.tab-selection\s*\{[^}]*width:\s*56px;[^}]*height:\s*48px;""", RegexOption.DOT_MATCHES_ALL)
                .containsMatchIn(web)
        )
        assertTrue(
            Regex("""\.tabs-panel\s*>\s*\.tab:first-child\s+\.tab-selection\s*\{[^}]*left:\s*0;""", RegexOption.DOT_MATCHES_ALL)
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
    fun unifiedStitchContainerWrapsPreservedNavigationIcons() {
        val settings = readRepositoryFile("android/settings.gradle")
        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainBottomNavigationController.kt"
        )
        val glass = readRepositoryFile("android/app/src/main/res/drawable/bg_bottom_nav_glass.xml")
        val circleGlass = readRepositoryFile(
            "android/app/src/main/res/drawable/bg_bottom_nav_glass_circle.xml"
        )
        val colors = readRepositoryFile("android/app/src/main/res/values/colors.xml")
        assertTrue(settings.contains("url = uri('https://jitpack.io')"))
        assertTrue(settings.contains("includeGroup('com.github.Dimezis')"))
        assertTrue(layout.contains("<eightbitlab.com.blurview.BlurTarget"))
        assertTrue(layout.contains("android:id=\"@+id/bottomNavGlass\""))
        assertTrue(layout.contains("android:id=\"@+id/bottomComposeGlass\""))
        assertTrue(
            Regex("""app:blurOverlayColor="@color/elon_nav_glass_overlay"""")
                .findAll(layout).count() == 2
        )
        assertTrue(controller.contains("listOf(binding.bottomNavGlass, binding.bottomComposeGlass)"))
        assertTrue(controller.contains(".setBlurRadius(18f)"))
        assertTrue(colors.contains("<color name=\"elon_nav_glass_overlay\">#73353A42</color>"))
        assertTrue(colors.contains("<color name=\"elon_nav_glass_border\">#1FFFFFFF</color>"))
        assertTrue(glass.contains("android:radius=\"28dp\""))
        assertTrue(glass.contains("android:color=\"@android:color/transparent\""))
        assertTrue(glass.contains("android:color=\"@color/elon_nav_glass_border\""))
        assertTrue(!glass.contains("<gradient"))
        assertTrue(circleGlass.contains("android:shape=\"oval\""))
        assertTrue(circleGlass.contains("android:color=\"@android:color/transparent\""))
        assertTrue(circleGlass.contains("android:color=\"@color/elon_nav_glass_border\""))
        assertTrue(!circleGlass.contains("<gradient"))

        val web = readRepositoryFile("server/src/assets/web_page.html")
        val panel = Regex("""\.tabs-panel\s*\{[^}]*}""", RegexOption.DOT_MATCHES_ALL)
            .find(web)?.value ?: error("Missing tabs panel styles")
        assertTrue(panel.contains("background: transparent;"))
        assertTrue(!panel.contains("linear-gradient"))
        assertTrue(panel.contains("border-radius: 0;"))
        val compose = Regex("""\.bottom-compose-button\s*\{[^}]*}""", RegexOption.DOT_MATCHES_ALL)
            .find(web)?.value ?: error("Missing compose button styles")
        assertTrue(compose.contains("background: #353534;"))
        assertTrue(!compose.contains("linear-gradient"))
        assertTrue(compose.contains("border-radius: 50%;"))

        val theme = readRepositoryFile("server/src/assets/orbital_mobile_theme.css")
        assertTrue(Regex("""\.tabs-bar\s*\{[^}]*background:\s*rgba\(32, 31, 31, 0\.8\);""").containsMatchIn(theme))
        assertTrue(!Regex("""\.tabs-panel\s*\{[^}]*linear-gradient""", RegexOption.DOT_MATCHES_ALL).containsMatchIn(theme))
        assertTrue(!Regex("""\.bottom-compose-button\s*\{[^}]*linear-gradient""", RegexOption.DOT_MATCHES_ALL).containsMatchIn(theme))
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
