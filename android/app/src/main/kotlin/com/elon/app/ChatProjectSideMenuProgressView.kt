package com.elon.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.view.View

internal data class ProgressionState(
    val level: Int,
    val percent: Int,
    val segments: FloatArray
)

internal class LevelProgressBarView(context: Context) : View(context) {
    private val trackRect = RectF()
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val segmentColors = intArrayOf(
        Color.parseColor("#58BE6A"),
        Color.parseColor("#5DA6FF"),
        Color.parseColor("#E58F46"),
        Color.parseColor("#F2C94C")
    )
    private var segments = floatArrayOf(0f, 0f, 0f, 0f)

    fun setSegments(values: FloatArray) {
        segments = FloatArray(4) { index ->
            if (index < values.size) values[index].coerceIn(0f, 1f) else 0f
        }
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val radius = height / 2f
        trackRect.set(0f, 0f, width.toFloat(), height.toFloat())
        paint.color = Color.parseColor("#34363B")
        canvas.drawRoundRect(trackRect, radius, radius, paint)
        var left = 0f
        val total = segments.sum().coerceAtMost(1f)
        segments.forEachIndexed { index, ratio ->
            if (ratio <= 0f || left >= width * total) return@forEachIndexed
            val right = (left + width * ratio).coerceAtMost(width.toFloat())
            paint.color = segmentColors[index]
            canvas.drawRect(left, 0f, right, height.toFloat(), paint)
            left = right
        }
    }
}
