package com.elon.app

import android.animation.ValueAnimator
import android.content.Context
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.HorizontalScrollView
import kotlin.math.abs
import kotlin.math.max

internal const val PROJECT_PLAZA_CARD_MIN_SCALE = 0.90f

internal fun projectPlazaCardScale(
    centerDistancePx: Float,
    snapDistancePx: Float,
    minimumScale: Float = PROJECT_PLAZA_CARD_MIN_SCALE
): Float {
    val safeMinimum = minimumScale.coerceIn(0f, 1f)
    val normalizedDistance = if (snapDistancePx > 0f) {
        (abs(centerDistancePx) / snapDistancePx).coerceIn(0f, 1f)
    } else {
        if (centerDistancePx == 0f) 0f else 1f
    }
    return 1f - ((1f - safeMinimum) * normalizedDistance)
}

internal fun nearestProjectPlazaCardIndex(
    cardCentersPx: List<Float>,
    previewCenterPx: Float
): Int? = cardCentersPx.indices.minByOrNull { index ->
    abs(cardCentersPx[index] - previewCenterPx)
}

internal fun projectPlazaTrailingPadding(
    viewportWidthPx: Int,
    leadingPaddingPx: Int,
    cardWidthPx: Int,
    minimumTrailingPaddingPx: Int
): Int = max(
    minimumTrailingPaddingPx,
    viewportWidthPx - leadingPaddingPx - cardWidthPx
)

/**
 * Keeps fixed layout/snap slots while scaling each complete featured card around its visual center.
 * Fixed slots preserve the half-preview geometry and touch semantics; only drawing is transformed.
 */
internal class ProjectPlazaCarousel(context: Context) : HorizontalScrollView(context) {
    private var touchActive = false
    private var snapTargetX: Int? = null
    private var minimumTrailingPaddingPx = 0
    private val settleRunnable = Runnable { snapToNearestCard() }

    init {
        isHorizontalScrollBarEnabled = false
        overScrollMode = View.OVER_SCROLL_NEVER
        clipToPadding = false
    }

    fun configureContentInsets(leadingPaddingPx: Int, minimumTrailingPaddingPx: Int) {
        this.minimumTrailingPaddingPx = minimumTrailingPaddingPx
        setPadding(leadingPaddingPx, paddingTop, minimumTrailingPaddingPx, paddingBottom)
        post { updateTrailingPadding() }
    }

    fun refreshCardScales() {
        val row = cardRow() ?: return
        if (row.childCount == 0) return
        val first = row.getChildAt(0)
        if (first.width <= 0) return
        val previewCenter = first.left + first.width / 2f
        val snapDistance = if (row.childCount > 1) {
            (row.getChildAt(1).left - first.left).toFloat()
        } else {
            first.width.toFloat()
        }
        for (index in 0 until row.childCount) {
            val card = row.getChildAt(index)
            val cardCenter = card.left + card.width / 2f - scrollX
            val scale = projectPlazaCardScale(cardCenter - previewCenter, snapDistance)
            card.pivotX = card.width / 2f
            card.pivotY = card.height / 2f
            card.scaleX = scale
            card.scaleY = scale
        }
    }

    override fun onScrollChanged(left: Int, top: Int, oldLeft: Int, oldTop: Int) {
        super.onScrollChanged(left, top, oldLeft, oldTop)
        refreshCardScales()
        if (!touchActive) scheduleSettle()
    }

    override fun onSizeChanged(width: Int, height: Int, oldWidth: Int, oldHeight: Int) {
        super.onSizeChanged(width, height, oldWidth, oldHeight)
        post {
            updateTrailingPadding()
            refreshCardScales()
        }
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                touchActive = true
                snapTargetX = null
                removeCallbacks(settleRunnable)
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                touchActive = false
                scheduleSettle()
            }
        }
        return super.onTouchEvent(event)
    }

    override fun onDetachedFromWindow() {
        removeCallbacks(settleRunnable)
        super.onDetachedFromWindow()
    }

    private fun scheduleSettle() {
        removeCallbacks(settleRunnable)
        postDelayed(settleRunnable, SETTLE_DELAY_MS)
    }

    private fun snapToNearestCard() {
        val row = cardRow() ?: return
        if (touchActive || row.childCount == 0) return
        val first = row.getChildAt(0)
        val previewCenter = first.left + first.width / 2f
        val centers = (0 until row.childCount).map { index ->
            val card = row.getChildAt(index)
            card.left + card.width / 2f - scrollX
        }
        val nearestIndex = nearestProjectPlazaCardIndex(centers, previewCenter) ?: return
        val targetX = (row.getChildAt(nearestIndex).left - first.left).coerceAtLeast(0)
        if (abs(scrollX - targetX) <= SNAP_EPSILON_PX) {
            if (scrollX != targetX) scrollTo(targetX, 0)
            snapTargetX = null
            refreshCardScales()
            return
        }
        if (snapTargetX == targetX) return
        snapTargetX = targetX
        if (ValueAnimator.areAnimatorsEnabled()) {
            smoothScrollTo(targetX, 0)
        } else {
            scrollTo(targetX, 0)
            snapTargetX = null
            refreshCardScales()
        }
    }

    private fun cardRow(): ViewGroup? = getChildAt(0) as? ViewGroup

    private fun updateTrailingPadding() {
        val firstCard = cardRow()?.takeIf { it.childCount > 0 }?.getChildAt(0) ?: return
        if (firstCard.width <= 0 || width <= 0) return
        val trailingPadding = projectPlazaTrailingPadding(
            width,
            paddingLeft,
            firstCard.width,
            minimumTrailingPaddingPx
        )
        if (paddingRight != trailingPadding) {
            setPadding(paddingLeft, paddingTop, trailingPadding, paddingBottom)
        }
    }

    private companion object {
        const val SETTLE_DELAY_MS = 96L
        const val SNAP_EPSILON_PX = 1
    }
}
