package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class AppUiPaletteContractTest {
    @Test
    fun androidAndMobilePwaShareTheQuietNightPalette() {
        val colors = readRepositoryFile("android/app/src/main/res/values/colors.xml")
        val web = readRepositoryFile("server/src/assets/web_page.html")

        listOf(
            "<color name=\"elon_bg_app\">#0B1017</color>",
            "<color name=\"elon_surface_card\">#111923</color>",
            "<color name=\"elon_surface_header\">#172231</color>",
            "<color name=\"elon_button_primary_bg\">#7AA7FF</color>",
            "<color name=\"elon_status_info\">#73C7E8</color>",
            "<color name=\"elon_status_success\">#5AC8A0</color>",
            "<color name=\"elon_status_project\">#E7B86A</color>",
            "<color name=\"elon_status_danger\">#F07884</color>"
        ).forEach { token -> assertTrue("missing Android token $token", colors.contains(token)) }

        listOf(
            "--bg: #0B1017;",
            "--panel: #111923;",
            "--panel-2: #172231;",
            "--brand: #7AA7FF;",
            "--accent: #73C7E8;",
            "--success: #5AC8A0;",
            "--warning: #E7B86A;",
            "--danger: #F07884;"
        ).forEach { token -> assertTrue("missing PWA token $token", web.contains(token)) }

        assertFalse(colors.contains("<color name=\"elon_bg_app\">#000000</color>"))
        assertFalse(web.contains("--brand: #FFFFFF;"))
    }

    @Test
    fun apkProjectSurfacesReadCentralColorResources() {
        val projectHome = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProjectManagementHomeView.kt"
        )
        val marketplace = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val featured = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProjectPlazaFeaturedSection.kt"
        )

        assertTrue(projectHome.contains("R.color.elon_bg_app"))
        assertTrue(projectHome.contains("R.color.elon_segment_selected"))
        assertTrue(marketplace.contains("R.color.elon_plaza_surface_search"))
        assertTrue(marketplace.contains("R.color.elon_plaza_signal"))
        assertTrue(featured.contains("R.color.elon_plaza_action"))
        assertTrue(featured.contains("R.color.elon_plaza_status_success"))
        assertFalse(projectHome.contains("const val COLOR_BG ="))
        assertFalse(featured.contains("const val COLOR_CARD ="))
    }

    @Test
    fun projectPlazaSharesTheDeepSpaceObservatoryPalette() {
        val colors = readRepositoryFile("android/app/src/main/res/values/colors.xml")
        val styles = readRepositoryFile("server/src/assets/project_plaza.css")

        listOf(
            "<color name=\"elon_bg_plaza\">#0C0E12</color>",
            "<color name=\"elon_plaza_surface_card\">#111318</color>",
            "<color name=\"elon_plaza_surface_card_high\">#1F232B</color>",
            "<color name=\"elon_plaza_surface_header\">#171A20</color>",
            "<color name=\"elon_plaza_signal\">#8EA7D5</color>",
            "<color name=\"elon_plaza_action_end\">#8BB8C3</color>"
        ).forEach { token -> assertTrue("missing Android plaza token $token", colors.contains(token)) }
        listOf(
            "--plaza-bg: #0c0e12;",
            "--plaza-card: #111318;",
            "--plaza-card-high: #1f232b;",
            "--plaza-header: #171a20;",
            "--plaza-primary: #8ea7d5;",
            "--plaza-accent: #8bb8c3;"
        ).forEach { token -> assertTrue("missing PWA plaza token $token", styles.contains(token)) }
    }

    private fun readRepositoryFile(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Unable to locate repository root from $cwd")
    }
}
