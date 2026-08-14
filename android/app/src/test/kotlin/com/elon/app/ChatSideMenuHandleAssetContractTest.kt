package com.elon.app

import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.security.MessageDigest
import java.util.Base64
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatSideMenuHandleAssetContractTest {
    @Test
    fun userProvidedHandleIsSharedByAndroidAndWebWithoutDistortion() {
        val root = repositoryRoot()
        val png = Files.readAllBytes(
            root.resolve("android/app/src/main/res/drawable-nodpi/ic_chat_side_menu_handle.png")
        )
        val webPng = Base64.getDecoder().decode(
            Files.readString(root.resolve("server/src/assets/ic_chat_side_menu_handle.b64")).trim()
        )
        val layout = Files.readString(root.resolve("android/app/src/main/res/layout/activity_main.xml"))
        val web = Files.readString(root.resolve("server/src/assets/web_page.html"))

        assertEquals("472b15102dbc18d2b2df5bdbdc478dbac911205c868763aa54b2d1413e417712", sha256(png))
        assertArrayEquals(png, webPng)
        assertTrue(layout.contains("android:layout_height=\"80dp\""))
        assertTrue(layout.contains("android:paddingEnd=\"33dp\""))
        assertTrue("width: 15px;\\s+height: 68px;".toRegex().containsMatchIn(web))
    }

    private fun sha256(bytes: ByteArray): String = MessageDigest.getInstance("SHA-256")
        .digest(bytes)
        .joinToString("") { "%02x".format(it) }

    private fun repositoryRoot(): Path {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        return generateSequence(cwd) { it.parent }
            .take(6)
            .first { Files.isRegularFile(it.resolve("android/app/build.gradle")) }
    }
}
