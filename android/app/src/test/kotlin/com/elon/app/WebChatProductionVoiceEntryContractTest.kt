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
        assertTrue(tools.contains("startNativeRealtimeVoice"))
        assertFalse(tools.contains("ChatGptWebTestActivity"))
        assertTrue(directRoute.contains("executeCommand(provider, command)"))
        assertTrue(commandRoute.contains("command.action == REALTIME_VOICE_ACTION"))
        assertTrue(commandRoute.contains("startNativeRealtimeVoice(provider)"))
        assertFalse(commandRoute.contains("openOfficialRealtimeVoice()"))
        assertTrue(feature.contains("WebChatRealtimeVoiceOverlay"))
        assertTrue(feature.contains("createWebChatRealtimeVoiceCoordinator"))
        assertTrue(feature.contains("openOfficialFallback = modeController::openOfficialRealtimeVoice"))
        assertTrue(feature.contains("beginRealtimeVoiceBacking"))
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

    @Test
    fun nativeRealtimeVoiceUsesAStableTitleAndPreservesConversationDuringExit() {
        val surface = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatRealtimeVoiceSurface.kt",
        )
        val backing = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/" +
                "ChatGptRealtimeVoiceBackingController.kt",
        )
        val controller = read(
            "android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt",
        )

        assertTrue(surface.contains("text = \"语音 AI\""))
        assertFalse(surface.contains("text = \"ChatGPT 网页 AI\""))
        assertTrue(backing.contains("if (!gracefulExit) view.reload()"))
        assertFalse(backing.contains("view.stopLoading()"))
        assertTrue(controller.contains("backingActive = realtimeVoiceBackingStarted"))
        assertFalse(controller.contains("renderStatusMessage(\"正在同步语音会话…\")"))
    }

    @Test
    fun productionVoiceGatesOnlyConfirmedGuestsAndDelegatesCredentialsToTheOfficialPage() {
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val coordinator = read("android/app/src/main/kotlin/com/elon/app/WebChatRealtimeVoiceCoordinator.kt")
        val gate = read("android/app/src/main/kotlin/com/elon/app/WebChatRealtimeVoiceLoginGate.kt")
        val mode = read("android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt")
        val strings = read("android/app/src/main/res/values/strings.xml")

        assertTrue(feature.contains("authenticated = { isChatModeActive() && activeController().authenticated() }"))
        assertTrue(feature.contains("sessionState ="))
        assertTrue(feature.contains("openOfficialLogin = modeController::openOfficialLogin"))
        assertTrue(coordinator.contains("WebChatRealtimeVoiceAuthenticationState.GUEST"))
        assertTrue(coordinator.contains("WebChatRealtimeVoiceAuthenticationState.AUTHENTICATED"))
        assertTrue(mode.contains("ChatGptWebOfficialFallbackIntent.createLogin(activity)"))
        assertTrue(gate.contains("web_chat_realtime_voice_login_methods"))
        assertTrue(strings.contains("官方可能提供的登录方式"))
        assertFalse(gate.contains("EditText"))
        assertFalse(gate.contains("password"))
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
