package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionNavigationTouchTargetContractTest {
    @Test
    fun productionNavigationAndRichPartsUseFortyEightDpTargets() {
        val sideMenu = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptWebSideMenuView.kt",
        )
        val richContent = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionRichContent.kt",
        )

        assertTrue(sideMenu.contains("LinearLayout.LayoutParams(dp(48), dp(48))"))
        assertTrue(richContent.contains("minHeight = dp(container, 48)"))
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
