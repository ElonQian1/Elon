package com.elon.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.PixelFormat
import android.graphics.RectF
import android.graphics.drawable.Drawable
import androidx.core.content.ContextCompat

/** Static, deterministic deep-space field. It never animates or allocates while drawing. */
internal class ProjectPlazaBackdropDrawable(context: Context) : Drawable() {
    private val voidPaint = Paint().apply {
        color = ContextCompat.getColor(context, R.color.elon_bg_plaza)
    }
    private val constellationPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_plaza_constellation)
        strokeWidth = context.resources.displayMetrics.density
        style = Paint.Style.STROKE
    }
    private val starPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_plaza_star)
        style = Paint.Style.FILL
    }
    private val gridPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_plaza_grid)
        strokeWidth = context.resources.displayMetrics.density
        style = Paint.Style.STROKE
    }
    private val reticlePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = ContextCompat.getColor(context, R.color.elon_plaza_calibration)
        strokeWidth = context.resources.displayMetrics.density
        style = Paint.Style.STROKE
    }
    private val orbit = RectF()
    private val gridX = floatArrayOf(0.2f, 0.5f, 0.8f)
    private val gridY = floatArrayOf(0.29f, 0.71f)
    private val stars = arrayOf(
        0.07f to 0.06f, 0.18f to 0.17f, 0.34f to 0.09f, 0.52f to 0.21f,
        0.73f to 0.11f, 0.91f to 0.25f, 0.12f to 0.38f, 0.39f to 0.34f,
        0.63f to 0.43f, 0.84f to 0.36f, 0.25f to 0.57f, 0.48f to 0.64f,
        0.78f to 0.59f, 0.94f to 0.71f, 0.09f to 0.76f, 0.31f to 0.84f,
        0.59f to 0.79f, 0.82f to 0.91f
    )

    override fun draw(canvas: Canvas) {
        canvas.drawRect(bounds, voidPaint)
    }

    override fun setAlpha(alpha: Int) {
        voidPaint.alpha = alpha
        constellationPaint.alpha = alpha
        starPaint.alpha = alpha
        gridPaint.alpha = alpha
        reticlePaint.alpha = alpha
        invalidateSelf()
    }

    override fun setColorFilter(colorFilter: android.graphics.ColorFilter?) {
        voidPaint.colorFilter = colorFilter
        constellationPaint.colorFilter = colorFilter
        starPaint.colorFilter = colorFilter
        gridPaint.colorFilter = colorFilter
        reticlePaint.colorFilter = colorFilter
        invalidateSelf()
    }

    @Deprecated("Deprecated in Android")
    override fun getOpacity(): Int = PixelFormat.OPAQUE

    private fun drawReticle(canvas: Canvas, x: Float, y: Float, radiusDp: Float) {
        val density = reticlePaint.strokeWidth
        val radius = radiusDp * density
        val gap = 3f * density
        val arm = 6f * density
        canvas.drawCircle(x, y, radius, reticlePaint)
        canvas.drawLine(x - radius - arm, y, x - radius - gap, y, reticlePaint)
        canvas.drawLine(x + radius + gap, y, x + radius + arm, y, reticlePaint)
        canvas.drawLine(x, y - radius - arm, x, y - radius - gap, reticlePaint)
        canvas.drawLine(x, y + radius + gap, x, y + radius + arm, reticlePaint)
    }
}
