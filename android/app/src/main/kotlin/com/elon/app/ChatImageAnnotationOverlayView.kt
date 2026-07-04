package com.elon.app

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
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
    private val annotationIconRenderer = ChatImageAnnotationIconRenderer()
    private val boundsPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        color = Color.WHITE
        alpha = 220
        strokeWidth = dp(2).toFloat()
    }
    private val fallbackIcon: Bitmap? by lazy {
        BitmapFactory.decodeResource(resources, R.drawable.ic_chat_image_tool_annotation_filled)
            ?: BitmapFactory.decodeResource(resources, R.drawable.ic_chat_image_tool_annotation)
    }
    private val annotationBubbleRenderer = ChatImageAnnotationBubbleRenderer(context)

    private var imageWidth = 0
    private var imageHeight = 0
    private var annotations: List<ChatImageAnnotation> = emptyList()
    private var pressedIndex: Int? = null
    private var collapseOnTouchUp = false
    private var expandedIndex: Int? = null
    private var appearingIndex: Int? = null
    private var appearingProgress = 1f
    private var iconAppearAnimator: ValueAnimator? = null

    fun setImageInfo(width: Int?, height: Int?, nextAnnotations: List<ChatImageAnnotation>) {
        imageWidth = width?.takeIf { it > 0 } ?: 0
        imageHeight = height?.takeIf { it > 0 } ?: 0
        annotations = nextAnnotations.filter { it.hasNote() }
        expandedIndex = expandedIndex?.takeIf { it in annotations.indices }
        appearingIndex = appearingIndex?.takeIf { it in annotations.indices }
        visibility = if (imageWidth > 0 && imageHeight > 0 && annotations.isNotEmpty()) {
            VISIBLE
        } else {
            expandedIndex = null
            appearingIndex = null
            iconAppearAnimator?.cancel()
            GONE
        }
        invalidate()
    }

    fun collapseExpandedAnnotation(): Boolean {
        val index = expandedIndex ?: return false
        expandedIndex = null
        startIconAppearAnimation(index)
        invalidate()
        return true
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val icon = fallbackIcon ?: return
        val imageRect = displayedImageRect() ?: return
        drawExpandedAnnotation(canvas, imageRect)
        annotations.forEachIndexed { index, annotation ->
            if (expandedIndex == index) return@forEachIndexed
            val iconRect = iconRectOnView(annotation, imageRect) ?: return@forEachIndexed
            drawAnnotationIcon(canvas, icon, iconRect, index)
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (annotations.isEmpty()) return false
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                val hitIndex = findAnnotationAt(event.x, event.y)
                if (hitIndex == null) {
                    if (expandedIndex == null) return false
                    collapseOnTouchUp = true
                    parent?.requestDisallowInterceptTouchEvent(true)
                    return true
                }
                pressedIndex = hitIndex
                parent?.requestDisallowInterceptTouchEvent(true)
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                return pressedIndex != null || collapseOnTouchUp
            }
            MotionEvent.ACTION_UP -> {
                if (collapseOnTouchUp) {
                    collapseOnTouchUp = false
                    collapseExpandedAnnotation()
                    return true
                }
                val hitIndex = pressedIndex ?: return false
                pressedIndex = null
                if (findAnnotationAt(event.x, event.y) == hitIndex) {
                    iconAppearAnimator?.cancel()
                    appearingIndex = null
                    expandedIndex = hitIndex
                    invalidate()
                }
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                if (pressedIndex == null && !collapseOnTouchUp) return false
                pressedIndex = null
                collapseOnTouchUp = false
                return true
            }
        }
        return false
    }

    private fun drawExpandedAnnotation(canvas: Canvas, imageRect: RectF) {
        val index = expandedIndex ?: return
        val annotation = annotations.getOrNull(index) ?: return
        val bounds = annotationBoundsOnView(annotation, imageRect) ?: return
        canvas.drawRect(bounds, boundsPaint)
        annotationBubbleRenderer.draw(canvas, annotation.note, bounds, width, height)
    }

    private fun drawAnnotationIcon(canvas: Canvas, icon: Bitmap, iconRect: RectF, index: Int) {
        val progress = if (appearingIndex == index) appearingProgress.coerceIn(0f, 1f) else 1f
        val scale = 0.72f + 0.28f * progress
        val drawRect = if (scale >= 0.999f) {
            iconRect
        } else {
            val insetX = iconRect.width() * (1f - scale) / 2f
            val insetY = iconRect.height() * (1f - scale) / 2f
            RectF(iconRect).apply { inset(insetX, insetY) }
        }
        annotationIconRenderer.draw(
            canvas = canvas,
            icon = icon,
            iconRect = drawRect,
            number = index + 1,
            alpha = (235 * progress).toInt().coerceIn(0, 235)
        )
    }

    private fun startIconAppearAnimation(index: Int) {
        iconAppearAnimator?.cancel()
        appearingIndex = index
        appearingProgress = 0f
        iconAppearAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 150L
            addUpdateListener { animator ->
                appearingProgress = animator.animatedValue as Float
                invalidate()
            }
            start()
        }
    }

    private fun findAnnotationAt(x: Float, y: Float): Int? {
        val imageRect = displayedImageRect() ?: return null
        for (index in annotations.lastIndex downTo 0) {
            if (index == expandedIndex) continue
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

    override fun onDetachedFromWindow() {
        iconAppearAnimator?.cancel()
        iconAppearAnimator = null
        super.onDetachedFromWindow()
    }
}
