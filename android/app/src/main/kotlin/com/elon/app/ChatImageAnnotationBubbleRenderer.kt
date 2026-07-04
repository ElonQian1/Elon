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
        drawBubbleShape(canvas, placement)

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
        val tail = dp(13).toFloat()
        val left = (anchor.centerX() - bubbleWidth / 2f)
            .coerceIn(edgePad, max(edgePad, viewWidth - bubbleWidth - edgePad))
        val topAbove = anchor.top - tail - bubbleHeight
        val topBelow = anchor.bottom + tail
        val showAbove = topAbove >= edgePad || topBelow + bubbleHeight > viewHeight - edgePad
        val rawTop = if (showAbove) topAbove else topBelow
        val top = rawTop.coerceIn(edgePad, max(edgePad, viewHeight - bubbleHeight - edgePad))
        val rect = RectF(left, top, left + bubbleWidth, top + bubbleHeight)
        val targetX = anchor.centerX().coerceIn(rect.left + dp(27), rect.right - dp(27))
        return BubblePlacement(rect, targetX, showAbove)
    }

    private fun drawBubbleShape(canvas: Canvas, placement: BubblePlacement) {
        val rect = placement.rect
        val radius = dp(13).toFloat()
        val tailHalf = dp(14).toFloat()
        val tailHeight = dp(13).toFloat()
        val tipHalf = dp(3).toFloat()
        val tipRound = dp(2).toFloat()
        val path = Path()
        val arc = RectF()

        path.moveTo(rect.left + radius, rect.top)
        if (placement.aboveAnchor) {
            path.lineTo(rect.right - radius, rect.top)
            arc.set(rect.right - radius * 2f, rect.top, rect.right, rect.top + radius * 2f)
            path.arcTo(arc, -90f, 90f)
            path.lineTo(rect.right, rect.bottom - radius)
            arc.set(rect.right - radius * 2f, rect.bottom - radius * 2f, rect.right, rect.bottom)
            path.arcTo(arc, 0f, 90f)
            path.lineTo(placement.targetX + tailHalf, rect.bottom)
            path.cubicTo(
                placement.targetX + tailHalf * 0.64f,
                rect.bottom + tailHeight * 0.12f,
                placement.targetX + tipHalf,
                rect.bottom + tailHeight * 0.72f,
                placement.targetX + tipHalf,
                rect.bottom + tailHeight - tipRound
            )
            path.quadTo(
                placement.targetX,
                rect.bottom + tailHeight,
                placement.targetX - tipHalf,
                rect.bottom + tailHeight - tipRound
            )
            path.cubicTo(
                placement.targetX - tipHalf,
                rect.bottom + tailHeight * 0.72f,
                placement.targetX - tailHalf * 0.64f,
                rect.bottom + tailHeight * 0.12f,
                placement.targetX - tailHalf,
                rect.bottom
            )
            path.lineTo(rect.left + radius, rect.bottom)
            arc.set(rect.left, rect.bottom - radius * 2f, rect.left + radius * 2f, rect.bottom)
            path.arcTo(arc, 90f, 90f)
            path.lineTo(rect.left, rect.top + radius)
            arc.set(rect.left, rect.top, rect.left + radius * 2f, rect.top + radius * 2f)
            path.arcTo(arc, 180f, 90f)
        } else {
            path.lineTo(placement.targetX - tailHalf, rect.top)
            path.cubicTo(
                placement.targetX - tailHalf * 0.64f,
                rect.top - tailHeight * 0.12f,
                placement.targetX - tipHalf,
                rect.top - tailHeight * 0.72f,
                placement.targetX - tipHalf,
                rect.top - tailHeight + tipRound
            )
            path.quadTo(
                placement.targetX,
                rect.top - tailHeight,
                placement.targetX + tipHalf,
                rect.top - tailHeight + tipRound
            )
            path.cubicTo(
                placement.targetX + tipHalf,
                rect.top - tailHeight * 0.72f,
                placement.targetX + tailHalf * 0.64f,
                rect.top - tailHeight * 0.12f,
                placement.targetX + tailHalf,
                rect.top
            )
            path.lineTo(rect.right - radius, rect.top)
            arc.set(rect.right - radius * 2f, rect.top, rect.right, rect.top + radius * 2f)
            path.arcTo(arc, -90f, 90f)
            path.lineTo(rect.right, rect.bottom - radius)
            arc.set(rect.right - radius * 2f, rect.bottom - radius * 2f, rect.right, rect.bottom)
            path.arcTo(arc, 0f, 90f)
            path.lineTo(rect.left + radius, rect.bottom)
            arc.set(rect.left, rect.bottom - radius * 2f, rect.left + radius * 2f, rect.bottom)
            path.arcTo(arc, 90f, 90f)
            path.lineTo(rect.left, rect.top + radius)
            arc.set(rect.left, rect.top, rect.left + radius * 2f, rect.top + radius * 2f)
            path.arcTo(arc, 180f, 90f)
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
