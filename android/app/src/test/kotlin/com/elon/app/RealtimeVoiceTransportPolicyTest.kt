package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RealtimeVoiceTransportPolicyTest {
    @Test
    fun officialWebRtcIsTheConsumerDefaultForTheCurrentProviderConversation() {
        val transport = RealtimeVoiceTransportCatalog.officialWebRtc

        assertTrue(RealtimeVoiceTransportPolicy.canUseCurrentProviderConversation(transport))
        assertEquals(
            RealtimeVoiceConversationScope.CURRENT_PROVIDER_CONVERSATION,
            transport.scope,
        )
        assertEquals("persistent_background_webview", transport.identityLayer)
        assertEquals("official_webrtc", transport.mediaTransport)
        assertEquals("native_ui_overlay", transport.presentationLayer)
        assertTrue(transport.consumerDefault)
        assertTrue(transport.runtimeEnabled)
        assertTrue(transport.userVisible)
    }

    @Test
    fun serverApiExperimentHasNoConsumerEntry() {
        val transport = RealtimeVoiceTransportCatalog.serverApiExperiment
        val context = RealtimeVoiceTransportPolicy.contextFor(transport)

        assertFalse(RealtimeVoiceTransportPolicy.canUseCurrentProviderConversation(transport))
        assertEquals(RealtimeVoiceConversationScope.NEW_LOCAL_CONVERSATION, transport.scope)
        assertEquals("一龙 AI 新会话", context.label)
        assertTrue(context.savedToHistory)
        assertTrue(context.openable)
        assertFalse(transport.consumerDefault)
        assertFalse(transport.runtimeEnabled)
        assertFalse(transport.userVisible)
    }

    @Test
    fun nativeAndWebCapabilitiesHaveStableDistinctIds() {
        assertEquals(
            "android_openai_native_realtime_voice_v1",
            RealtimeVoiceTransportCatalog.serverApiExperiment.capabilityId,
        )
        assertTrue(
            RealtimeVoiceTransportCatalog.serverApiExperiment.capabilityId !=
                RealtimeVoiceTransportCatalog.officialWebRtc.capabilityId,
        )
    }

    @Test
    fun runtimeInventoryExposesTheExplicitConversationScopes() {
        val rows = RealtimeVoiceTransportCatalog.describe()

        assertEquals(2, rows.length())
        assertEquals(
            "current_provider_conversation",
            rows.getJSONObject(0).getString("conversation_scope"),
        )
        assertEquals(
            "new_local_conversation",
            rows.getJSONObject(1).getString("conversation_scope"),
        )
        assertTrue(rows.getJSONObject(0).getBoolean("consumer_default"))
        assertEquals("official_webrtc", rows.getJSONObject(0).getString("media_transport"))
        assertFalse(rows.getJSONObject(1).getBoolean("runtime_enabled"))
        assertFalse(rows.getJSONObject(1).getBoolean("user_visible"))
    }
}
