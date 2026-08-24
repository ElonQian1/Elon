package com.elon.app

internal data class WebChatRealtimeVoiceFloatingMetrics(
    val edgeInset: Int,
    val collapsedSize: Int,
    val expandedWidth: Int,
)

internal data class WebChatRealtimeVoiceFloatingPosition(
    val left: Float,
    val top: Float,
)

internal object WebChatRealtimeVoiceFloatingLayoutPolicy {
    fun resolve(widthPx: Int, density: Float): WebChatRealtimeVoiceFloatingMetrics {
        fun dp(value: Int): Int = (value * density).toInt()
        val edgeInset = dp(16)
        val availableWidth = (widthPx - edgeInset * 2).coerceAtLeast(dp(64))
        return WebChatRealtimeVoiceFloatingMetrics(
            edgeInset = edgeInset,
            collapsedSize = dp(64),
            expandedWidth = minOf(dp(304), availableWidth),
        )
    }

    fun initialPosition(
        hostWidth: Int,
        hostHeight: Int,
        panelWidth: Int,
        panelHeight: Int,
        edgeInset: Int,
    ): WebChatRealtimeVoiceFloatingPosition = clamp(
        desiredLeft = (hostWidth - panelWidth - edgeInset).toFloat(),
        desiredTop = ((hostHeight - panelHeight) * 0.56f),
        hostWidth = hostWidth,
        hostHeight = hostHeight,
        panelWidth = panelWidth,
        panelHeight = panelHeight,
        edgeInset = edgeInset,
    )

    fun clamp(
        desiredLeft: Float,
        desiredTop: Float,
        hostWidth: Int,
        hostHeight: Int,
        panelWidth: Int,
        panelHeight: Int,
        edgeInset: Int,
    ): WebChatRealtimeVoiceFloatingPosition {
        val maxLeft = (hostWidth - panelWidth - edgeInset).coerceAtLeast(edgeInset).toFloat()
        val maxTop = (hostHeight - panelHeight - edgeInset).coerceAtLeast(edgeInset).toFloat()
        return WebChatRealtimeVoiceFloatingPosition(
            left = desiredLeft.coerceIn(edgeInset.toFloat(), maxLeft),
            top = desiredTop.coerceIn(edgeInset.toFloat(), maxTop),
        )
    }
}
