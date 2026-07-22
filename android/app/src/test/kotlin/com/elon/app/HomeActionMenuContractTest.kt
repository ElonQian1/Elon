package com.elon.app

import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.security.MessageDigest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class HomeActionMenuContractTest {
    @Test
    fun suppliedHomeAssetsAreTrackedWithoutAlteration() {
        val directory = repositoryRoot().resolve("android/app/src/main/res/drawable-nodpi")
        val expected = mapOf(
            "ic_home_ai_avatar.png" to "2E8EB21119085B110639B68F236E83D6CE961C86E1B05BEEFB70F1A056E91971",
            "ic_home_action_group.png" to "A03C5EA153A1321378153CAD547344D9E825B295335897DC5A526EEA1FD49EE0",
            "ic_home_action_add_friend.png" to "9A7F5201ACC2343A792A5FFDF068D3011F2BD001C85454470BBC391C56327006",
            "ic_home_action_new_project.png" to "DFBD26470E92A4897179A84DA27CC91F5058FAF5D15D12CBB230A51498BCC569"
        )

        expected.forEach { (name, expectedHash) ->
            val path = directory.resolve(name)
            assertTrue("missing $name", Files.isRegularFile(path))
            assertEquals("$name dimensions", 97 to 97, readPngDimensions(path))
            assertEquals("$name hash", expectedHash, sha256(path))
        }
    }

    @Test
    fun androidMovesHomeActionsToTheBottomPlusButton() {
        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        val navigation = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainNavigationController.kt"
        )
        val bottomNavigation = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainBottomNavigationController.kt"
        )
        val popup = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/MainActionPopups.kt"
        )
        val plusIcon = readRepositoryFile(
            "android/app/src/main/res/drawable/ic_bottom_action_plus.xml"
        )

        assertTrue(layout.contains("android:id=\"@+id/bottomActionPlusIcon\""))
        assertTrue(layout.contains("android:src=\"@drawable/ic_bottom_action_plus\""))
        assertTrue(layout.contains("android:layout_width=\"24dp\""))
        assertTrue(layout.contains("android:layout_height=\"24dp\""))
        assertTrue(plusIcon.contains("android:pathData=\"M12,4 L12,20 M4,12 L20,12\""))
        assertTrue(plusIcon.contains("android:strokeWidth=\"2\""))
        assertTrue(layout.contains("android:src=\"@drawable/ic_home_ai_avatar\""))
        assertTrue(navigation.contains("binding.addButton.visibility = View.GONE"))
        assertTrue(bottomNavigation.contains("showHomeActions(binding.bottomComposeButton, tab)"))
        assertTrue(popup.contains("renderer().showBottomActionPopup"))
        assertTrue(popup.contains("anchor === binding.bottomComposeButton"))
        assertTrue(popup.contains("binding.bottomActionPlusIcon"))
        assertTrue(popup.contains(".rotation(if (expanded) 45f else 0f)"))
        assertTrue(popup.contains("R.drawable.ic_home_action_group"))
        assertTrue(popup.contains("R.drawable.ic_home_action_add_friend"))
        assertTrue(popup.contains("R.drawable.ic_home_action_new_project"))
    }

    @Test
    fun webMirrorUsesTheSameBottomMenuAndAvatarAssets() {
        val web = readRepositoryFile("server/src/assets/web_page.html")
        val server = readRepositoryFile("server/src/web.rs")

        assertTrue(web.contains("id=\"homeActionMenu\""))
        assertTrue(web.contains(".bottom-compose-button[aria-expanded=\"true\"]::before"))
        assertTrue(web.contains("width: 16px; height: 2px;"))
        assertTrue(web.contains("<span>发起群聊</span>"))
        assertTrue(web.contains("<span>添加好友</span>"))
        assertTrue(web.contains("<span>新建项目</span>"))
        assertTrue(web.contains("__HOME_AI_AVATAR_PNG_B64__"))
        assertTrue(web.contains("#addBtn {"))
        assertTrue(web.contains("display: none !important;"))
        assertTrue(server.contains("ic_home_ai_avatar.png"))
        assertTrue(server.contains("ic_home_action_group.png"))
        assertTrue(server.contains("ic_home_action_add_friend.png"))
        assertTrue(server.contains("ic_home_action_new_project.png"))
    }

    private fun readRepositoryFile(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun readPngDimensions(path: Path): Pair<Int, Int> {
        val bytes = Files.readAllBytes(path)
        assertTrue("invalid PNG header: $path", bytes.size >= 24)
        val header = ByteBuffer.wrap(bytes, 16, 8).order(ByteOrder.BIG_ENDIAN)
        return header.int to header.int
    }

    private fun sha256(path: Path): String = MessageDigest.getInstance("SHA-256")
        .digest(Files.readAllBytes(path))
        .joinToString("") { "%02X".format(it) }

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .firstOrNull { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
            ?: error("Unable to locate repository root from $cwd")
    }
}
