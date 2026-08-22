package com.elon.app

import com.elon.app.chatgptweb.ChatGptWebUiControl
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceLaunchCacheTest {
    @Test
    fun currentVoiceControlStartsDirectlyWithoutAStoredHint() {
        val cache = WebChatRealtimeVoiceLaunchCache()

        assertEquals(
            WebChatRealtimeVoiceLaunchPlan.DIRECT,
            cache.plan(WebChatProviderId.CHATGPT_WEB, state("one", voice = true), sessionReady = true),
        )
    }

    @Test
    fun restoredConversationHintRefreshesControlsWithoutRecoveringTheWholeSession() {
        val storage = MemoryStorage()
        val first = WebChatRealtimeVoiceLaunchCache(storage, nowMs = { 1_000L })
        first.observe(WebChatProviderId.CHATGPT_WEB, state("one", voice = true))

        val restored = WebChatRealtimeVoiceLaunchCache(storage, nowMs = { 2_000L })

        assertEquals(
            WebChatRealtimeVoiceLaunchPlan.REFRESH_CONTROLS,
            restored.plan(
                WebChatProviderId.CHATGPT_WEB,
                state("one", voice = false),
                sessionReady = true,
            ),
        )
    }

    @Test
    fun hintsNeverCrossConversationOrProviderBoundaries() {
        val cache = WebChatRealtimeVoiceLaunchCache(nowMs = { 1_000L })
        cache.observe(WebChatProviderId.CHATGPT_WEB, state("one", voice = true))

        assertEquals(
            WebChatRealtimeVoiceLaunchPlan.RECOVER_SESSION,
            cache.plan(WebChatProviderId.CHATGPT_WEB, state("two"), sessionReady = true),
        )
        assertEquals(
            WebChatRealtimeVoiceLaunchPlan.RECOVER_SESSION,
            cache.plan(WebChatProviderId.GOOGLE_WEB, state("one"), sessionReady = true),
        )
    }

    @Test
    fun staleAdapterOrSessionAlwaysUsesRecovery() {
        val cache = WebChatRealtimeVoiceLaunchCache()

        assertEquals(
            WebChatRealtimeVoiceLaunchPlan.RECOVER_SESSION,
            cache.plan(
                WebChatProviderId.CHATGPT_WEB,
                state("one", voice = true).copy(adapterCurrent = false),
                sessionReady = true,
            ),
        )
        assertEquals(
            WebChatRealtimeVoiceLaunchPlan.RECOVER_SESSION,
            cache.plan(WebChatProviderId.CHATGPT_WEB, state("one", voice = true), false),
        )
    }

    @Test
    fun persistedPayloadContainsOnlyHashedConversationIdentity() {
        val snapshot = WebChatRealtimeVoiceLaunchSnapshot(listOf(
            WebChatRealtimeVoiceLaunchEntry(
                providerId = WebChatProviderId.CHATGPT_WEB,
                conversationHash = "a".repeat(64),
                updatedAtMs = 1_000L,
            ),
        ))

        val encoded = WebChatRealtimeVoiceLaunchSnapshotCodec.encode(snapshot)
        val decoded = WebChatRealtimeVoiceLaunchSnapshotCodec.decode(encoded)

        assertEquals(snapshot, decoded)
        assertFalse(encoded.contains("/c/"))
        assertFalse(encoded.contains("token"))
        assertTrue(encoded.contains("a".repeat(64)))
    }

    private fun state(conversationId: String, voice: Boolean = false) = WebChatConsumerState(
        streaming = false,
        dictationActive = false,
        composerSections = emptyMap(),
        pageKind = "conversation",
        pageUrl = "https://chatgpt.com/c/$conversationId",
        features = emptyList(),
        controls = if (voice) listOf(voiceControl()) else emptyList(),
        commandRequests = emptyList(),
    )

    private fun voiceControl() = WebChatConsumerControlDescriptor(
        control = ChatGptWebUiControl(
            id = "voice",
            label = "实时语音",
            semantic = "voice_mode",
            region = "composer",
            role = "button",
            enabled = true,
            selected = false,
        ),
        requiresUserConfirmation = false,
        presentation = WebChatConsumerControlPresentation.DIRECT,
        nativeSelector = "voice",
    )

    private class MemoryStorage : WebChatRealtimeVoiceLaunchStorage {
        private var snapshot: WebChatRealtimeVoiceLaunchSnapshot? = null
        override fun restore(): WebChatRealtimeVoiceLaunchSnapshot? = snapshot
        override fun save(snapshot: WebChatRealtimeVoiceLaunchSnapshot) {
            this.snapshot = snapshot
        }
    }
}
