package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionComposerTouchTargetContractTest {
    @Test
    fun productionComposerUsesFortyEightDpTouchTargets() {
        val source = read("android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt")
        val motion = read("android/app/src/main/kotlin/com/elon/app/InputComposerMotion.kt")

        assertTrue(source.contains("FrameLayout.LayoutParams(dp(48), dp(48)"))
        assertTrue(source.contains("LinearLayout.LayoutParams(dp(48), dp(48))"))
        assertTrue(source.contains("LinearLayout.LayoutParams(dp(76), dp(48))"))
        assertTrue(source.contains("LinearLayout.LayoutParams(dp(72), dp(48))"))
        assertFalse(source.contains("dp(38)"))
        assertFalse(source.contains("dp(34)"))
        assertFalse(source.contains("dp(36)"))
        assertTrue(motion.contains("density * 48f"))
        assertFalse(motion.contains("density * 38f"))
    }

    private fun read(relativePath: String): String =
        String(Files.readAllBytes(repositoryRoot().resolve(relativePath)), StandardCharsets.UTF_8)

    private fun repositoryRoot(): Path {
        var current = Paths.get(System.getProperty("user.dir")).toAbsolutePath().normalize()
        repeat(5) {
            if (Files.exists(current.resolve("android/app/src/main"))) return current
            current = current.parent ?: return@repeat
        }
        error("Repository root not found from ${System.getProperty("user.dir")}")
    }
}
