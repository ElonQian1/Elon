package com.elon.app

import android.content.Context
import android.graphics.Canvas
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.Shader
import android.util.AttributeSet
import androidx.appcompat.widget.AppCompatTextView
import kotlin.math.min

class ProjectSpaceAiLabelView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyleAttr: Int = android.R.attr.textViewStyle
) : AppCompatTextView(context, attrs, defStyleAttr) {

    override fun onDraw(canvas: Canvas) {
        val value = text?.toString().orEmpty()
        val contentWidth = width - paddingLeft - paddingRight
        val contentHeight = height - paddingTop - paddingBottom
        if (value.isEmpty() || contentWidth <= 0 || contentHeight <= 0) return

        val originalShader = paint.shader
        val originalAlign = paint.textAlign
        val originalColor = paint.color
        val textWidth = paint.measureText(value)
        val needsLeadingFade = contentWidth < textWidth - 1
        if (needsLeadingFade) {
            val fadeWidth = min(contentWidth.toFloat(), textFadeWidthPx())
            paint.shader = LinearGradient(
                paddingLeft.toFloat(),
                0f,
                paddingLeft + fadeWidth,
                0f,
                transparentColor(currentTextColor),
                currentTextColor,
                Shader.TileMode.CLAMP
            )
        } else {
            paint.shader = null
            paint.color = currentTextColor
        }

        paint.textAlign = Paint.Align.LEFT
        val x = paddingLeft + contentWidth - textWidth
        val metrics = paint.fontMetrics
        val baseline = paddingTop + (contentHeight - metrics.ascent - metrics.descent) / 2f

        val saveCount = canvas.save()
        canvas.clipRect(paddingLeft, paddingTop, width - paddingRight, height - paddingBottom)
        canvas.drawText(value, x, baseline, paint)
        canvas.restoreToCount(saveCount)

        paint.shader = originalShader
        paint.textAlign = originalAlign
        paint.color = originalColor
    }

    private fun transparentColor(color: Int): Int {
        return color and 0x00FFFFFF
    }

    private fun textFadeWidthPx(): Float {
        return PROJECT_SPACE_AI_TEXT_FADE_WIDTH_DP * resources.displayMetrics.density
    }

    private companion object {
        const val PROJECT_SPACE_AI_TEXT_FADE_WIDTH_DP = 18f
    }
}
