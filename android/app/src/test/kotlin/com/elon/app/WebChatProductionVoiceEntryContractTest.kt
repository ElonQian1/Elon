package com.elon.app

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import org.junit.Assert.assertFalse
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
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
        val commands = read("android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerCommands.kt")
        val controls = read("android/app/src/main/kotlin/com/elon/app/WebChatProductionVoiceControls.kt")
        val composer = read("android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt")
        val directRoute = tools.substringAfter("fun startRealtimeVoice")
            .substringBefore("fun executeCommand")
        val commandRoute = tools.substringAfter("fun executeCommand")
            .substringBefore("fun cancelPending")

        assertTrue(feature.contains("fun startWebChatRealtimeVoice(): Boolean"))
        assertTrue(feature.contains("productionComposerTools.startRealtimeVoice"))
        assertTrue(feature.contains("productionVoiceControls.render"))
        assertTrue(feature.contains("productionVoiceControls.restoreLocalVoiceInput"))
        assertTrue(commands.contains("WebChatProviderCapability.REALTIME_VOICE"))
        assertTrue(tools.contains("chatgpt_start_realtime_voice"))
        assertTrue(tools.contains("openOfficialFallback()"))
        assertFalse(tools.contains("ChatGptWebTestActivity"))
        assertTrue(directRoute.contains("executeCommand(provider, command)"))
        assertTrue(commandRoute.contains("command.action == REALTIME_VOICE_ACTION"))
        assertTrue(commandRoute.contains("openOfficialFallback()"))
        assertFalse(commandRoute.contains("mcpPort()"))
        assertTrue(tools.contains("port.executeSessionCommand(pending.command.action)"))
        assertTrue(tools.contains("pendingSessionCommand"))
        assertTrue(controls.contains("REALTIME_VOICE_BLUE"))
        assertTrue(controls.contains("views.webDictationButton"))
        assertTrue(composer.contains("inputBarContainer.addView(webDictationButton)"))
    }

    @Test
    fun productionVoicePolicySeparatesDictationAndRealtimeVoice() {
        val chatGpt = WebChatProviderRegistry.get(WebChatProviderId.CHATGPT_WEB)
        val idle = WebChatProductionVoicePresentationPolicy.resolve(
            provider = chatGpt,
            streaming = false,
            dictationActive = false,
        )
        assertEquals("chatgpt_start_dictation", idle.dictation?.action)
        assertEquals("chatgpt_start_realtime_voice", idle.realtimeVoice?.action)

        val active = WebChatProductionVoicePresentationPolicy.resolve(
            provider = chatGpt,
            streaming = false,
            dictationActive = true,
        )
        assertEquals("chatgpt_submit_dictation", active.dictation?.action)
        assertNull(active.realtimeVoice)

        val google = WebChatProductionVoicePresentationPolicy.resolve(
            provider = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB),
            streaming = false,
            dictationActive = false,
        )
        assertNull(google.dictation)
        assertNull(google.realtimeVoice)
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
