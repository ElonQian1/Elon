package com.elon.app

import android.graphics.Path
import android.graphics.RectF

internal enum class ChatImageEditTool {
    BRUSH,
    CIRCLE,
    SQUARE,
    TEXT,
    MOSAIC
}

internal enum class ChatImageEditShape {
    CIRCLE,
    SQUARE
}

internal sealed class ChatImageEditOp {
    data class Stroke(
        val path: Path,
        val color: Int,
        val width: Float,
        val mosaic: Boolean
    ) : ChatImageEditOp()

    data class Label(
        val value: String,
        var x: Float,
        var y: Float,
        val color: Int,
        val textSize: Float
    ) : ChatImageEditOp()

    data class Shape(
        val bounds: RectF,
        val color: Int,
        val width: Float,
        val shape: ChatImageEditShape
    ) : ChatImageEditOp()
}
