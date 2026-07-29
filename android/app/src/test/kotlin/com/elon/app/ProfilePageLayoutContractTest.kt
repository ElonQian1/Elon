package com.elon.app

import java.security.MessageDigest
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
        val profileViews = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/UserProfileViews.kt"
        )
        val memoriesCard = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/UserMemoriesCard.kt"
        )
        assertTrue(layout.contains("android:id=\"@+id/profilePageContent\""))
        assertTrue(layout.contains("android:paddingStart=\"20dp\""))
        assertTrue(profileViews.contains("setPadding(context.dp(16), context.dp(12), context.dp(10), context.dp(12))"))
        assertTrue(memoriesCard.contains("setPadding(dp(16), 0, dp(10), 0)"))
        assertTrue(layout.contains("android:paddingStart=\"16dp\""))
        assertTrue(layout.contains("android:paddingEnd=\"10dp\""))
        assertTrue(!layout.contains("android:paddingStart=\"36dp\""))
        assertTrue(layout.contains("android:background=\"@drawable/profile_panel_primary_actions_stable\""))
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
        assertTrue(tokenCard.contains("R.drawable.profile_pill_selected"))
        assertTrue(tokenCard.contains("R.drawable.profile_pill_unselected"))
        assertTrue(tokenCard.contains("private var selectedDays = 7"))
        assertTrue(tokenCard.contains("ProfileQuotaGaugeView"))
        val selectedPill = readRepositoryFile(
            "android/app/src/main/res/drawable/profile_pill_selected.xml"
        )
        assertTrue(selectedPill.contains("@color/elon_profile_quota_selected"))
        assertTrue(selectedPill.contains("@color/elon_profile_quota_border"))
        val unselectedPill = readRepositoryFile(
            "android/app/src/main/res/drawable/profile_pill_unselected.xml"
        )
        assertTrue(unselectedPill.contains("@color/elon_profile_quota_control"))
        assertTrue(unselectedPill.contains("@color/elon_profile_quota_border"))
    }

    @Test
    fun webMirrorKeepsTheSameProfileGeometryAndSections() {
        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(web.contains("data-tab=\"profilePage\" data-title=\"个人中心\""))
        assertTrue(web.contains(".profile-action-group"))
        assertTrue(web.contains("min-height: 284px"))
        assertTrue(web.contains("grid-template-columns: repeat(3"))
        assertTrue(web.contains("--profile-quota-selected: #5DA6FF"))
        assertTrue(web.contains("class=\"usage-period-button selected\" id=\"usageWeekBtn\""))
        assertTrue(web.contains("let profileUsageDays = 7"))
        assertTrue(web.contains("class=\"usage-gauge-progress\""))
        assertTrue(web.contains("padding: 12px 10px 12px 16px"))
        assertTrue(web.contains("padding: 0 10px 0 16px"))
        assertTrue(web.contains("--profile-action-group-radius: 18px"))
        val profileGroupCss =
            web.substringAfter("#profilePage .profile-action-group {").substringBefore("}")
        assertTrue(profileGroupCss.contains("border-radius: var(--profile-action-group-radius)"))
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
            "profile_panel_primary_actions_stable.9.png",
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

    @Test
    fun stretchablePrimaryPanelKeepsTheVerifiedCompleteBitmap() {
        val path = repositoryRoot().resolve(
            "android/app/src/main/res/drawable-nodpi/profile_panel_primary_actions_stable.9.png"
        )
        val bytes = Files.readAllBytes(path)
        fun pngInt(offset: Int): Int =
            ((bytes[offset].toInt() and 0xFF) shl 24) or
                ((bytes[offset + 1].toInt() and 0xFF) shl 16) or
                ((bytes[offset + 2].toInt() and 0xFF) shl 8) or
                (bytes[offset + 3].toInt() and 0xFF)

        assertTrue(pngInt(16) == 1148)
        assertTrue(pngInt(20) == 945)
        val sha256 = MessageDigest.getInstance("SHA-256")
            .digest(bytes)
            .joinToString("") { "%02x".format(it) }
        assertTrue(sha256 == "b4acf2bb56b76f6a6d4fb0e8bc556bb39eb8e576fc09f29fc0f5cf334e542b19")
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
