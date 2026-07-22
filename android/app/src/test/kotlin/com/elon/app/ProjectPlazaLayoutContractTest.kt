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
        assertTrue(source.contains("FEATURED_CARD_WIDTH_FRACTION = 0.656f"))
        assertTrue(source.contains("FEATURED_CARD_HEIGHT_RATIO = 1.126f"))
        assertTrue(source.contains("FEATURED_CARD_GAP_DP = 22"))
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
    fun plazaListArrowMatchesTheProfileChevronContainerStandard() {
        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val projectListRow = androidSource.substringAfter("private fun buildProjectListRow")
            .substringBefore("private fun projectThumbnail")
        val listArrow = requireNotNull(
            Regex(
                """setImageResource\(R\.drawable\.project_view_chevron\)([\s\S]*?)""" +
                    """\}, LinearLayout\.LayoutParams\(dp\((\d+)\), dp\((\d+)\)\)\)"""
            ).find(projectListRow)
        )
        val listPadding = requireNotNull(
            Regex(
                """setPadding\(dp\((\d+)\), dp\((\d+)\), dp\((\d+)\), dp\((\d+)\)\)"""
            ).find(listArrow.groupValues[1])
        )

        val profileLayout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        val profileArrow = profileLayout.substringAfter(
            "android:id=\"@+id/profileNodeResourceArrow\""
        ).substringBefore("/>")
        val profileWidth = xmlDpAttribute(profileArrow, "android:layout_width")
        val profileHeight = xmlDpAttribute(profileArrow, "android:layout_height")
        val profilePadding = xmlDpAttribute(profileArrow, "android:padding")

        assertEquals(32, profileWidth)
        assertEquals(48, profileHeight)
        assertEquals(8, profilePadding)
        assertEquals(profileWidth, listArrow.groupValues[2].toInt())
        assertEquals(profileHeight, listArrow.groupValues[3].toInt())
        listPadding.groupValues.drop(1).forEach { value ->
            assertEquals(profilePadding, value.toInt())
        }
        assertEquals(16, listArrow.groupValues[2].toInt() - (profilePadding * 2))
    }

    @Test
    fun plazaFeaturedArrowUsesTheSameSixteenDpBitmapInsideItsRoundButton() {
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

        assertEquals(28, featuredArrow.groupValues[2].toInt())
        assertEquals(28, featuredArrow.groupValues[3].toInt())
        featuredPadding.forEach { assertEquals(6, it) }
        assertEquals(16, featuredArrow.groupValues[2].toInt() - featuredPadding[0] - featuredPadding[2])
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
    fun webPlazaFeaturedChevronIsSixteenPixelsWhileItsSpinnerRemainsTwentyTwo() {
        val styles = readRepositoryFile("pc-frontend/src/features/plaza/PlazaPage.module.css")
        assertTrue(
            Regex("""\.primaryAction\s*\{[\s\S]*?width:\s*32px;[\s\S]*?height:\s*32px;[\s\S]*?\}""")
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
        assertTrue(styles.contains("65.6vw"))
        assertTrue(styles.contains("aspect-ratio: 837 / 943"))
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

    private fun xmlDpAttribute(fragment: String, attribute: String): Int = requireNotNull(
        Regex("""${Regex.escape(attribute)}=\"(\d+)dp\"""").find(fragment)
    ).groupValues[1].toInt()

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Unable to locate repository root from $cwd")
    }
}
