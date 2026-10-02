package com.elon.app

import kotlin.math.max
import kotlin.math.min

internal data class ChatImageReadingGeometry(
    val sourceWidth: Int,
    val sourceHeight: Int,
    val viewportWidth: Int,
    val viewportHeight: Int
) {
    val fitScale: Float = min(1f, min(viewportWidth.toFloat() / sourceWidth, viewportHeight.toFloat() / sourceHeight))
    val widthScale: Float = viewportWidth.toFloat() / sourceWidth
    val isLong: Boolean = sourceHeight.toFloat() / sourceWidth >= 2.5f && sourceHeight * widthScale > viewportHeight * 1.5f
    val maxScale: Float = max(4f, widthScale * 4f)
    val topCenterY: Float = min(sourceHeight / 2f, viewportHeight / (2f * widthScale))
}
