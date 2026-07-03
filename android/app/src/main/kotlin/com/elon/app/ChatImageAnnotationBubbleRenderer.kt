package com.elon.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.text.Layout
import android.text.StaticLayout
import android.text.TextPaint
import kotlin.math.ceil
import kotlin.math.max
import kotlin.math.min

internal class ChatImageAnnotationBubbleRenderer(context: Context) {
    private val density = context.resources.displayMetrics.density
    private val fillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = Color.parseColor("#D9D9D9")
    }
    private val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        strokeJoin = Paint.Join.ROUND
        strokeWidth = dp(3).toFloat()
        color = Color.parseColor("#3F3F3F")
    }
    private val textPaint = TextPaint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#2E2E2E")
        textSize = dp(14).toFloat()
    }

    fun draw(canvas: Canvas, note: String, anchor: RectF, viewWidth: Int, viewHeight: Int) {
        val cleanNote = note.trim()
        if (cleanNote.isEmpty() || viewWidth <= 0 || viewHeight <= 0) return

        val metrics = measure(cleanNote, viewWidth)
        val placement = placeBubble(anchor, metrics.width, metrics.height, viewWidth, viewHeight)
        drawTail(canvas, placement)
        canvas.drawRoundRect(placement.rect, dp(10).toFloat(), dp(10).toFloat(), fillPaint)
        canvas.drawRoundRect(placement.rect, dp(10).toFloat(), dp(10).toFloat(), strokePaint)

        canvas.save()
        canvas.translate(placement.rect.left + dp(14), placement.rect.top + dp(11))
        metrics.textLayout.draw(canvas)
        canvas.restore()
    }

    private fun measure(note: String, viewWidth: Int): BubbleMeasure {
        val edgePad = dp(12)
        val maxWidth = min(dp(320), max(dp(132), viewWidth - edgePad * 2))
        val desiredWidth = ceil(textPaint.measureText(note)).toInt() + dp(28)
        val bubbleWidth = desiredWidth.coerceIn(dp(116), maxWidth)
        val textWidth = max(dp(48), bubbleWidth - dp(28))
        val textLayout = StaticLayout.Builder
            .obtain(note, 0, note.length, textPaint, textWidth)
            .setAlignment(Layout.Alignment.ALIGN_NORMAL)
            .setLineSpacing(0f, 1f)
            .setIncludePad(false)
            .setMaxLines(8)
            .build()
        val bubbleHeight = max(dp(44), textLayout.height + dp(22))
        return BubbleMeasure(bubbleWidth.toFloat(), bubbleHeight.toFloat(), textLayout)
    }

    private fun placeBubble(
        anchor: RectF,
        bubbleWidth: Float,
        bubbleHeight: Float,
        viewWidth: Int,
        viewHeight: Int
    ): BubblePlacement {
        val edgePad = dp(12).toFloat()
        val tail = dp(12).toFloat()
        val left = (anchor.centerX() - bubbleWidth / 2f)
            .coerceIn(edgePad, max(edgePad, viewWidth - bubbleWidth - edgePad))
        val topAbove = anchor.top - tail - bubbleHeight
        val topBelow = anchor.bottom + tail
        val showAbove = topAbove >= edgePad || topBelow + bubbleHeight > viewHeight - edgePad
        val rawTop = if (showAbove) topAbove else topBelow
        val top = rawTop.coerceIn(edgePad, max(edgePad, viewHeight - bubbleHeight - edgePad))
        val rect = RectF(left, top, left + bubbleWidth, top + bubbleHeight)
        val targetX = anchor.centerX().coerceIn(rect.left + dp(22), rect.right - dp(22))
        return BubblePlacement(rect, targetX, showAbove)
    }

    private fun drawTail(canvas: Canvas, placement: BubblePlacement) {
        val tailHalf = dp(11).toFloat()
        val tailHeight = dp(12).toFloat()
        val path = Path()
        if (placement.aboveAnchor) {
            val y = placement.rect.bottom
            path.moveTo(placement.targetX - tailHalf, y - dp(1))
            path.lineTo(placement.targetX, y + tailHeight)
            path.lineTo(placement.targetX + tailHalf, y - dp(1))
        } else {
            val y = placement.rect.top
            path.moveTo(placement.targetX - tailHalf, y + dp(1))
            path.lineTo(placement.targetX, y - tailHeight)
            path.lineTo(placement.targetX + tailHalf, y + dp(1))
        }
        path.close()
        canvas.drawPath(path, fillPaint)
        canvas.drawPath(path, strokePaint)
    }

    private fun dp(value: Int): Int = (value * density).toInt()

    private data class BubbleMeasure(
        val width: Float,
        val height: Float,
        val textLayout: StaticLayout
    )

    private data class BubblePlacement(
        val rect: RectF,
        val targetX: Float,
        val aboveAnchor: Boolean
    )
}
