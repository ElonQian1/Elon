package com.elon.app.chatgptweb

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatGptWebOfficialStartupCoordinatorTest {
    @Test
    fun waitsThroughAuthenticationAndDelayedDomDiscovery() {
        val harness = Harness()

        harness.coordinator.onHostResumed()
        harness.coordinator.onPageReady(enhancedModeSupported = false)

        assertTrue(harness.feedback.isEmpty())
        assertEquals(0, harness.manifestRequests)

        harness.coordinator.onPageReady(enhancedModeSupported = true)
        harness.emitManifest(emptyList())
        harness.emitManifest(listOf(voiceControl()))

        assertEquals(ChatGptWebOfficialStartupFeedback.CONNECTING, harness.feedback.first())
        assertTrue(harness.manifestRequests >= 1)
        assertEquals(1, harness.microphoneRequests)
        assertFalse(harness.feedback.contains(ChatGptWebOfficialStartupFeedback.UNAVAILABLE))
    }

    @Test
    fun refreshesTheControlAfterPermissionAndHostResumeBeforeInvoking() {
        val harness = Harness()
        harness.coordinator.onHostResumed()
        harness.coordinator.onPageReady(enhancedModeSupported = true)
        harness.emitManifest(listOf(voiceControl("voice-before-permission")))

        harness.coordinator.onHostPaused()
        harness.grantMicrophone()
        assertTrue(harness.invocations.isEmpty())

        harness.coordinator.onHostResumed()
        harness.emitManifest(listOf(voiceControl("voice-after-resume")))

        assertEquals(listOf("voice-after-resume" to "voice-request"), harness.invocations)
        assertTrue(harness.coordinator.requestConsumed())
    }

    @Test
    fun reportsStartedOnlyForTheMatchingSuccessfulCommand() {
        val harness = Harness()
        harness.startAndGrant()

        harness.coordinator.onEvent(commandResult(requestId = "other", ok = true))
        assertFalse(harness.feedback.contains(ChatGptWebOfficialStartupFeedback.STARTED))

        harness.coordinator.onEvent(commandResult(requestId = "voice-request", ok = true))
        assertEquals(ChatGptWebOfficialStartupFeedback.STARTED, harness.feedback.last())
    }

    @Test
    fun reportsUnavailableOnlyAfterTheActiveDiscoveryTimeout() {
        val harness = Harness()
        harness.coordinator.onHostResumed()
        harness.coordinator.onPageReady(enhancedModeSupported = true)

        assertFalse(harness.feedback.contains(ChatGptWebOfficialStartupFeedback.UNAVAILABLE))
        harness.runScheduled(delayMs = 12_000L)

        assertEquals(ChatGptWebOfficialStartupFeedback.UNAVAILABLE, harness.feedback.last())
    }

    @Test
    fun reportsMicrophoneDenialWithoutClaimingTheFeatureIsUnavailable() {
        val harness = Harness()
        harness.coordinator.onHostResumed()
        harness.coordinator.onPageReady(enhancedModeSupported = true)
        harness.emitManifest(listOf(voiceControl()))
        harness.denyMicrophone()

        assertEquals(ChatGptWebOfficialStartupFeedback.MICROPHONE_DENIED, harness.feedback.last())
        assertFalse(harness.feedback.contains(ChatGptWebOfficialStartupFeedback.UNAVAILABLE))
    }

    @Test
    fun disposeMakesPendingTimeoutsInert() {
        val harness = Harness()
        harness.coordinator.onHostResumed()
        harness.coordinator.onPageReady(enhancedModeSupported = true)
        harness.coordinator.dispose()

        harness.runScheduled(delayMs = 12_000L)

        assertFalse(harness.feedback.contains(ChatGptWebOfficialStartupFeedback.UNAVAILABLE))
    }

    private class Harness {
        var manifestRequests = 0
        var microphoneRequests = 0
        val feedback = mutableListOf<ChatGptWebOfficialStartupFeedback>()
        val invocations = mutableListOf<Pair<String, String>>()
        val scheduled = mutableListOf<Pair<Long, () -> Unit>>()
        private var grant: (() -> Unit)? = null
        private var deny: (() -> Unit)? = null
        val coordinator = ChatGptWebOfficialStartupCoordinator(
            action = ChatGptWebOfficialStartupAction.REALTIME_VOICE,
            requestManifest = { manifestRequests += 1 },
            requestMicrophone = { onGranted, onDenied ->
                microphoneRequests += 1
                grant = onGranted
                deny = onDenied
            },
            invokeControl = { controlId, requestId ->
                invocations += controlId to requestId
            },
            schedule = { delayMs, action -> scheduled += delayMs to action },
            requestId = { "voice-request" },
            onFeedback = feedback::add,
        )

        fun emitManifest(controls: List<ChatGptWebUiControl>) {
            coordinator.onEvent(ChatGptWebEvent.UiManifest(manifest(controls)))
        }

        fun grantMicrophone() = checkNotNull(grant).invoke()

        fun denyMicrophone() = checkNotNull(deny).invoke()

        fun startAndGrant() {
            coordinator.onHostResumed()
            coordinator.onPageReady(enhancedModeSupported = true)
            emitManifest(listOf(voiceControl()))
            grantMicrophone()
            emitManifest(listOf(voiceControl()))
        }

        fun runScheduled(delayMs: Long) {
            checkNotNull(scheduled.firstOrNull { it.first == delayMs }).second.invoke()
        }
    }

    private companion object {
        fun manifest(controls: List<ChatGptWebUiControl>) = ChatGptWebUiManifest(
            version = 3,
            pageKind = "conversation",
            title = "ChatGPT",
            compatibility = "healthy",
            controls = controls,
        )

        fun voiceControl(id: String = "voice-control") = ChatGptWebUiControl(
            id = id,
            semantic = ChatGptRealtimeVoicePolicy.SEMANTIC,
            label = "实时语音",
            region = ChatGptWebUiRegion.COMPOSER,
            role = "button",
            enabled = true,
            selected = false,
        )

        fun commandResult(requestId: String, ok: Boolean) = ChatGptWebEvent.CommandResult(
            action = "invoke_ui_control",
            ok = ok,
            detail = if (ok) "clicked" else "failed",
            requestId = requestId,
        )
    }
}
