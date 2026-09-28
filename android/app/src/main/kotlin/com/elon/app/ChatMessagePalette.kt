package com.elon.app

import android.content.Context
import android.graphics.LinearGradient
import android.graphics.Shader

/** Message roles must resolve against the same theme as their bubble, including recycled rows. */
internal class ChatMessagePalette(context: Context) {
    private val colors = MobileColors(context)

    fun foreground(role: String, recalled: Boolean): Int = when {
        role == "user" -> colors.onPrimaryContainer
        recalled -> colors.muted
        role == "error" -> colors.error
        role == "ai-stopped" -> colors.warning
        role in setOf("ai-working", "ai-progress", "ai-cli-log", "ai-tool", "ai-complete") -> colors.muted
        else -> colors.text
    }

    fun status(failed: Boolean, read: Boolean): Int = when {
        failed -> colors.error
        read -> colors.success
        else -> colors.muted
    }

    fun shimmer(width: Int): LinearGradient = LinearGradient(
        0f, 0f, width.toFloat(), 0f,
        intArrayOf(colors.muted, colors.text, colors.text, colors.text, colors.muted),
        floatArrayOf(0f, 0.28f, 0.5f, 0.72f, 1f), Shader.TileMode.CLAMP
    )
}
