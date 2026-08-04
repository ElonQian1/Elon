package com.elon.app

import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.security.MessageDigest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ProjectPlazaLayoutContractTest {
    @Test
    fun apkFeaturedBitmapAssetsRemainTrackedAtTheirOriginalDimensions() {
        val directory = repositoryRoot().resolve("android/app/src/main/res/drawable-nodpi")
        val expected = mapOf(
            "project_plaza_ui1_card.png" to (837 to 943),
            "project_plaza_ui2_thumbnail.png" to (159 to 155),
            "project_plaza_ui3_avatar.png" to (97 to 98),
            "project_plaza_ui4_heart.png" to (59 to 55),
            "project_plaza_ui5_star.png" to (65 to 59)
        )

        expected.forEach { (name, dimensions) ->
            val path = directory.resolve(name)
            assertTrue("missing $name", Files.isRegularFile(path))
            assertEquals("$name dimensions", dimensions, readPngDimensions(path))
        }
    }

    @Test
    fun androidKeepsTheApkFeaturedCarouselAsThePrimaryDesign() {
        val source = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        listOf(
            "R.drawable.project_plaza_ui1_card",
            "R.drawable.project_plaza_ui2_thumbnail",
            "R.drawable.project_plaza_ui3_avatar",
            "R.drawable.project_plaza_ui4_heart",
            "R.drawable.project_plaza_ui5_star"
        ).forEach { resource -> assertTrue(source.contains(resource)) }
        assertTrue(source.contains("buildFeaturedStrip(projects.take(5))"))
        assertTrue(source.contains("FEATURED_CARD_WIDTH_FRACTION = 0.6564706f"))
        assertTrue(source.contains("FEATURED_CARD_HEIGHT_RATIO = 1.1266428f"))
        assertTrue(source.contains("FEATURED_CARD_GAP_DP = 9"))
    }

    @Test
    fun mobilePwaMirrorsTheApkFeaturedCarouselAndListRhythm() {
        val script = readRepositoryFile("server/src/assets/project_plaza.js")
        val styles = readRepositoryFile("server/src/assets/project_plaza.css")
        val page = readRepositoryFile("server/src/assets/web_page.html")

        assertTrue(script.contains("project-plaza-featured-scroller"))
        assertTrue(script.contains("state.projects.slice(0, 5).map(renderFeaturedCard)"))
        assertTrue(script.contains("project-plaza-featured-media"))
        assertTrue(styles.contains("width: 65.647vw;"))
        assertTrue(styles.contains("aspect-ratio: 837 / 943;"))
        assertTrue(styles.contains("gap: 9px;"))
        assertTrue(styles.contains("padding: 0 98px 0 20px;"))
        assertTrue(page.contains("data-ui-design=\"apk-featured-carousel-v1\""))
        assertFalse(script.contains("project-plaza-status-spine"))
    }

    @Test
    fun plazaEntryArrowsUseTheVerifiedBitmapOnAndroidAndMobilePwa() {
        val expectedSha256 = "3147797d74d7ee606612a79224216210865d6b783273fc3166eb332f3ed37275"
        val androidAsset = repositoryRoot().resolve(
            "android/app/src/main/res/drawable-nodpi/project_view_chevron.png"
        )
        val webAsset = repositoryRoot().resolve("server/src/assets/project_view_chevron.png")
        assertEquals(expectedSha256, sha256(androidAsset))
        assertEquals(expectedSha256, sha256(webAsset))

        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val webScript = readRepositoryFile("server/src/assets/project_plaza.js")
        assertEquals(2, Regex(Regex.escape("R.drawable.project_view_chevron")).findAll(androidSource).count())
        assertEquals(2, Regex(Regex.escape("/assets/project_view_chevron.png")).findAll(webScript).count())
        assertFalse(androidSource.contains("text = \"›\""))
        assertFalse(webScript.contains(">›<"))
    }

    @Test
    fun carouselAndListActionsKeepFortyEightPixelTouchTargets() {
        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val webStyles = readRepositoryFile("server/src/assets/project_plaza.css")
        val projectListRow = androidSource.substringAfter("private fun buildProjectListRow")
            .substringBefore("private fun projectThumbnail")

        assertTrue(projectListRow.contains("LinearLayout.LayoutParams(dp(48), dp(48))"))
        assertTrue(androidSource.contains("FrameLayout.LayoutParams(dp(27), dp(27)"))
        assertTrue(webStyles.contains(".project-plaza-featured-open"))
        assertTrue(webStyles.contains(".project-plaza-open"))
        assertTrue(Regex("width:\\s*48px;[\\s\\S]*?height:\\s*48px;").containsMatchIn(webStyles))
    }

    @Test
    fun searchAndFiltersStayVisibleAboveTheApkLedContent() {
        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val webScript = readRepositoryFile("server/src/assets/project_plaza.js")
        val mobileWeb = readRepositoryFile("server/src/assets/web_page.html")

        assertTrue(androidSource.contains("shell.addView(buildSearchBar())"))
        assertTrue(androidSource.contains("shell.addView(buildFilterScroller()"))
        assertTrue(androidSource.contains("R.drawable.ic_top_search_custom"))
        assertTrue(webScript.contains("placeholder=\"搜索项目、作者\""))
        assertTrue(mobileWeb.contains("data-search-artwork=\"/assets/project_view_search_icon.png\""))
    }

    @Test
    fun projectEntryKeepsTheAcceptedSinglePlazaStructure() {
        val navigation = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainNavigationController.kt"
        )
        val marketplaceChrome = navigation.substringAfter("private fun applyMarketplaceChrome()")
            .substringBefore("private fun applyProjectSpaceChrome")
        assertTrue(navigation.contains("binding.tabProject -> binding.marketplacePage"))
        assertTrue(
            Regex("if \\(tab == binding\\.tabProject\\) \\{\\s+loadMarketplace\\(\\)")
                .containsMatchIn(navigation)
        )
        assertTrue(marketplaceChrome.contains("hideProjectTopTabs()"))
        assertTrue(marketplaceChrome.contains("binding.addButton.visibility = View.GONE"))
        assertFalse(marketplaceChrome.contains("showProjectTopTabs"))
    }

    @Test
    fun mobilePwaAutoOpensTheVisibleLateLoadedPlazaModule() {
        val script = readRepositoryFile("server/src/assets/project_plaza.js")
        val mobileWeb = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(script.contains("!inlineRoot.classList.contains('hidden')"))
        assertTrue(mobileWeb.contains("projectPage.classList.add('project-plaza-mode')"))
        assertTrue(mobileWeb.contains("projectPage.classList.remove('project-plaza-mode')"))
    }

    private fun readRepositoryFile(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun readPngDimensions(path: Path): Pair<Int, Int> {
        val bytes = Files.readAllBytes(path)
        assertTrue("invalid PNG header: $path", bytes.size >= 24)
        assertEquals(0x89.toByte(), bytes[0])
        assertEquals('P'.code.toByte(), bytes[1])
        val header = ByteBuffer.wrap(bytes, 16, 8).order(ByteOrder.BIG_ENDIAN)
        return header.int to header.int
    }

    private fun sha256(path: Path): String = MessageDigest.getInstance("SHA-256")
        .digest(Files.readAllBytes(path))
        .joinToString("") { "%02x".format(it) }

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Unable to locate repository root from $cwd")
    }
}
