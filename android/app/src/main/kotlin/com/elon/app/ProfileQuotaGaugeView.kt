package com.elon.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.util.AttributeSet
import android.view.View
import kotlin.math.cos
import kotlin.math.min
import kotlin.math.roundToInt
import kotlin.math.sin

internal class ProfileQuotaGaugeView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null
) : View(context, attrs) {
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#F8F7F4")
        textAlign = Paint.Align.CENTER
        typeface = android.graphics.Typeface.create("sans-serif", android.graphics.Typeface.NORMAL)
    }
    private val captionPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.parseColor("#80BEBEBA")
        textAlign = Paint.Align.CENTER
        typeface = android.graphics.Typeface.create("sans-serif", android.graphics.Typeface.NORMAL)
    }
    private val blueShort = bitmap(R.drawable.profile_gauge_tick_blue_short)
    private val blueLong = bitmap(R.drawable.profile_gauge_tick_blue_long)
    private val neutral = bitmap(R.drawable.profile_gauge_tick_neutral)

    private var displayValue = "60%"
    private var caption = "剩余"
    private var remainingPercent: Int? = 60

    init {
        minimumHeight = dp(188)
        importantForAccessibility = IMPORTANT_FOR_ACCESSIBILITY_YES
        refreshContentDescription()
    }

    fun showQuota(percent: Int?) {
        remainingPercent = percent?.coerceIn(0, 100)
        displayValue = remainingPercent?.let { "$it%" } ?: "—"
        caption = "剩余"
        refreshContentDescription()
        invalidate()
    }

    fun showState(value: String, detail: String) {
        remainingPercent = null
        displayValue = value
        caption = detail
        refreshContentDescription()
        invalidate()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val width = MeasureSpec.getSize(widthMeasureSpec)
        val desiredHeight = dp(190)
        setMeasuredDimension(
            resolveSize(width, widthMeasureSpec),
            resolveSize(desiredHeight, heightMeasureSpec)
        )
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val centerX = width / 2f
        val centerY = height * 0.86f
        val radius = min(width * 0.44f, height * 0.76f)
        val tickCount = 31
        val usedRatio = 1f - ((remainingPercent ?: 100) / 100f)
        val blueTicks = (tickCount * usedRatio).roundToInt().coerceIn(0, tickCount)
        val tickLength = dp(9).toFloat()
        val tickThickness = dpFloat(2.6f)

        repeat(tickCount) { index ->
            val angle = 180f + (180f * index / (tickCount - 1))
            val radians = Math.toRadians(angle.toDouble())
            val x = centerX + cos(radians).toFloat() * radius
            val y = centerY + sin(radians).toFloat() * radius
            val source = when {
                index >= blueTicks -> neutral
                index % 4 == 0 -> blueLong
                else -> blueShort
            }
            canvas.save()
            canvas.translate(x, y)
            canvas.rotate(angle)
            canvas.drawBitmap(
                source,
                null,
                RectF(-tickLength / 2f, -tickThickness / 2f, tickLength / 2f, tickThickness / 2f),
                null
            )
            canvas.restore()
        }

        textPaint.textSize = sp(48f)
        val valueY = centerY - dp(16)
        canvas.drawText(displayValue, centerX, valueY, textPaint)
        captionPaint.textSize = sp(13f)
        canvas.drawText(caption, centerX, valueY + dp(27), captionPaint)
    }

    private fun bitmap(resourceId: Int): Bitmap =
        requireNotNull(BitmapFactory.decodeResource(resources, resourceId))

    private fun refreshContentDescription() {
        contentDescription = "Token 额度，$displayValue，$caption"
    }

    private fun dp(value: Int): Int =
        (value * resources.displayMetrics.density + 0.5f).toInt()

    private fun dpFloat(value: Float): Float = value * resources.displayMetrics.density

    private fun sp(value: Float): Float = value * resources.displayMetrics.scaledDensity
}
