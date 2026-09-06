package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionAttachmentPanelContractTest {
    @Test
    fun webChatHidesWorkOnlyUiDesignActionButKeepsRealAttachmentPickers() {
        val input = read("android/app/src/main/kotlin/com/elon/app/MainInputActions.kt")
        val panel = read("android/app/src/main/kotlin/com/elon/app/MainAttachmentPanelActions.kt")

        assertTrue(input.contains("binding.modelButton.tag != WEB_CHAT_MODEL_BUTTON_OWNER"))
        assertTrue(panel.contains("uiDesignAction?.visibility"))
        for ((selector, handler) in listOf(
            "attachment-action-camera" to "openCameraAttachment",
            "attachment-action-photos" to "openPhotoAttachment",
            "attachment-action-files" to "openDocumentAttachment",
        )) {
            val binding = Regex("\"${Regex.escape(selector)}\"\\s*\\)\\s*\\{\\s*${Regex.escape(handler)}\\(\\)")
            assertTrue("Missing attachment action binding: $selector", binding.containsMatchIn(panel))
        }
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
