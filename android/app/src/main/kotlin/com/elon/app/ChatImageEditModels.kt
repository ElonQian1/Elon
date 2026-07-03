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

data class ChatImageAnnotation(
    val x: Float,
    val y: Float,
    val width: Float,
    val height: Float,
    val note: String,
    val iconX: Float? = null,
    val iconY: Float? = null,
    val iconWidth: Float? = null,
    val iconHeight: Float? = null
) {
    fun hasNote(): Boolean = note.trim().isNotEmpty()
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
