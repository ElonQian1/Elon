package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class RealtimeVoiceTransportPolicyTest {
    @Test
    fun webAccountTransportOwnsOnlyTheCurrentProviderConversation() {
        val transport = RealtimeVoiceTransportCatalog.webAccount

        assertTrue(RealtimeVoiceTransportPolicy.canUseCurrentProviderConversation(transport))
        assertEquals(
            RealtimeVoiceConversationScope.CURRENT_PROVIDER_CONVERSATION,
            transport.scope,
        )
    }

    @Test
    fun nativeApiTransportCreatesAnExplicitLocalConversation() {
        val transport = RealtimeVoiceTransportCatalog.nativeApi
        val context = RealtimeVoiceTransportPolicy.contextFor(transport)

        assertFalse(RealtimeVoiceTransportPolicy.canUseCurrentProviderConversation(transport))
        assertEquals(RealtimeVoiceConversationScope.NEW_LOCAL_CONVERSATION, transport.scope)
        assertEquals("一龙 AI 新会话", context.label)
        assertTrue(context.savedToHistory)
        assertTrue(context.openable)
    }

    @Test
    fun nativeAndWebCapabilitiesHaveStableDistinctIds() {
        assertEquals(
            "android_openai_native_realtime_voice_v1",
            RealtimeVoiceTransportCatalog.nativeApi.capabilityId,
        )
        assertTrue(
            RealtimeVoiceTransportCatalog.nativeApi.capabilityId !=
                RealtimeVoiceTransportCatalog.webAccount.capabilityId,
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
    }
}
