package com.elon.app

import android.graphics.Path

internal enum class ChatImageEditTool {
    BRUSH,
    TEXT,
    STICKER,
    CROP,
    MOSAIC
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
        val textSize: Float,
        val sticker: Boolean
    ) : ChatImageEditOp()
}
