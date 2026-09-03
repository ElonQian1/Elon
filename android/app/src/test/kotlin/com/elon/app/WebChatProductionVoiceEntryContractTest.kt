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
    fun friendChatPhoneStartsTheManagedAccountBoundDefault() {
        val activity = read("android/app/src/main/kotlin/com/elon/app/MainActivity.kt")
        val route = activity.substringAfter("private fun openSocialAiVoiceCall()")
            .substringBefore("private fun suspendSocialChatForProjectReturn()")

        assertTrue(route.contains("socialAiChatFeature.startDefaultRealtimeVoice()"))
        assertFalse(route.contains("ServerApi"))
        assertFalse(route.contains("NativeApi"))
        assertFalse(route.contains("SocialAiVoiceCallActivity.createIntent"))
    }

    @Test
    fun managedNativePeerKeepsCurrentAndRemoteCreatedDataChannels() {
        val peer = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/" +
                "ChatGptWebNativeVoicePeer.kt",
        )
        val relay = read("android/app/src/main/assets/chatgpt_web_private_voice_relay.js")

        assertTrue(relay.contains("label: ''"))
        assertFalse(relay.contains("label: 'oai-events'"))
        assertTrue(peer.contains("override fun onDataChannel(channel: DataChannel)"))
        assertTrue(peer.contains("bindDataChannel(token, channel)"))
        assertTrue(peer.contains("IdentityHashMap<DataChannel, DataChannel.Observer>()"))
        assertTrue(peer.contains("dataChannels.size >= MAX_DATA_CHANNELS"))
        assertTrue(peer.contains("hasDataChannel(observedChannel)"))
        assertTrue(peer.contains("dataChannels.keys.toList()"))
        assertFalse(peer.contains("currentChannel?.close()"))
    }

    @Test
    fun productionFeatureDelegatesToTheCurrentProviderToolCoordinator() {
        val feature = read("android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt")
        val transports = read("android/app/src/main/kotlin/com/elon/app/MainRealtimeVoiceTransports.kt")
        val factory = read("android/app/src/main/kotlin/com/elon/app/MainRealtimeVoiceCoordinatorFactory.kt")
        val tools = read("android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerTools.kt")
        val commands = read("android/app/src/main/kotlin/com/elon/app/WebChatProductionComposerCommands.kt")
        val controls = read("android/app/src/main/kotlin/com/elon/app/WebChatProductionVoiceControls.kt")
        val speech = read("android/app/src/main/kotlin/com/elon/app/MainSpeechInputActions.kt")
        val dictation = read("android/app/src/main/kotlin/com/elon/app/MainWebChatDictationActions.kt")
        val messageActions = read(
            "android/app/src/main/kotlin/com/elon/app/WebChatProductionMessageActions.kt",
        )
        val composer = read("android/app/src/main/kotlin/com/elon/app/MainInputComposerSetup.kt")
        val sendVisual = read("android/app/src/main/kotlin/com/elon/app/MainSendButtonVisualActions.kt")
        val pageAdapter = read("android/app/src/main/assets/chatgpt_web_adapter.js")
        val directRoute = tools.substringAfter("fun startRealtimeVoice")
            .substringBefore("fun executeCommand")
        val commandRoute = tools.substringAfter("fun executeCommand")
            .substringBefore("fun cancelPending")

        assertTrue(feature.contains("fun startDefaultRealtimeVoice(): Boolean"))
        assertTrue(feature.contains("providerSwitchCoordinator.selectWithoutPrompt(WebChatProviderId.CHATGPT_WEB)"))
        assertFalse(feature.contains("selectChatProvider(WebChatProviderId.CHATGPT_WEB)"))
        assertTrue(feature.contains("productionComposerTools.startRealtimeVoice"))
        assertTrue(feature.contains("productionVoiceControls.render"))
        assertTrue(feature.contains("productionVoiceControls.restoreLocalVoiceInput"))
        assertTrue(commands.contains("WebChatProviderCapability.REALTIME_VOICE"))
        assertTrue(tools.contains("chatgpt_start_realtime_voice"))
        assertTrue(tools.contains("startWebRealtimeVoice"))
        assertFalse(tools.contains("startNativeApiRealtimeVoice"))
        assertFalse(tools.contains("原生实时 AI"))
        assertFalse(tools.contains("ChatGptWebTestActivity"))
        assertTrue(directRoute.contains("executeCommand(provider, command)"))
        assertTrue(commandRoute.contains("command.action == REALTIME_VOICE_ACTION"))
        assertTrue(commandRoute.contains("startWebRealtimeVoice(provider)"))
        assertFalse(commandRoute.contains("openOfficialRealtimeVoice()"))
        assertTrue(transports.contains("startDefaultOfficialWebRtc"))
        assertTrue(transports.contains("officialWebRtc.runtimeEnabled"))
        assertTrue(transports.contains("serverApiExperiment.runtimeEnabled"))
        assertTrue(transports.contains("WebChatRealtimeVoiceOverlay"))
        assertTrue(factory.contains("createWebChatRealtimeVoiceCoordinator"))
        assertEquals(
            "provider changes must hand voice control between the native and system overlays",
            2,
            "realtimeVoices.onActiveSurfaceChanged()".toRegex().findAll(feature).count(),
        )
        assertTrue(transports.contains("modeController.openOfficialRealtimeVoice()"))
        assertTrue(factory.contains("beginRealtimeVoiceBacking"))
        assertFalse(commandRoute.contains("mcpPort()"))
        assertTrue(tools.contains("port.executeSessionCommand(pending.command.action)"))
        assertTrue(tools.contains("pendingSessionCommand"))
        assertTrue(controls.contains("REALTIME_VOICE_BLUE"))
        assertTrue(controls.contains("views.webDictationButton"))
        assertTrue(controls.contains("WebChatDictationModeSelector"))
        assertTrue(controls.contains("setOnLongClickListener"))
        assertTrue(controls.contains("web_chat_dictation_mode_changed"))
        assertTrue(controls.contains("WebChatDictationRearmGate"))
        assertTrue(controls.contains("prepareDictationCapture()"))
        assertTrue(controls.contains("web_chat_dictation_tap"))
        assertTrue(controls.contains("privateDictation.start"))
        assertTrue(controls.contains("sharedDictation.start"))
        assertFalse(controls.contains("startSharedThenDom"))
        assertFalse(controls.contains("startDomDictation"))
        assertFalse(controls.contains("web_chat_dictation_dom_fallback"))
        assertTrue(controls.contains("onUnavailableBeforeCapture = { false }"))
        assertTrue(controls.contains("onDomCommandResult"))
        assertFalse(controls.contains("providerDictationReady"))
        assertFalse(controls.contains("startProviderDictation"))
        assertTrue(controls.contains("ic_web_chat_dictation_cancel"))
        assertTrue(controls.contains("ic_web_chat_dictation_done"))
        assertTrue(speech.contains("sharedAgentVoiceBridge"))
        assertTrue(dictation.contains("WebChatNativeDictationSession"))
        assertTrue(dictation.contains("AgentVoiceDictationEngine(bridge())"))
        assertTrue(dictation.contains("FALLBACK_COOLDOWN_MS"))
        assertTrue(dictation.contains("onUnavailableBeforeCapture"))
        assertTrue(dictation.contains("fallback_accepted"))
        assertFalse(dictation.contains("再点一次可使用官网听写"))
        assertFalse(dictation.contains("WebChatResilientDictationEngine"))
        assertTrue(messageActions.contains("WebChatNativeReadAloudController"))
        assertTrue(messageActions.contains("observed.filterNot { it.semantic == READ_ALOUD_SEMANTIC }"))
        assertTrue(composer.contains("inputBarContainer.addView(webDictationButton)"))
        assertTrue(composer.contains("LinearLayout.LayoutParams(dp(48), dp(48))"))
        assertTrue(sendVisual.contains("button.isEnabled = button.isEnabled &&"))
        assertTrue(pageAdapter.contains("setComposerValue: setComposerValueWithoutFocus"))
        val silentComposerUpdate = pageAdapter.substringAfter("function setComposerValueWithoutFocus")
            .substringBefore("function result")
        assertFalse(silentComposerUpdate.contains("composer.focus()"))
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
        assertEquals("chatgpt_cancel_dictation", active.dictationCancel?.action)
        assertNull(active.realtimeVoice)

        val google = WebChatProductionVoicePresentationPolicy.resolve(
            provider = WebChatProviderRegistry.get(WebChatProviderId.GOOGLE_WEB),
            streaming = false,
            dictationActive = false,
        )
        assertNull(google.dictation)
        assertNull(google.dictationCancel)
        assertNull(google.realtimeVoice)
    }

    @Test
    fun productionDictationDoesNotMislabelOfficialDomAsPrivateTransport() {
        assertEquals(
            WebChatProductionDictationTapRoute.START,
            WebChatProductionDictationRoutePolicy.resolve(
                privateActive = false,
                sharedActive = false,
                domActive = false,
                startAvailable = true,
            ),
        )
        assertEquals(
            WebChatProductionDictationTapRoute.SUBMIT_DOM,
            WebChatProductionDictationRoutePolicy.resolve(
                privateActive = false,
                sharedActive = false,
                domActive = true,
                startAvailable = true,
            ),
        )
        assertEquals(
            WebChatProductionDictationTapRoute.SUBMIT_PRIVATE,
            WebChatProductionDictationRoutePolicy.resolve(
                privateActive = true,
                sharedActive = false,
                domActive = true,
                startAvailable = true,
            ),
        )
    }

    @Test
    fun officialDomDictationActivatesOnlyTheRedactedResearchObserver() {
        val actions = read("android/app/src/main/assets/chatgpt_web_adapter_dictation_actions.js")

        val start = actions.substringAfter("function start(composer")
            .substringBefore("function finish(kind")
        assertTrue(start.contains("__elonChatGptRealtimeVoiceResearch"))
        assertTrue(start.contains("research.activate()"))
        assertFalse(start.contains("authorization"))
        assertFalse(start.contains("cookie"))
    }

    @Test
    fun bufferedPrivateDictationCannotReintroduceRealtimeVoiceReuse() {
        val controller = read(
            "android/app/src/main/kotlin/com/elon/app/ChatGptSocialChatController.kt",
        )
        val session = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )
        val build = read("android/app/build.gradle")

        assertTrue(controller.contains("ChatGptWebPrivateDictationTransport("))
        assertFalse(controller.contains("WebChatPrivateDictationSessionPort("))
        assertFalse(session.contains("ChatGptWebPrivateDictationHost("))
        assertTrue(build.contains("ELON_CHATGPT_PRIVATE_DICTATION"))
        assertTrue(build.contains("CHATGPT_PRIVATE_DICTATION_ENABLED"))
        assertFalse(build.contains("ELON_CHATGPT_PRIVATE_DICTATION_NATIVE_RTC"))
        assertFalse(build.contains("CHATGPT_PRIVATE_DICTATION_NATIVE_RTC_ENABLED"))
    }

    @Test
    fun officialRealtimeVoiceUsesNativeUiAndPreservesTheBackgroundWebView() {
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
        val feature = read(
            "android/app/src/main/kotlin/com/elon/app/MainSocialAiChatFeature.kt",
        )
        val factory = read(
            "android/app/src/main/kotlin/com/elon/app/MainRealtimeVoiceCoordinatorFactory.kt",
        )
        val session = read(
            "android/app/src/main/kotlin/com/elon/app/chatgptweb/ChatGptBackgroundSession.kt",
        )

        assertTrue(surface.contains("\"语音 AI · ${'$'}{visibleState.label}\""))
        assertTrue(surface.contains("\"记录到：${'$'}{it.label}\""))
        assertTrue(surface.contains("WebChatRealtimeVoiceOverlayHost.resolve(activity)"))
        assertFalse(surface.contains("text = \"ChatGPT 网页 AI\""))
        assertTrue(surface.contains("setBackgroundColor(Color.TRANSPARENT)"))
        assertTrue(surface.contains("isClickable = false"))
        assertFalse(surface.contains("panel.setOnTouchListener"))
        assertTrue(surface.contains("installDragInteraction(collapsedOrb)"))
        assertTrue(surface.contains("installDragInteraction(expandedIcon)"))
        assertFalse(surface.contains("root.requestFocus()"))
        assertFalse(backing.contains("if (!gracefulExit) view.reload()"))
        assertTrue(backing.contains("requestConversationSnapshot()"))
        assertTrue(backing.contains("requestPrivateConversationSnapshot()"))
        assertTrue(backing.contains("scheduleActiveTranscriptRefresh"))
        assertTrue(backing.contains("nativeVoiceState.transcriptEventCount > 0"))
        assertTrue(backing.contains("beginWebChatRealtimeVoiceInteraction()"))
        assertTrue(backing.contains("if (gracefulExit) return"))
        assertTrue(backing.contains("conversationRecoveredSince(recoveryToken.snapshotRevision)"))
        assertTrue(backing.contains("recoveryGate.shouldReload"))
        assertFalse(backing.contains("view.stopLoading()"))
        assertTrue(controller.contains("WebChatRealtimeVoiceTranscriptContinuity()"))
        assertTrue(controller.contains("realtimeVoiceTranscript.begin(session.currentSnapshot())"))
        assertTrue(controller.contains("realtimeVoiceTranscript.end(session.currentSnapshot())"))
        assertTrue(controller.contains("if (!session.realtimeVoiceActive())"))
        assertFalse(controller.contains("renderStatusMessage(\"正在同步语音会话…\")"))
        val deactivate = feature.substringAfter("private fun deactivateChatProvider")
            .substringBefore("private fun activateChatProvider")
        assertFalse(deactivate.contains("realtimeVoice.close()"))
        assertTrue(factory.contains("resolveConversationContext = { resolveRealtimeVoiceContext"))
        assertTrue(factory.contains("openRealtimeVoiceConversation(it"))
        assertTrue(session.contains("if (realtimeVoiceBacking.isActive())"))
        assertTrue(session.contains("fun onHostPaused() = if (realtimeVoiceBacking.isActive()) cookieManager.flush()"))
        assertTrue(session.contains("pageAdapter?.requestConversationRefresh()"))
    }

    @Test
    fun productionVoiceGatesOnlyConfirmedGuestsAndDelegatesCredentialsToTheOfficialPage() {
        val factory = read("android/app/src/main/kotlin/com/elon/app/MainRealtimeVoiceCoordinatorFactory.kt")
        val coordinator = read("android/app/src/main/kotlin/com/elon/app/WebChatRealtimeVoiceCoordinator.kt")
        val gate = read("android/app/src/main/kotlin/com/elon/app/WebChatRealtimeVoiceLoginGate.kt")
        val mode = read("android/app/src/main/kotlin/com/elon/app/SocialAiChatModeController.kt")
        val strings = read("android/app/src/main/res/values/strings.xml")

        assertTrue(factory.contains("authenticated = controller::authenticated"))
        assertTrue(factory.contains("sessionState = controller::stateWireValue"))
        assertTrue(factory.contains("openOfficialLogin = modeController::openOfficialLogin"))
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
