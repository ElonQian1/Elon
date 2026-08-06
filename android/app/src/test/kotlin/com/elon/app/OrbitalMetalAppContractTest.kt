package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class OrbitalMetalAppContractTest {
    @Test
    fun majorApkScreensUseTheSharedOrbitalPanel() {
        val panel = source("android/app/src/main/res/drawable/bg_orbital_panel.xml")
        val action = source("android/app/src/main/res/drawable/bg_orbital_action.xml")
        assertTrue(panel.contains("@color/elon_metal_highlight"))
        assertTrue(panel.contains("@color/elon_surface_header"))
        assertTrue(action.contains("@color/elon_titanium"))
        assertTrue(action.contains("@color/elon_titanium_end"))

        listOf(
            "android/app/src/main/res/layout/activity_login.xml",
            "android/app/src/main/res/layout/activity_settings.xml",
            "android/app/src/main/res/layout/activity_token_usage.xml",
            "android/app/src/main/res/layout/activity_voice_engine.xml",
            "android/app/src/main/res/layout/page_agent.xml"
        ).forEach { path ->
            assertTrue("$path does not use the orbital panel", source(path).contains("@drawable/bg_orbital_panel"))
        }

        val main = source("android/app/src/main/res/layout/activity_main.xml")
        val profile = source("android/app/src/main/kotlin/com/elon/app/UserProfileViews.kt")
        val quota = source("android/app/src/main/kotlin/com/elon/app/ProfileTokenUsageCard.kt")
        assertTrue(main.count("@drawable/bg_orbital_panel") >= 2)
        assertTrue(profile.contains("R.drawable.bg_orbital_panel"))
        assertTrue(quota.contains("R.drawable.bg_orbital_panel"))
        assertFalse(main.contains("@drawable/profile_panel_primary_actions_stable"))
        assertFalse(main.contains("@drawable/profile_panel_support_actions"))
    }

    @Test
    fun sharedInteractiveSurfacesCarryMetalMaterialAndDarkErrors() {
        listOf(
            "bg_bubble_ai.xml",
            "bg_bubble_user.xml",
            "bg_home_floating_nav.xml",
            "bg_input_pill.xml",
            "bg_update_sheet.xml",
            "bg_update_primary.xml",
            "bg_send_button.xml"
        ).forEach { name ->
            val drawable = source("android/app/src/main/res/drawable/$name")
            assertTrue("$name lost material depth", drawable.contains("<gradient"))
        }

        val error = source("android/app/src/main/res/drawable/bg_error_message.xml")
        assertTrue(error.contains("@color/elon_badge_danger_bg"))
        assertFalse(error.contains("#FFF1F0"))
        assertFalse(error.contains("#FFD6D2"))
    }

    @Test
    fun pwaLoadsTheApkLedThemeAsTheFinalStyleLayer() {
        val page = source("server/src/assets/web_page.html")
        val theme = source("server/src/assets/orbital_mobile_theme.css")
        val router = source("server/src/router.rs")
        val web = source("server/src/web.rs")

        val projectHomeIndex = page.indexOf("/assets/project_home.css")
        val orbitalIndex = page.indexOf("/assets/orbital_mobile_theme.css")
        assertTrue(projectHomeIndex >= 0 && orbitalIndex > projectHomeIndex)
        assertTrue(page.contains("data-ui-system=\"apk-orbital-metal-workbench-v1\""))
        assertTrue(router.contains("/assets/orbital_mobile_theme.css"))
        assertTrue(web.contains("assets/orbital_mobile_theme.css"))
        assertTrue(theme.contains(".tabs-panel"))
        assertTrue(theme.contains(".input-panel"))
        assertTrue(theme.contains("#profilePage .profile-action-group"))
        assertTrue(theme.contains(".bubble.user"))
        assertTrue(theme.contains("@media (prefers-reduced-motion: reduce)"))
        assertFalse(theme.contains("text-shadow"))
        assertFalse(theme.contains("filter: drop-shadow"))
    }

    @Test
    fun androidSourceDoesNotReintroduceTheLegacyGrayBluePalette() {
        val legacyColors = listOf(
            "#0B1017", "#070B10", "#0A0F16", "#151515", "#222222", "#2E2E2E",
            "#3B3B3E", "#172231", "#1D2A39", "#7AA7FF", "#A8C5FF", "#73C7E8",
            "#5AC8A0", "#E7B86A", "#F07884"
        )
        val androidSource = repositoryRoot().resolve("android/app/src/main")
        Files.walk(androidSource).use { paths ->
            paths.filter { path ->
                Files.isRegularFile(path) &&
                    (path.toString().endsWith(".kt") ||
                        path.toString().endsWith(".java") ||
                        path.toString().endsWith(".xml"))
            }.forEach { path ->
                val content = String(Files.readAllBytes(path), StandardCharsets.UTF_8).uppercase()
                legacyColors.forEach { color ->
                    assertFalse("legacy color $color remains in $path", content.contains(color))
                }
            }
        }
    }

    private fun source(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Unable to locate repository root from $cwd")
    }
}
