package com.elon.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.ColorFilter
import android.graphics.LinearGradient
import android.graphics.Outline
import android.graphics.Paint
import android.graphics.Path
import android.graphics.PixelFormat
import android.graphics.RectF
import android.graphics.Shader
import android.graphics.drawable.Drawable
import androidx.core.content.ContextCompat

/**
 * A quiet orbital instrument panel: faceted gunmetal, double edges and a calibration rail.
 * Geometry and shaders are cached when bounds change so the carousel stays inexpensive to draw.
 */
internal class ProjectPlazaMetalPanelDrawable(context: Context) : Drawable() {
    private val density = context.resources.displayMetrics.density
    private val fillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val edgePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_plaza_metal_edge)
        style = Paint.Style.STROKE
        strokeWidth = density
    }
    private val highlightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_plaza_metal_highlight_soft)
        style = Paint.Style.STROKE
        strokeWidth = density
    }
    private val calibrationPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_plaza_calibration)
        style = Paint.Style.STROKE
        strokeWidth = density
        strokeCap = Paint.Cap.SQUARE
    }
    private val lowColor = ContextCompat.getColor(context, R.color.elon_plaza_surface_card)
    private val midColor = ContextCompat.getColor(context, R.color.elon_plaza_surface_card_mid)
    private val highColor = ContextCompat.getColor(context, R.color.elon_plaza_surface_card_high)
    private val panelPath = Path()
    private val innerPath = Path()
    private val topHighlightPath = Path()
    private val panelRect = RectF()

    override fun onBoundsChange(bounds: android.graphics.Rect) {
        val left = bounds.left.toFloat()
        val top = bounds.top.toFloat()
        val right = bounds.right.toFloat()
        val bottom = bounds.bottom.toFloat()
        val cut = 18f * density
        val smallCut = 11f * density
        panelRect.set(left, top, right, bottom)
        buildPanelPath(panelPath, left, top, right, bottom, cut, smallCut)

        val inset = 3f * density
        buildPanelPath(
            innerPath,
            left + inset,
            top + inset,
            right - inset,
            bottom - inset,
            cut - inset,
            smallCut - inset,
        )
        topHighlightPath.reset()
        topHighlightPath.moveTo(left + cut, top + 1.5f * density)
        topHighlightPath.lineTo(right - cut - 1.5f * density, top + 1.5f * density)
        topHighlightPath.lineTo(right - 1.5f * density, top + cut)

        fillPaint.shader = LinearGradient(
            left,
            top,
            right,
            bottom,
            intArrayOf(highColor, midColor, lowColor, Color.rgb(10, 13, 17)),
            floatArrayOf(0f, 0.28f, 0.72f, 1f),
            Shader.TileMode.CLAMP,
        )
    }

    override fun draw(canvas: Canvas) {
        canvas.drawPath(panelPath, fillPaint)
        canvas.drawPath(panelPath, edgePaint)
        canvas.drawPath(innerPath, highlightPaint)
        canvas.drawPath(topHighlightPath, highlightPaint)

        val right = panelRect.right - 7f * density
        val startY = panelRect.top + 83f * density
        repeat(5) { index ->
            val y = startY + index * 9f * density
            val length = if (index == 2) 12f * density else 7f * density
            canvas.drawLine(right - length, y, right, y, calibrationPaint)
        }
        val left = panelRect.left + 7f * density
        val bottom = panelRect.bottom - 11f * density
        canvas.drawLine(left, bottom, left + 16f * density, bottom, calibrationPaint)
    }

    override fun getOutline(outline: Outline) {
        @Suppress("DEPRECATION")
        outline.setConvexPath(panelPath)
    }

    override fun setAlpha(alpha: Int) {
        fillPaint.alpha = alpha
        edgePaint.alpha = alpha
        highlightPaint.alpha = alpha
        calibrationPaint.alpha = alpha
        invalidateSelf()
    }

    override fun setColorFilter(colorFilter: ColorFilter?) {
        fillPaint.colorFilter = colorFilter
        edgePaint.colorFilter = colorFilter
        highlightPaint.colorFilter = colorFilter
        calibrationPaint.colorFilter = colorFilter
        invalidateSelf()
    }

    @Deprecated("Deprecated in Android")
    override fun getOpacity(): Int = PixelFormat.TRANSLUCENT

    private fun buildPanelPath(
        path: Path,
        left: Float,
        top: Float,
        right: Float,
        bottom: Float,
        cut: Float,
        smallCut: Float,
    ) {
        path.reset()
        path.moveTo(left + smallCut, top)
        path.lineTo(right - cut, top)
        path.lineTo(right, top + cut)
        path.lineTo(right, bottom - smallCut)
        path.lineTo(right - smallCut, bottom)
        path.lineTo(left + cut, bottom)
        path.lineTo(left, bottom - cut)
        path.lineTo(left, top + smallCut)
        path.close()
    }
}

internal class ProjectPlazaMetalActionDrawable(context: Context) : Drawable() {
    private val density = context.resources.displayMetrics.density
    private val rect = RectF()
    private val fillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val edgePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_plaza_action_edge)
        style = Paint.Style.STROKE
        strokeWidth = density
    }
    private val highlightPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_plaza_action_highlight)
        style = Paint.Style.STROKE
        strokeWidth = density
    }
    private val startColor = ContextCompat.getColor(context, R.color.elon_plaza_action)
    private val midColor = ContextCompat.getColor(context, R.color.elon_plaza_action_mid)
    private val endColor = ContextCompat.getColor(context, R.color.elon_plaza_action_end)

    override fun onBoundsChange(bounds: android.graphics.Rect) {
        rect.set(bounds)
        fillPaint.shader = LinearGradient(
            rect.left,
            rect.top,
            rect.right,
            rect.bottom,
            intArrayOf(startColor, midColor, endColor),
            floatArrayOf(0f, 0.48f, 1f),
            Shader.TileMode.CLAMP,
        )
    }

    override fun draw(canvas: Canvas) {
        val radius = rect.height() / 2f
        canvas.drawRoundRect(rect, radius, radius, fillPaint)
        val borderInset = 0.5f * density
        canvas.drawRoundRect(
            rect.left + borderInset,
            rect.top + borderInset,
            rect.right - borderInset,
            rect.bottom - borderInset,
            radius,
            radius,
            edgePaint,
        )
        val top = rect.top + 2f * density
        canvas.drawLine(rect.left + radius, top, rect.right - radius, top, highlightPaint)
    }

    override fun getOutline(outline: Outline) {
        outline.setRoundRect(bounds, bounds.height() / 2f)
    }

    override fun setAlpha(alpha: Int) {
        fillPaint.alpha = alpha
        edgePaint.alpha = alpha
        highlightPaint.alpha = alpha
        invalidateSelf()
    }

    override fun setColorFilter(colorFilter: ColorFilter?) {
        fillPaint.colorFilter = colorFilter
        edgePaint.colorFilter = colorFilter
        highlightPaint.colorFilter = colorFilter
        invalidateSelf()
    }

    @Deprecated("Deprecated in Android")
    override fun getOpacity(): Int = PixelFormat.TRANSLUCENT
}
