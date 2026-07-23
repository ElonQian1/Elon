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

class ToolbarBackIconContractTest {
    @Test
    fun suppliedBackIconIsTrackedWithoutAlteration() {
        val path = repositoryRoot()
            .resolve("android/app/src/main/res/drawable-nodpi/ic_toolbar_back_custom.png")

        assertTrue("missing toolbar back icon", Files.isRegularFile(path))
        assertEquals(43 to 43, readPngDimensions(path))
        assertEquals(
            "3147797D74D7EE606612A79224216210865D6B783273FC3166EB332F3ED37275",
            sha256(path)
        )
    }

    @Test
    fun androidUsesCompactImageInsideAccessibleTouchTarget() {
        val layout = readRepositoryFile("android/app/src/main/res/layout/activity_main.xml")
        val backButton = layout.substring(
            layout.indexOf("<ImageButton\n                android:id=\"@+id/backButton\""),
            layout.indexOf("/>", layout.indexOf("android:id=\"@+id/backButton\"")) + 2
        )

        assertTrue(backButton.contains("android:layout_width=\"50dp\""))
        assertTrue(backButton.contains("android:layout_height=\"50dp\""))
        assertTrue(backButton.contains("android:padding=\"14dp\""))
        assertTrue(backButton.contains("android:scaleType=\"fitCenter\""))
        assertTrue(backButton.contains("android:scaleX=\"-1\""))
        assertTrue(backButton.contains("android:src=\"@drawable/ic_toolbar_back_custom\""))
        assertTrue(backButton.contains("android:contentDescription=\"返回\""))
    }

    @Test
    fun webMirrorUsesTheSameBackIconAsset() {
        val web = readRepositoryFile("server/src/assets/web_page.html")
        val server = readRepositoryFile("server/src/web.rs")

        assertTrue(web.contains("id=\"backBtn\" title=\"返回\" aria-label=\"返回\""))
        assertTrue(web.contains("__TOOLBAR_BACK_ICON_PNG_B64__"))
        assertTrue(web.contains("#backBtn {"))
        assertTrue(web.contains("padding: 13px;"))
        assertTrue(web.contains("#backBtn img { transform: scaleX(-1); }"))
        assertTrue(server.contains("ic_toolbar_back_custom.png"))
        assertTrue(server.contains("\"__TOOLBAR_BACK_ICON_PNG_B64__\""))
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
