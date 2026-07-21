package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class ProfilePageLayoutContractTest {
    @Test
    fun androidProfileUsesTheProvidedPanelAndActionAssets() {
        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        assertTrue(layout.contains("android:id=\"@+id/profilePageContent\""))
        assertTrue(layout.contains("android:paddingStart=\"20dp\""))
        assertTrue(layout.contains("android:background=\"@drawable/profile_panel_primary_actions\""))
        assertTrue(layout.contains("android:background=\"@drawable/profile_panel_support_actions\""))
        listOf(
            "profile_icon_pc_node",
            "profile_icon_ai_settings",
            "profile_icon_agent_automation",
            "profile_icon_share",
            "profile_icon_check_update",
            "profile_icon_chevron"
        ).forEach { resource -> assertTrue(layout.contains("@drawable/$resource")) }

        val tokenCard = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ProfileTokenUsageCard.kt"
        )
        assertTrue(tokenCard.contains("R.drawable.profile_panel_quota"))
        assertTrue(tokenCard.contains("R.drawable.profile_pill_dark"))
        assertTrue(tokenCard.contains("R.drawable.profile_pill_light"))
        assertTrue(tokenCard.contains("ProfileQuotaGaugeView"))
    }

    @Test
    fun webMirrorKeepsTheSameProfileGeometryAndSections() {
        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(web.contains("data-tab=\"profilePage\" data-title=\"个人中心\""))
        assertTrue(web.contains(".profile-action-group"))
        assertTrue(web.contains("min-height: 284px"))
        assertTrue(web.contains("grid-template-columns: repeat(3"))
        assertTrue(web.contains("class=\"usage-gauge-progress\""))
        listOf("PC 节点", "AI 记忆", "AI 代理设置", "Agent 自动化", "分享推广", "检测更新")
            .forEach { label -> assertTrue(web.contains(label)) }
    }

    @Test
    fun copiedBitmapResourcesAreTrackedByTheProfileContract() {
        val root = repositoryRoot()
        val resources = listOf(
            "profile_panel_identity.png",
            "profile_panel_quota.png",
            "profile_panel_primary_actions.png",
            "profile_panel_support_actions.png",
            "profile_pill_dark.png",
            "profile_pill_light.png",
            "profile_gauge_tick_blue_short.png",
            "profile_gauge_tick_blue_long.png",
            "profile_gauge_tick_neutral.png",
            "profile_icon_ai_memory.png"
        )
        val directory = root.resolve("android/app/src/main/res/drawable-nodpi")
        resources.forEach { resource ->
            assertTrue("missing $resource", Files.isRegularFile(directory.resolve(resource)))
        }
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
