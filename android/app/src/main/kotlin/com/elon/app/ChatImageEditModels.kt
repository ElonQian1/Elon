package com.elon.app

import android.graphics.Path
import android.graphics.PointF
import android.graphics.RectF

internal enum class ChatImageEditTool {
    ANNOTATION,
    HORIZONTAL_LINE,
    CIRCLE,
    SQUARE,
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

    data class Shape(
        val bounds: RectF,
        val color: Int,
        val width: Float,
        val shape: ChatImageEditShape
    ) : ChatImageEditOp()

    data class HorizontalLine(
        val start: PointF,
        val end: PointF,
        val color: Int,
        val width: Float
    ) : ChatImageEditOp()

    data class Annotation(
        val bounds: RectF,
        val color: Int,
        val width: Float,
        var note: String = ""
    ) : ChatImageEditOp()
}
