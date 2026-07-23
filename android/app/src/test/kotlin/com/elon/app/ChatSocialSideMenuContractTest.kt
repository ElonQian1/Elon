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

class ChatSocialSideMenuContractTest {
    @Test
    fun allFiveUserAssetsAreTrackedByteForByteAndUsedByAndroid() {
        val expected = mapOf(
            "social_sidebar_date_pill.png" to AssetProof(
                123,
                235,
                "7bb51f938e541b7e7c2f4e7d0804471b86747b43c05677af4404361d2ad04f22"
            ),
            "social_sidebar_avatar_placeholder.png" to AssetProof(
                110,
                108,
                "be28db51b7feb7b8d7e709b78de3ce29ffa7ec9d0cc7d8f25c2c636f7fa02094"
            ),
            "social_sidebar_timeline_dot.png" to AssetProof(
                51,
                50,
                "74fa3e1581b7ea9567af5dd2be85766f0b6e66509305018b07a5601c05386295"
            ),
            "social_sidebar_search.png" to AssetProof(
                62,
                57,
                "e25db170e32daba836fc9bb74b45007a9da7820f8624f02609feaca93a3fcc83"
            ),
            "social_sidebar_play.png" to AssetProof(
                44,
                57,
                "4e8f304cfbf740344569981fd789e2fec02359d6d13b9689b0e426f0998aeb40"
            )
        )
        val directory = repositoryRoot().resolve("android/app/src/main/res/drawable-nodpi")
        val source = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSocialSideMenuView.kt"
        ) + readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSocialSideMenuDateStrip.kt"
        )

        expected.forEach { (name, proof) ->
            val path = directory.resolve(name)
            assertTrue("missing $name", Files.isRegularFile(path))
            assertEquals("$name sha", proof.sha256, sha256(path))
            assertEquals("$name dimensions", proof.width to proof.height, readPngDimensions(path))
            assertTrue(source.contains("R.drawable.${name.removeSuffix(".png")}"))
        }
    }

    @Test
    fun socialModeIsIsolatedFromTheAiProjectSidebar() {
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSideMenuController.kt"
        )
        val coordinator = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSocialSideMenuCoordinator.kt"
        )
        val aiSidebar = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatAiSideMenuView.kt"
        )

        assertTrue(controller.contains("socialSideMenu: ChatSocialSideMenuCoordinator"))
        assertTrue(coordinator.contains("view = ChatSocialSideMenuView("))
        assertTrue(controller.contains("showProjectShareSideMenu() ->"))
        assertTrue(coordinator.contains("view.visibility = View.VISIBLE"))
        assertTrue(controller.contains("aiMenuView.visibility = View.VISIBLE"))
        assertFalse(aiSidebar.contains("ChatSocialSideMenuView"))
        assertFalse(aiSidebar.contains("social_sidebar_"))
        assertFalse(aiSidebar.contains("SocialSidebarTab"))
    }

    @Test
    fun dateFavoritesSearchFiltersAndDragSendAreRealInteractivePaths() {
        val view = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSocialSideMenuView.kt"
        ) + readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSocialSideMenuDock.kt"
        ) + readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSocialSideMenuDateStrip.kt"
        )
        val controller = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSideMenuController.kt"
        ) + readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSocialSideMenuCoordinator.kt"
        )
        val activityActions = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainSocialSidebarActions.kt"
        )
        val activity = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainActivity.kt"
        )
        val loader = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/ChatSocialSidebarMessageLoader.kt"
        )

        assertTrue(view.contains("SocialSidebarTab.DATE"))
        assertTrue(view.contains("SocialSidebarTab.FAVORITES"))
        assertTrue(view.contains("搜索侧栏消息"))
        assertTrue(view.contains("LinearLayout.LayoutParams(dp(124)"))
        assertTrue(view.contains("LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)"))
        assertTrue(view.contains("setPadding(dp(14), dp(15), dp(14), dp(15))"))
        assertTrue(view.contains("FrameLayout.LayoutParams(dp(47), dp(90))"))
        assertTrue(view.contains("if (selected) \"#464646\" else \"#D9D9D9\""))
        listOf("图片与视频", "文本", "链接", "笔记", "文件", "设置")
            .forEach { assertTrue(view.contains("\"$it\"")) }
        assertTrue(view.contains("SocialTimelineDragPayload"))
        assertTrue(controller.contains("event.x > panelWidth"))
        assertTrue(controller.contains("sendTimelineMessage(payload.message)"))
        assertTrue(activityActions.contains("trySendForwardedMessage(message)"))
        assertTrue(loader.contains("preserve_unread=true"))
        assertTrue(view.contains("current?.lastReceivedAt == item.lastReceivedAt"))
        assertEquals(3, "refreshChatTabBadge\\(\\)".toRegex()
            .findAll(activity.substringAfter("private val friendActions"))
            .count())
        assertTrue(
            activity.substringAfter("private fun refreshChatTabBadge")
                .substringBefore("private fun showGitProjectDialog")
                .contains("chatSideMenuController.refreshVisibleContent()")
        )
    }

    @Test
    fun webMirrorHasOnlyTheExistingProjectSidebarSoSocialSidebarNeedsNoWebCopy() {
        val web = readRepositoryFile("server/src/assets/web_page.html")

        assertTrue(web.contains("aria-label=\"项目侧边栏\""))
        assertTrue(web.contains("await loadProjects();"))
        assertFalse(web.contains("social_sidebar"))
        assertFalse(web.contains("图片与视频"))
        assertFalse(web.contains("好友侧栏"))
    }

    private fun readRepositoryFile(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun readPngDimensions(path: Path): Pair<Int, Int> {
        val bytes = Files.readAllBytes(path)
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

    private data class AssetProof(val width: Int, val height: Int, val sha256: String)
}
