package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatRealtimeVoiceDelayedCloseReconcilerTest {
    @Test
    fun completesOnlyAfterStableConversationEvidence() {
        val reconciler = WebChatRealtimeVoiceDelayedCloseReconciler(
            watchdogDelaysMs = longArrayOf(100L, 200L),
            stableConversationPolls = 2,
            stableConversationMs = 100L,
            controlRefreshInterval = 2,
        )
        reconciler.begin()

        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Wait(refreshControls = false),
            reconciler.observeEvent(state(endControlAvailable = false), observedAtMs = 1_000L),
        )
        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Complete,
            reconciler.observeEvent(state(endControlAvailable = false), observedAtMs = 1_100L),
        )
    }

    @Test
    fun rapidDuplicateEventsCannotConfirmAnOfficialHangup() {
        val reconciler = WebChatRealtimeVoiceDelayedCloseReconciler(
            watchdogDelaysMs = longArrayOf(100L, 200L),
            stableConversationPolls = 2,
            stableConversationMs = 100L,
            controlRefreshInterval = 2,
        )
        reconciler.begin()

        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Wait(refreshControls = false),
            reconciler.observeEvent(state(endControlAvailable = false), observedAtMs = 1_000L),
        )
        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Wait(refreshControls = false),
            reconciler.observeEvent(state(endControlAvailable = false), observedAtMs = 1_001L),
        )
        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Complete,
            reconciler.observeEvent(state(endControlAvailable = false), observedAtMs = 1_100L),
        )
    }

    @Test
    fun eventObservationsDoNotConsumeTheSparseWatchdogBudget() {
        val reconciler = WebChatRealtimeVoiceDelayedCloseReconciler(
            watchdogDelaysMs = longArrayOf(100L, 200L),
            stableConversationPolls = 2,
            stableConversationMs = 100L,
            controlRefreshInterval = 2,
        )
        reconciler.begin()

        assertEquals(100L, reconciler.nextWatchdogDelayMs())
        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Wait(refreshControls = false),
            reconciler.observeEvent(state(endControlAvailable = true), observedAtMs = 1_000L),
        )
        assertEquals(100L, reconciler.nextWatchdogDelayMs())
    }

    @Test
    fun activeVoiceEvidenceNeverCompletesAndEventuallyExpires() {
        val reconciler = WebChatRealtimeVoiceDelayedCloseReconciler(
            watchdogDelaysMs = longArrayOf(100L, 200L),
            stableConversationPolls = 2,
            stableConversationMs = 100L,
            controlRefreshInterval = 2,
        )
        reconciler.begin()

        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Wait(refreshControls = true),
            reconciler.observeWatchdog(state(endControlAvailable = true), observedAtMs = 1_000L),
        )
        assertEquals(200L, reconciler.nextWatchdogDelayMs())
        assertEquals(
            WebChatRealtimeVoiceDelayedCloseDecision.Expired,
            reconciler.observeWatchdog(state(endControlAvailable = true), observedAtMs = 1_200L),
        )
        assertEquals(null, reconciler.nextWatchdogDelayMs())
    }

    private fun state(endControlAvailable: Boolean) = WebChatConsumerState(
        streaming = false,
        dictationActive = false,
        composerSections = emptyMap(),
        pageKind = "conversation",
        pageUrl = "https://chatgpt.com/c/test",
        features = emptyList(),
        controls = if (endControlAvailable) {
            listOf(
                WebChatConsumerControlDescriptor(
                    control = VoiceEndControl,
                    requiresUserConfirmation = true,
                    presentation = WebChatConsumerControlPresentation.DIRECT,
                    nativeSelector = "voice_close",
                ),
            )
        } else {
            emptyList()
        },
        commandRequests = emptyList(),
    )

    private object VoiceEndControl : WebChatConsumerControl {
        override val id = "voice_end"
        override val semantic = "close"
        override val label = "结束语音"
        override val region = "voice"
        override val role = "button"
        override val enabled = true
        override val selected = false
        override val inputKind: String? = null
        override val writable = false
        override val stateSettable = false
        override val choiceLabels = emptyList<String>()
        override val selectedChoiceIndex: Int? = null
        override val slider: WebChatConsumerSlider? = null
        override val expanded: Boolean? = null
        override val expandable = false
        override val contextId: String? = null
        override val inViewport = true
        override val webXRatio: Double? = null
        override val webYRatio: Double? = null
    }
}
