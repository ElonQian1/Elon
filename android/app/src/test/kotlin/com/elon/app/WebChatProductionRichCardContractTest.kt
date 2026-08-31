package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionRichCardContractTest {
    @Test
    fun validatedPrivateCardsUseTheNativeProductionRendererAndViewer() {
        val binder = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionRichContent.kt",
        )
        val views = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionRichCardViews.kt",
        )
        val controller = read(
            "android/app/src/main/kotlin/com/elon/app/ChatGptSocialImageContentController.kt",
        )
        val preview = read(
            "android/app/src/debug/kotlin/com/elon/app/WebChatRichCardPreviewScenario.kt",
        )

        assertTrue(binder.contains("WebChatProductionRichCardViews.inline"))
        assertTrue(views.contains("WebChatProductionLineChartView"))
        assertTrue(views.contains("BottomSheetDialog"))
        assertTrue(views.contains("internal fun detail"))
        assertTrue(controller.contains("WebChatProductionRichCardViews.show"))
        assertTrue(preview.contains("WebChatProductionRichCardViews.inline"))
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
