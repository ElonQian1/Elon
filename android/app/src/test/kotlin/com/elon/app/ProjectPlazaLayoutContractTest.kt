package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.ByteBuffer
import java.nio.ByteOrder
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
    fun providedBitmapAssetsAreTrackedAtTheirOriginalDimensions() {
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
            val actual = readPngDimensions(path)
            assertEquals("$name width", dimensions.first, actual.first)
            assertEquals("$name height", dimensions.second, actual.second)
        }
    }

    @Test
    fun androidPlazaDirectlyReferencesAllFiveAssetsAndTargetRatios() {
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
        assertTrue(source.contains("FEATURED_CARD_WIDTH_FRACTION = 0.6564706f"))
        assertTrue(source.contains("FEATURED_CARD_HEIGHT_RATIO = 1.1266428f"))
        assertTrue(source.contains("FEATURED_CARD_GAP_DP = 9"))
        assertTrue(source.contains("LIST_ROW_GAP_PX = 64"))
        assertTrue(source.contains("LIST_TEXT_GAP_PX = 48"))
    }

    @Test
    fun plazaEntryArrowsUseTheVerifiedOriginalBitmapOnAndroidAndWeb() {
        val expectedSha256 = "3147797d74d7ee606612a79224216210865d6b783273fc3166eb332f3ed37275"
        val androidAsset = repositoryRoot().resolve(
            "android/app/src/main/res/drawable-nodpi/project_view_chevron.png"
        )
        val webAsset = repositoryRoot().resolve("server/src/assets/project_view_chevron.png")
        assertEquals(expectedSha256, sha256(androidAsset))
        assertEquals(expectedSha256, sha256(webAsset))
        assertEquals(43 to 43, readPngDimensions(androidAsset))

        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        assertEquals(2, Regex(Regex.escape("R.drawable.project_view_chevron")).findAll(androidSource).count())
        assertFalse(androidSource.contains("text = \"›\""))

        val webSource = readRepositoryFile("pc-frontend/src/features/plaza/ProjectPlazaView.tsx")
        assertTrue(webSource.contains("server/src/assets/project_view_chevron.png'"))
        assertEquals(2, Regex(Regex.escape("src={plazaChevronAsset}")).findAll(webSource).count())
        assertFalse(webSource.contains("ChevronRight"))
    }

    @Test
    fun plazaListArrowKeepsAFullTouchTargetWhileRenderingTheOriginalBitmapSize() {
        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val projectListRow = androidSource.substringAfter("private fun buildProjectListRow")
            .substringBefore("private fun projectThumbnail")
        assertTrue(projectListRow.contains("FrameLayout.LayoutParams(designPx(LIST_CHEVRON_PX)"))
        assertTrue(projectListRow.contains("marginEnd = designPx(LIST_CHEVRON_END_INSET_PX)"))
        assertTrue(projectListRow.contains("LinearLayout.LayoutParams(dp(48), dp(48))"))
        assertTrue(androidSource.contains("LIST_CHEVRON_PX = 43"))
        assertTrue(androidSource.contains("LIST_CHEVRON_END_INSET_PX = 12"))
    }

    @Test
    fun plazaFeaturedArrowMatchesTheTargetRoundButtonSize() {
        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val featuredCard = androidSource.substringAfter("private fun buildFeaturedCard")
            .substringBefore("private fun mediaPlaceholder")
        val featuredArrow = requireNotNull(
            Regex(
                """setImageResource\(R\.drawable\.project_view_chevron\)([\s\S]*?)""" +
                    """\}, FrameLayout\.LayoutParams\(dp\((\d+)\), dp\((\d+)\),"""
            ).find(featuredCard)
        )
        val featuredPadding = requireNotNull(
            Regex(
                """setPadding\(dp\((\d+)\), dp\((\d+)\), dp\((\d+)\), dp\((\d+)\)\)"""
            ).find(featuredArrow.groupValues[1])
        ).groupValues.drop(1).map(String::toInt)

        assertEquals(27, featuredArrow.groupValues[2].toInt())
        assertEquals(27, featuredArrow.groupValues[3].toInt())
        featuredPadding.forEach { assertEquals(6, it) }
        assertEquals(15, featuredArrow.groupValues[2].toInt() - featuredPadding[0] - featuredPadding[2])
    }

    @Test
    fun webPlazaListChevronIsSixteenPixelsInsideTheFortyEightPixelAction() {
        val styles = readRepositoryFile("pc-frontend/src/features/plaza/PlazaPage.module.css")
        assertTrue(
            Regex("""\.rowAction\s*\{[\s\S]*?width:\s*48px;[\s\S]*?height:\s*48px;[\s\S]*?\}""")
                .containsMatchIn(styles)
        )
        assertTrue(
            Regex("""\.rowAction > img\s*\{[\s\S]*?width:\s*16px;[\s\S]*?height:\s*16px;[\s\S]*?\}""")
                .containsMatchIn(styles)
        )
        assertTrue(
            Regex("""\.rowAction > svg\s*\{[\s\S]*?width:\s*24px;[\s\S]*?height:\s*24px;[\s\S]*?\}""")
                .containsMatchIn(styles)
        )
    }

    @Test
    fun webPlazaFeaturedChevronKeepsAFortyEightPixelTouchTargetAndTargetCircle() {
        val styles = readRepositoryFile("pc-frontend/src/features/plaza/PlazaPage.module.css")
        assertTrue(
            Regex("""\.primaryAction\s*\{[\s\S]*?width:\s*48px;[\s\S]*?height:\s*48px;[\s\S]*?\}""")
                .containsMatchIn(styles)
        )
        assertTrue(
            Regex("""\.primaryAction::before\s*\{[\s\S]*?width:\s*27px;[\s\S]*?height:\s*27px;[\s\S]*?\}""")
                .containsMatchIn(styles)
        )
        assertTrue(
            Regex("""\.primaryAction > img\s*\{[\s\S]*?width:\s*16px;[\s\S]*?height:\s*16px;[\s\S]*?\}""")
                .containsMatchIn(styles)
        )
        assertTrue(
            Regex("""\.primaryAction > svg\s*\{[\s\S]*?width:\s*22px;[\s\S]*?height:\s*22px;[\s\S]*?\}""")
                .containsMatchIn(styles)
        )
    }

    @Test
    fun projectEntryHasNoMineTabOrProjectOnlyAddButton() {
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
    fun webProjectEntryUsesTheSameSinglePlazaStructureAndAssets() {
        val entry = readRepositoryFile("pc-frontend/src/features/projects/ProjectsPage.tsx")
        val view = readRepositoryFile("pc-frontend/src/features/plaza/ProjectPlazaView.tsx")
        val styles = readRepositoryFile("pc-frontend/src/features/plaza/PlazaPage.module.css")
        assertFalse(entry.contains("我的项目"))
        assertTrue(entry.contains("<ProjectPlazaView />"))
        listOf("card.png", "thumbnail.png", "avatar.png", "heart.png", "star.png")
            .forEach { asset -> assertTrue(view.contains(asset)) }
        assertTrue(styles.contains("65.6471vw"))
        assertTrue(styles.contains("aspect-ratio: 837 / 943"))
    }

    @Test
    fun plazaSearchReusesTheFriendHomeArtworkAcrossAndroidAndWeb() {
        val androidSearch = repositoryRoot().resolve(
            "android/app/src/main/res/drawable-nodpi/ic_top_search_custom.png"
        )
        val webSearch = repositoryRoot().resolve("server/src/assets/project_view_search_icon.png")
        assertEquals(sha256(androidSearch), sha256(webSearch))

        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        ).substringAfter("private fun buildDiscoveryHeader")
            .substringBefore("private fun buildSearchBar")
        assertTrue(androidSource.contains("R.drawable.ic_top_search_custom"))
        assertTrue(androidSource.contains("PLAZA_SEARCH_END_MARGIN_DP"))

        val webView = readRepositoryFile("pc-frontend/src/features/plaza/ProjectPlazaView.tsx")
        assertTrue(webView.contains("project_view_search_icon.png"))
        assertTrue(webView.contains("src={sharedTopSearchAsset}"))

        val mobileWeb = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(mobileWeb.contains("data-search-artwork=\"/assets/project_view_search_icon.png\""))
    }

    @Test
    fun plazaListUsesComfortableVerticalSpacingAcrossAndroidAndMobileWeb() {
        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        assertTrue(androidSource.contains("LIST_ROW_GAP_PX = 64"))

        val mobileWeb = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(mobileWeb.contains("#projectPlazaInlineRoot .project-plaza-results { gap: 38px; }"))
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
