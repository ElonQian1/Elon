package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class HomeWorkspaceLayoutContractTest {
    @Test
    fun androidHomeUsesWorkspaceStructureAndPanelScopedPullGesture() {
        val dashboard = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/HomeWorkspaceDashboardView.kt"
        )
        assertTrue(dashboard.contains("PROJECT_SECTION_HEIGHT_DP = 153"))
        assertTrue(dashboard.contains("PROJECT_ITEM_HEIGHT_DP = 88"))
        assertTrue(dashboard.contains("PROJECT_ADD_GAP_DP = 8"))
        assertTrue(dashboard.contains("setPadding(dp(21), dp(3), dp(12), 0)"))
        assertTrue(dashboard.contains("FRIENDS_PANEL_FALLBACK_HEIGHT_DP = 555"))
        assertTrue(dashboard.contains("R.id.homeWorkspaceProjectStrip"))
        assertTrue(dashboard.contains("R.id.homeWorkspaceFriendsPanel"))
        assertTrue(dashboard.contains("R.drawable.bg_home_workspace_friends_panel"))

        val actions = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainHomeListActions.kt"
        )
        assertTrue(actions.contains("homeListFilterMode = HomeListFilterMode.Friends"))
        assertTrue(actions.contains("activationRegion = { event -> homeWorkspaceSurface?.contains(event) == true }"))
        assertTrue(actions.contains("stretchTarget = { homeWorkspaceSurface?.friendsPanel"))
        assertTrue(actions.contains("return \"你的工作室\""))
    }

    @Test
    fun homeBottomNavigationUsesOnlyTheThreeHundredDpPanel() {
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainBottomNavigationController.kt"
        )
        assertTrue(controller.contains("bottomComposeButton.visibility = if (enabled) View.GONE"))
        assertTrue(controller.contains("bottomComposeGap.visibility = if (enabled) View.GONE"))
        assertTrue(controller.contains("R.drawable.bg_home_workspace_bottom_nav"))
        assertTrue(controller.contains("params.marginStart = if (enabled) dp(10)"))
        assertTrue(controller.contains("params.marginEnd = if (enabled) dp(10)"))

        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        assertTrue(layout.contains("@+id/bottomNavPrimaryBackground"))
        assertTrue(layout.contains("@+id/bottomComposeGap"))

        val composer = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt"
        )
        assertTrue(composer.contains("id = R.id.chatInputCapsule"))
    }

    @Test
    fun webMirrorMatchesWorkspaceHomeAndPanelScopedPullGesture() {
        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertTrue(web.contains("id=\"workspaceHome\" class=\"workspace-home\""))
        assertTrue(web.contains("id=\"workspaceProjectStrip\""))
        assertTrue(web.contains("id=\"workspaceFriendsPanel\""))
        assertTrue(web.contains("topTitle.textContent = '你的工作室'"))
        assertTrue(web.contains(".tabs-bar.workspace-home"))
        assertTrue(web.contains("padding-left: 30px"))
        assertTrue(web.contains("workspaceFriendsPanel.addEventListener('touchstart'"))
        assertTrue(web.contains("workspaceFriendsPanel.style.transform"))
    }

    @Test
    fun suppliedWorkspaceAssetsAreCommittedAsAndroidResources() {
        listOf(
            "bg_home_workspace_friends_panel.png",
            "bg_home_workspace_bottom_nav.png",
            "bg_home_workspace_project_add_outline.png",
            "bg_home_workspace_project_add_inner.png",
            "bg_home_workspace_drag_handle.png",
            "bg_home_workspace_project_placeholder.png",
            "ic_home_workspace_chevron.png"
        ).forEach { file ->
            assertTrue(repositoryPath("android/app/src/main/res/drawable-nodpi/$file").toFile().isFile)
        }
    }

    private fun readRepositoryFile(relativePath: String): String {
        return String(Files.readAllBytes(repositoryPath(relativePath)), StandardCharsets.UTF_8)
    }

    private fun repositoryPath(relativePath: String): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .map { it.resolve(relativePath) }
            .take(6)
            .firstOrNull(Files::exists)
            ?: error("Unable to find $relativePath from $cwd")
    }
}
