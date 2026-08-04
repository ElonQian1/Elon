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
    fun androidKeepsTheApkProjectDossierAsThePrimaryDesign() {
        val marketplace = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val featured = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProjectPlazaFeaturedSection.kt"
        )
        listOf(
            "R.drawable.project_plaza_ui4_heart",
            "R.drawable.project_plaza_ui5_star"
        ).forEach { resource -> assertTrue(featured.contains(resource)) }
        assertTrue(marketplace.contains("featuredSection.build(projects.take(5))"))
        assertTrue(featured.contains("text = \"精选项目\""))
        assertTrue(featured.contains("val action = primaryAction(project)"))
        assertTrue(featured.contains("projectPlazaProjectCover("))
        assertTrue(featured.contains("val build = projectPlazaBuildStatus(project.lastTaskStatus)"))
        assertTrue(featured.contains("FEATURED_CARD_WIDTH_FRACTION = 0.6871795f"))
        assertTrue(featured.contains("FEATURED_CARD_HEIGHT_RATIO = 1.2014925f"))
        assertTrue(featured.contains("FEATURED_CARD_GAP_DP = 10"))
        assertTrue(featured.contains("R.color.elon_plaza_surface_header"))
        assertTrue(featured.contains("R.color.elon_plaza_signal"))
        assertTrue(featured.contains("R.color.elon_plaza_action"))
        assertTrue(featured.contains("R.color.elon_status_success"))
        assertTrue(featured.contains("R.color.elon_status_danger"))
        assertTrue(featured.contains("FEATURE_RAIL_WIDTH_DP = 3"))
        assertFalse(featured.contains("R.drawable.project_plaza_ui1_card"))
        assertFalse(featured.contains("R.drawable.project_plaza_ui3_avatar"))
    }

    @Test
    fun mobilePwaMirrorsTheApkFeaturedCarouselAndListRhythm() {
        val script = readRepositoryFile("server/src/assets/project_plaza.js")
        val styles = readRepositoryFile("server/src/assets/project_plaza.css")
        val page = readRepositoryFile("server/src/assets/web_page.html")

        assertTrue(script.contains("project-plaza-featured-scroller"))
        assertTrue(script.contains("const featuredProjects = state.projects.slice(0, 5)"))
        assertTrue(script.contains("project-plaza-featured-status"))
        assertTrue(script.contains("project-plaza-featured-cover"))
        assertTrue(script.contains("project-plaza-featured-facts"))
        assertTrue(script.contains("project-plaza-featured-primary"))
        assertTrue(script.contains("const action = primaryAction(project)"))
        assertTrue(script.contains("const build = projectBuildStatus(project)"))
        assertTrue(script.contains("data-plaza-featured-position"))
        assertFalse(script.contains("project-plaza-featured-media"))
        assertTrue(styles.contains("width: 68.718vw;"))
        assertTrue(styles.contains("aspect-ratio: 268 / 322;"))
        assertTrue(styles.contains("gap: 10px;"))
        assertTrue(styles.contains("padding: 0 98px 0 20px;"))
        assertTrue(styles.contains("--plaza-bg: #04070b;"))
        assertTrue(styles.contains("--plaza-header: #0c1724;"))
        assertTrue(styles.contains("--plaza-primary: #6ed8ff;"))
        assertTrue(styles.contains("--plaza-action: #c9e7f5;"))
        assertTrue(styles.contains("--plaza-success: #5ac8a0;"))
        assertTrue(styles.contains("--plaza-danger: #f07884;"))
        assertTrue(styles.contains("border-left: 3px solid var(--plaza-primary);"))
        assertTrue(page.contains("data-ui-design=\"apk-deep-space-observatory-v4\""))
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
        assertEquals(1, Regex(Regex.escape("R.drawable.project_view_chevron")).findAll(androidSource).count())
        assertEquals(1, Regex(Regex.escape("/assets/project_view_chevron.png")).findAll(webScript).count())
        assertFalse(androidSource.contains("text = \"›\""))
        assertFalse(webScript.contains(">›<"))
    }

    @Test
    fun carouselAndListActionsKeepFortyEightPixelTouchTargets() {
        val androidSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val featuredSource = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProjectPlazaFeaturedSection.kt"
        )
        val webStyles = readRepositoryFile("server/src/assets/project_plaza.css")
        val projectListRow = androidSource.substringAfter("private fun buildProjectListRow")
            .substringBefore("private fun projectThumbnail")

        assertTrue(projectListRow.contains("LinearLayout.LayoutParams(dp(48), dp(48))"))
        assertTrue(featuredSource.contains("ACTION_HEIGHT_DP = 48"))
        assertTrue(featuredSource.contains("LinearLayout.LayoutParams(0, dp(ACTION_HEIGHT_DP), 1f)"))
        assertTrue(webStyles.contains(".project-plaza-featured-primary"))
        assertTrue(webStyles.contains(".project-plaza-reaction"))
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
        assertTrue(androidSource.contains("SEARCH_HEIGHT_DP = 56"))
        assertTrue(androidSource.contains("SEARCH_RADIUS_DP = 28"))
        assertTrue(webScript.contains("placeholder=\"搜索项目、作者\""))
        assertTrue(mobileWeb.contains("data-search-artwork=\"/assets/project_view_search_icon.png\""))
    }

    @Test
    fun feedbackAndListDossierRemainActionableAndDataDriven() {
        val marketplace = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val feedback = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProjectPlazaFeedbackSection.kt"
        )
        val membership = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProjectPlazaMembershipActionController.kt"
        )
        val script = readRepositoryFile("server/src/assets/project_plaza.js")
        val styles = readRepositoryFile("server/src/assets/project_plaza.css")

        assertTrue(marketplace.contains("LIST_ROW_MIN_HEIGHT_DP = 112"))
        assertTrue(marketplace.contains("projectPlazaProjectCover("))
        assertTrue(marketplace.contains("buildProjectListMeta(project)"))
        assertTrue(feedback.contains("没有找到匹配项目"))
        assertTrue(feedback.contains("重新加载"))
        assertTrue(membership.contains("requestJoinStoreProject"))
        assertTrue(membership.contains("joinStoreProject"))
        assertTrue(script.contains("state.pendingIds.add(id)"))
        assertTrue(script.contains("project-plaza-list-meta"))
        assertTrue(styles.contains("min-height: 112px;"))
        assertTrue(styles.contains(".project-plaza-feedback"))
        assertTrue(styles.contains(".project-plaza-skeleton-row"))
    }

    @Test
    fun plazaUsesCacheFirstRefreshAndDelayedSkeletonOnBothSurfaces() {
        val marketplace = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainMarketplaceActions.kt"
        )
        val coordinator = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProjectPlazaLoadCoordinator.kt"
        )
        val cache = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProjectPlazaCache.kt"
        )
        val script = readRepositoryFile("server/src/assets/project_plaza.js")
        val webCache = readRepositoryFile("server/src/assets/project_plaza_cache.js")
        val styles = readRepositoryFile("server/src/assets/project_plaza.css")

        assertTrue(marketplace.contains("ProjectPlazaLoadCoordinator"))
        assertTrue(coordinator.contains("onCached(fallback, exact != null)"))
        assertTrue(coordinator.contains("PROJECT_PLAZA_SKELETON_DELAY_MS"))
        assertTrue(cache.contains("PROJECT_PLAZA_FRESH_MS = 60_000L"))
        assertTrue(cache.contains("AuthManager.userDataPrefs(context)"))
        assertTrue(script.contains("const CACHE_KEY = 'elon_project_plaza_snapshot_v1'"))
        assertTrue(script.contains("const SKELETON_DELAY_MS = 180"))
        assertTrue(script.contains("ElonProjectPlazaCache.write(CACHE_KEY, snapshot)"))
        assertTrue(webCache.contains("window.ElonProjectPlazaCache"))
        assertTrue(script.contains("state.cacheNotice = '使用缓存 · 同步失败，点击重试'"))
        assertTrue(styles.contains(".project-plaza-cache-notice"))
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
