package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatProductionVoiceEntryContractTest {
    @Test
    fun friendChatPhoneRoutesWebChatBeforeLegacyWorkVoice() {
        val activity = read("android/app/src/main/kotlin/com/elon/app/MainActivity.kt")
        val route = activity.substringAfter("private fun openSocialAiVoiceCall()")
            .substringBefore("private fun suspendSocialChatForProjectReturn()")

        assertTrue(route.contains("socialAiChatFeature.startWebChatRealtimeVoice()"))
        assertTrue(
            route.indexOf("startWebChatRealtimeVoice") <
                route.indexOf("SocialAiVoiceCallActivity.createIntent"),
        )
    }

    @Test
    fun productionFeatureDelegatesToTheCurrentProviderToolCoordinator() {
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val tools = read("android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerTools.kt")

        assertTrue(feature.contains("fun startWebChatRealtimeVoice(): Boolean"))
        assertTrue(feature.contains("productionComposerTools.startRealtimeVoice"))
        assertTrue(tools.contains("WebChatProviderCapability.REALTIME_VOICE"))
        assertTrue(tools.contains("chatgpt_start_realtime_voice"))
        assertTrue(tools.contains("openOfficialFallback()"))
        assertFalse(tools.contains("ChatGptWebTestActivity"))
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
