package com.elon.app

import java.io.File
import org.junit.Assert.assertTrue
import org.junit.Test

class InputVoiceButtonClippingContractTest {
    @Test
    fun initialVoiceActionKeepsAHostAtLeastAsWideAsTheButton() {
        val androidSource = read("android/app/src/main/kotlin/com/elon/app/MainSendButtonVisualActions.kt")
        assertTrue(
            "The 48dp initial voice action must not be placed inside the 38dp send-action host",
            androidSource.contains(
                "if (visualMode == WebChatProductionComposerVisualMode.INPUT_MODE) 48 else 38"
            ),
        )

        val pwaSource = read("server/src/assets/web_page.html")
        assertTrue(pwaSource.contains(".input-bar .voice-btn {"))
        assertTrue(pwaSource.contains("flex-basis: 48px;"))
        assertTrue(pwaSource.contains("width: 48px;"))
    }

    private fun read(relativePath: String): String {
        val candidates = sequenceOf(
            File(relativePath),
            File("../$relativePath"),
            File("../../$relativePath")
        )
        return candidates.firstOrNull(File::isFile)?.readText()
            ?: error("Missing contract source: $relativePath")
    }
}
