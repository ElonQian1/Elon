package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceActivationGateTest {
    @Test
    fun commandSuccessAloneDoesNotClaimVoiceIsActive() {
        val gate = WebChatRealtimeVoiceActivationGate(maxPolls = 2)
        gate.begin(evidence(revision = 4))

        assertTrue(gate.observe(evidence(revision = 4), attempt = 0) is
            WebChatRealtimeVoiceActivationDecision.Wait)
        assertTrue(gate.observe(evidence(revision = 4), attempt = 2) is
            WebChatRealtimeVoiceActivationDecision.Failed)
    }

    @Test
    fun aFreshWebMicrophoneGrantActivatesTheVoiceSurface() {
        val gate = WebChatRealtimeVoiceActivationGate()
        gate.begin(evidence(revision = 4))

        assertEquals(
            WebChatRealtimeVoiceActivationDecision.Active,
            gate.observe(evidence(revision = 5), attempt = 1),
        )
    }

    @Test
    fun anOfficialActiveVoiceControlReusesAnExistingMicrophoneGrant() {
        val gate = WebChatRealtimeVoiceActivationGate()
        gate.begin(evidence(revision = 4))

        assertEquals(
            WebChatRealtimeVoiceActivationDecision.Active,
            gate.observe(
                evidence(revision = 4, officialVoiceActive = true),
                attempt = 1,
            ),
        )
    }

    @Test
    fun deniedAndroidPermissionFailsImmediately() {
        val gate = WebChatRealtimeVoiceActivationGate()
        gate.begin(evidence(revision = 0))

        assertTrue(
            gate.observe(
                evidence(revision = 0, permissionGranted = false),
                attempt = 0,
            ) is WebChatRealtimeVoiceActivationDecision.Failed,
        )
    }

    private fun evidence(
        revision: Long,
        permissionGranted: Boolean = true,
        officialVoiceActive: Boolean = false,
    ) = WebChatRealtimeVoiceActivationEvidence(
        androidPermissionGranted = permissionGranted,
        webPermissionGrantRevision = revision,
        webRequestPending = false,
        requestState = "idle",
        officialVoiceActive = officialVoiceActive,
    )
}
