package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Test

class WebChatRealtimeVoiceLoginGateTest {
    @Test
    fun distinguishesAConfirmedGuestFromAStillLoadingSession() {
        assertEquals(
            WebChatRealtimeVoiceAuthenticationState.UNKNOWN,
            WebChatRealtimeVoiceAuthenticationPolicy.resolve(false, "loading"),
        )
        assertEquals(
            WebChatRealtimeVoiceAuthenticationState.GUEST,
            WebChatRealtimeVoiceAuthenticationPolicy.resolve(false, "ready"),
        )
        assertEquals(
            WebChatRealtimeVoiceAuthenticationState.GUEST,
            WebChatRealtimeVoiceAuthenticationPolicy.resolve(false, "login_required"),
        )
        assertEquals(
            WebChatRealtimeVoiceAuthenticationState.AUTHENTICATED,
            WebChatRealtimeVoiceAuthenticationPolicy.resolve(true, "loading"),
        )
    }

    @Test
    fun presentsOfficialAccountChoicesWithoutCollectingCredentials() {
        assertEquals(
            listOf("Google 账号", "Apple 账号", "电话号码", "电子邮箱"),
            WebChatRealtimeVoiceLoginPresentation.DEFAULT.methods.map { it.label },
        )
    }
}
