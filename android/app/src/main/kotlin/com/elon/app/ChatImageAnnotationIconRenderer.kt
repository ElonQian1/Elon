package com.elon.app

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface

internal class ChatImageAnnotationIconRenderer {
    private val iconPaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG).apply {
        alpha = ICON_ALPHA
    }
    private val badgePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = Color.parseColor("#E62129")
    }
    private val badgeTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        textAlign = Paint.Align.CENTER
        typeface = Typeface.DEFAULT_BOLD
    }

    fun draw(
        canvas: Canvas,
        icon: Bitmap,
        iconRect: RectF,
        number: Int?,
        alpha: Int = ICON_ALPHA
    ) {
        val oldAlpha = iconPaint.alpha
        iconPaint.alpha = alpha.coerceIn(0, ICON_ALPHA)
        canvas.drawBitmap(icon, null, iconRect, iconPaint)
        drawNumberBadge(canvas, iconRect, number, iconPaint.alpha / ICON_ALPHA.toFloat())
        iconPaint.alpha = oldAlpha
    }

    private fun drawNumberBadge(canvas: Canvas, iconRect: RectF, number: Int?, alphaScale: Float) {
        val safeNumber = number?.takeIf { it > 0 } ?: return
        val label = if (safeNumber > 99) "99+" else safeNumber.toString()
        val side = minOf(iconRect.width(), iconRect.height())
        if (side <= 0f) return

        val diameter = side * when (label.length) {
            1 -> 0.30f
            2 -> 0.36f
            else -> 0.38f
        }
        val radius = diameter / 2f
        val centerX = iconRect.left + iconRect.width() * 0.72f
        val centerY = iconRect.top + iconRect.height() * 0.48f
        val badgeAlpha = (255 * alphaScale).toInt().coerceIn(0, 255)
        if (badgeAlpha <= 0) return

        val oldBadgeAlpha = badgePaint.alpha
        val oldTextAlpha = badgeTextPaint.alpha
        val oldTextSize = badgeTextPaint.textSize
        badgePaint.alpha = badgeAlpha
        badgeTextPaint.alpha = badgeAlpha
        badgeTextPaint.textSize = diameter * when (label.length) {
            1 -> 0.64f
            2 -> 0.52f
            else -> 0.40f
        }

        canvas.drawCircle(centerX, centerY, radius, badgePaint)
        val fontMetrics = badgeTextPaint.fontMetrics
        val textY = centerY - (fontMetrics.ascent + fontMetrics.descent) / 2f
        canvas.drawText(label, centerX, textY, badgeTextPaint)

        badgePaint.alpha = oldBadgeAlpha
        badgeTextPaint.alpha = oldTextAlpha
        badgeTextPaint.textSize = oldTextSize
    }

    private companion object {
        const val ICON_ALPHA = 235
    }
}
