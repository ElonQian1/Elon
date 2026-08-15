package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class HomeConversationHeaderContractTest {
    @Test
    fun homeHeaderOmitsUnreadFilterAndMarkAllReadControlOnBothSurfaces() {
        val androidHeader = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/HomeConversationHeaderView.kt"
        )
        assertFalse(androidHeader.contains("HomeListFilterMode.Unread to"))
        assertFalse(androidHeader.contains("全部已读"))
        assertTrue(androidHeader.contains("HomeListFilterMode.Friends to"))
        assertTrue(androidHeader.contains("HomeListFilterMode.Projects to"))
        assertTrue(androidHeader.contains("HomeListFilterMode.Conversations to"))

        val web = readRepositoryFile("server/src/assets/web_page.html")
        assertFalse(web.contains("全部已读"))
        assertTrue(web.contains("free of standalone unread-filter and mark-all-read controls"))
    }

    private fun readRepositoryFile(relativePath: String): String {
        val cwd = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        val path: Path = generateSequence(cwd) { it.parent }
            .map { it.resolve(relativePath) }
            .take(6)
            .firstOrNull(Files::isRegularFile)
            ?: error("Unable to find $relativePath from $cwd")
        return String(Files.readAllBytes(path), StandardCharsets.UTF_8)
    }
}
