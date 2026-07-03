package com.elon.app

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.util.AttributeSet
import android.view.MotionEvent
import android.view.View
import kotlin.math.max
import kotlin.math.min

internal class ChatImageAnnotationOverlayView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null
) : View(context, attrs) {
    private val iconPaint = Paint(Paint.ANTI_ALIAS_FLAG or Paint.FILTER_BITMAP_FLAG).apply {
        alpha = 235
    }
    private val fallbackIcon: Bitmap? by lazy {
        BitmapFactory.decodeResource(resources, R.drawable.ic_chat_image_tool_annotation_filled)
            ?: BitmapFactory.decodeResource(resources, R.drawable.ic_chat_image_tool_annotation)
    }

    private var imageWidth = 0
    private var imageHeight = 0
    private var annotations: List<ChatImageAnnotation> = emptyList()
    private var pressedIndex: Int? = null

    var onAnnotationClick: ((ChatImageAnnotation) -> Unit)? = null

    fun setImageInfo(width: Int?, height: Int?, nextAnnotations: List<ChatImageAnnotation>) {
        imageWidth = width?.takeIf { it > 0 } ?: 0
        imageHeight = height?.takeIf { it > 0 } ?: 0
        annotations = nextAnnotations.filter { it.hasNote() }
        visibility = if (imageWidth > 0 && imageHeight > 0 && annotations.isNotEmpty()) {
            VISIBLE
        } else {
            GONE
        }
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val icon = fallbackIcon ?: return
        val imageRect = displayedImageRect() ?: return
        annotations.forEach { annotation ->
            val iconRect = iconRectOnView(annotation, imageRect) ?: return@forEach
            canvas.drawBitmap(icon, null, iconRect, iconPaint)
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (annotations.isEmpty()) return false
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                val hitIndex = findAnnotationAt(event.x, event.y) ?: return false
                pressedIndex = hitIndex
                parent?.requestDisallowInterceptTouchEvent(true)
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                return pressedIndex != null
            }
            MotionEvent.ACTION_UP -> {
                val hitIndex = pressedIndex ?: return false
                pressedIndex = null
                if (findAnnotationAt(event.x, event.y) == hitIndex) {
                    annotations.getOrNull(hitIndex)?.let { onAnnotationClick?.invoke(it) }
                }
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                if (pressedIndex == null) return false
                pressedIndex = null
                return true
            }
        }
        return false
    }

    private fun findAnnotationAt(x: Float, y: Float): Int? {
        val imageRect = displayedImageRect() ?: return null
        for (index in annotations.lastIndex downTo 0) {
            val iconRect = iconRectOnView(annotations[index], imageRect) ?: continue
            val hitRect = RectF(iconRect).apply {
                val extraX = max(0f, (dp(48).toFloat() - width()) / 2f)
                val extraY = max(0f, (dp(48).toFloat() - height()) / 2f)
                inset(-extraX, -extraY)
            }
            if (hitRect.contains(x, y)) return index
        }
        return null
    }

    private fun displayedImageRect(): RectF? {
        if (imageWidth <= 0 || imageHeight <= 0 || width <= 0 || height <= 0) return null
        val scale = min(width / imageWidth.toFloat(), height / imageHeight.toFloat())
        val drawnWidth = imageWidth * scale
        val drawnHeight = imageHeight * scale
        val left = (width - drawnWidth) / 2f
        val top = (height - drawnHeight) / 2f
        return RectF(left, top, left + drawnWidth, top + drawnHeight)
    }

    private fun iconRectOnView(annotation: ChatImageAnnotation, imageRect: RectF): RectF? {
        val iconX = annotation.iconX
        val iconY = annotation.iconY
        val iconWidth = annotation.iconWidth
        val iconHeight = annotation.iconHeight
        if (iconX != null && iconY != null && iconWidth != null && iconHeight != null &&
            iconWidth > 0f && iconHeight > 0f
        ) {
            return RectF(
                imageRect.left + imageRect.width() * iconX,
                imageRect.top + imageRect.height() * iconY,
                imageRect.left + imageRect.width() * (iconX + iconWidth),
                imageRect.top + imageRect.height() * (iconY + iconHeight)
            )
        }

        val bounds = annotationBoundsOnView(annotation, imageRect) ?: return null
        val size = dp(36).toFloat()
        val inset = dp(5).toFloat()
        val edgePad = dp(8).toFloat()
        val rawLeft = bounds.right - size - inset
        val rawTop = bounds.bottom - size - inset
        val left = rawLeft.coerceIn(
            imageRect.left + edgePad,
            max(imageRect.left + edgePad, imageRect.right - size - edgePad)
        )
        val top = rawTop.coerceIn(
            imageRect.top + edgePad,
            max(imageRect.top + edgePad, imageRect.bottom - size - edgePad)
        )
        return RectF(left, top, left + size, top + size)
    }

    private fun annotationBoundsOnView(annotation: ChatImageAnnotation, imageRect: RectF): RectF? {
        if (annotation.width <= 0f || annotation.height <= 0f) return null
        val left = imageRect.left + imageRect.width() * annotation.x
        val top = imageRect.top + imageRect.height() * annotation.y
        val right = imageRect.left + imageRect.width() * (annotation.x + annotation.width)
        val bottom = imageRect.top + imageRect.height() * (annotation.y + annotation.height)
        return RectF(left, top, right, bottom)
    }

    private fun dp(value: Int): Int {
        return (value * resources.displayMetrics.density).toInt()
    }
}
