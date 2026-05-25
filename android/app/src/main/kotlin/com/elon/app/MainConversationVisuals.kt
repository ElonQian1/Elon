package com.elon.app

import android.graphics.Color

internal fun blendColor(startColor: Int, endColor: Int, fraction: Float): Int {
    val clamped = fraction.coerceIn(0f, 1f)
    val alpha = (Color.alpha(startColor) + (Color.alpha(endColor) - Color.alpha(startColor)) * clamped).toInt()
    val red = (Color.red(startColor) + (Color.red(endColor) - Color.red(startColor)) * clamped).toInt()
    val green = (Color.green(startColor) + (Color.green(endColor) - Color.green(startColor)) * clamped).toInt()
    val blue = (Color.blue(startColor) + (Color.blue(endColor) - Color.blue(startColor)) * clamped).toInt()
    return Color.argb(alpha, red, green, blue)
}

internal fun avatarText(title: String): String {
    return if (title.startsWith("一龙")) "龙" else title.take(1).ifBlank { "新" }
}

internal fun conversationSubtitleColor(text: String): Int {
    return when {
        text.startsWith("已连接") -> Color.parseColor("#07C160")
        text.startsWith("未连接") -> Color.parseColor("#D93025")
        text.startsWith("工作完成") -> Color.parseColor("#07C160")
        text.startsWith("工作停止") -> Color.parseColor("#D93025")
        text.startsWith("会话已结束") -> Color.parseColor("#6E6E6E")
        else -> Color.parseColor("#A9A9A9")
    }
}
