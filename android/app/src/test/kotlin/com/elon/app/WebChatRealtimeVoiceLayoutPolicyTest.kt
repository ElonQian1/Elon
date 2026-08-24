package com.elon.app

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceLayoutPolicyTest {
    @Test
    fun portraitKeepsTheFullVoiceLayout() {
        val metrics = WebChatRealtimeVoiceLayoutPolicy.resolve(
            widthPx = 1080,
            heightPx = 2400,
            density = 3f,
        )

        assertFalse(metrics.compact)
        assertTrue(metrics.orbSize >= 600)
    }

    @Test
    fun landscapeShrinksTheOrbSoTheHangupControlRemainsVisible() {
        val metrics = WebChatRealtimeVoiceLayoutPolicy.resolve(
            widthPx = 2400,
            heightPx = 1080,
            density = 3f,
        )

        assertTrue(metrics.compact)
        assertTrue(metrics.orbSize <= 360)
        assertTrue(metrics.closeSize <= 160)
    }
}
