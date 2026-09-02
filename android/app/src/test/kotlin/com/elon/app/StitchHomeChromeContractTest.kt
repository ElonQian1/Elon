package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class StitchHomeChromeContractTest {
    @Test
    fun conversationHomeUsesStitchMenuAndLeftAlignedTitle() {
        val source = readRepositoryFile(
            "android/app/src/main/kotlin/com/elon/app/HomeChromeController.kt"
        )
        assertTrue(source.contains("homeMenuButton.visibility = android.view.View.VISIBLE"))
        assertTrue(source.contains("Typeface.create(\"sans-serif-medium\""))
        assertTrue(source.contains("marginStart = dp(58)"))
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
