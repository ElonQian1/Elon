package com.elon.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class WebChatRealtimeVoiceLayoutPolicyTest {
    @Test
    fun phoneLayoutKeepsTheCollapsedControlSmallAndExpandedPanelOnScreen() {
        val metrics = WebChatRealtimeVoiceFloatingLayoutPolicy.resolve(
            widthPx = 1_080,
            density = 3f,
        )

        assertEquals(192, metrics.collapsedSize)
        assertEquals(48, metrics.edgeInset)
        assertTrue(metrics.expandedWidth <= 1_080 - metrics.edgeInset * 2)
    }

    @Test
    fun narrowHostStillProvidesAUsableExpandedPanel() {
        val metrics = WebChatRealtimeVoiceFloatingLayoutPolicy.resolve(
            widthPx = 720,
            density = 3f,
        )

        assertEquals(624, metrics.expandedWidth)
    }

    @Test
    fun dragPositionIsClampedInsideEveryHostEdge() {
        val position = WebChatRealtimeVoiceFloatingLayoutPolicy.clamp(
            desiredLeft = 2_000f,
            desiredTop = -500f,
            hostWidth = 1_080,
            hostHeight = 2_400,
            panelWidth = 300,
            panelHeight = 240,
            edgeInset = 48,
        )

        assertEquals(732f, position.left)
        assertEquals(48f, position.top)
    }

    @Test
    fun initialPositionStartsAtTheRightWithoutCoveringTheComposer() {
        val position = WebChatRealtimeVoiceFloatingLayoutPolicy.initialPosition(
            hostWidth = 1_080,
            hostHeight = 2_400,
            panelWidth = 192,
            panelHeight = 192,
            edgeInset = 48,
        )

        assertEquals(840f, position.left)
        assertTrue(position.top in 1_100f..1_400f)
    }
}
