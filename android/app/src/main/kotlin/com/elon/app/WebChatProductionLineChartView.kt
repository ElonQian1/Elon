package com.elon.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.view.View
import androidx.core.content.ContextCompat

internal class WebChatProductionLineChartView(context: Context) : View(context) {
    private val gridPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_divider_card)
        strokeWidth = dp(1).toFloat()
    }
    private val linePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
        strokeWidth = dp(2).toFloat()
    }
    private val seriesColors = intArrayOf(
        ContextCompat.getColor(context, R.color.elon_accent_primary),
        ContextCompat.getColor(context, R.color.elon_status_success),
        ContextCompat.getColor(context, R.color.elon_status_project),
        ContextCompat.getColor(context, R.color.elon_status_danger),
    )
    private var points: List<WebChatProductionRichCard.Point> = emptyList()
    private var seriesCount: Int = 0

    fun bind(card: WebChatProductionRichCard) {
        points = card.points
        seriesCount = card.series.size.coerceAtMost(seriesColors.size)
        contentDescription = card.title
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        if (points.size < 2 || seriesCount == 0) return
        val values = points.flatMap { it.values.take(seriesCount) }
        if (values.isEmpty()) return
        var minimum = values.minOrNull() ?: return
        var maximum = values.maxOrNull() ?: return
        if (minimum == maximum) {
            val padding = if (minimum == 0.0) 1.0 else kotlin.math.abs(minimum) * 0.05
            minimum -= padding
            maximum += padding
        }
        val left = paddingLeft.toFloat()
        val top = paddingTop.toFloat()
        val right = (width - paddingRight).toFloat()
        val bottom = (height - paddingBottom).toFloat()
        if (right <= left || bottom <= top) return
        repeat(4) { index ->
            val y = top + (bottom - top) * index / 3f
            canvas.drawLine(left, y, right, y, gridPaint)
        }
        repeat(seriesCount) { seriesIndex ->
            val path = Path()
            points.forEachIndexed { index, point ->
                val value = point.values.getOrNull(seriesIndex) ?: return@forEachIndexed
                val x = left + (right - left) * index / (points.size - 1).toFloat()
                val y = bottom - ((value - minimum) / (maximum - minimum)).toFloat() * (bottom - top)
                if (index == 0) path.moveTo(x, y) else path.lineTo(x, y)
            }
            linePaint.color = seriesColors[seriesIndex]
            canvas.drawPath(path, linePaint)
        }
    }

    private fun dp(value: Int): Int = (value * resources.displayMetrics.density).toInt()
}
