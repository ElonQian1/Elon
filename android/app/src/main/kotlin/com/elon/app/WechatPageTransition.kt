package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.AnimatorSet
import android.animation.ObjectAnimator
import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.View
import android.view.ViewGroup
import android.view.animation.PathInterpolator
import android.widget.ImageView

internal object WechatPageTransition {
    private const val DURATION_MS = 260L
    private const val UNDER_PAGE_SHIFT = 0.28f
    private val interpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
    private var activeAnimator: AnimatorSet? = null

    fun cancelActive() {
        activeAnimator?.cancel()
        activeAnimator = null
    }

    fun enterFromRight(
        container: View,
        incoming: List<View>,
        outgoing: List<View>,
        incomingFull: List<View> = emptyList(),
        outgoingFull: List<View> = emptyList(),
        onEnd: () -> Unit
    ) {
        runWhenMeasured(container) {
            val width = container.width.toFloat()
            incoming.forEach {
                it.visibility = View.VISIBLE
                it.translationX = width
            }
            incomingFull.forEach {
                it.visibility = View.VISIBLE
                it.translationX = width
            }
            outgoing.forEach {
                it.visibility = View.VISIBLE
                it.translationX = 0f
            }
            outgoingFull.forEach {
                it.visibility = View.VISIBLE
                it.translationX = 0f
            }
            bringToFront(incoming + incomingFull)
            play(
                animators = incoming.map { slide(it, width, 0f) } +
                    outgoing.map { slide(it, 0f, -width * UNDER_PAGE_SHIFT) } +
                    incomingFull.map { slide(it, width, 0f) } +
                    outgoingFull.map { slide(it, 0f, -width) },
                onEnd = {
                    incoming.forEach { it.translationX = 0f }
                    incomingFull.forEach { it.translationX = 0f }
                    outgoing.forEach { it.translationX = 0f }
                    outgoingFull.forEach { it.translationX = 0f }
                    bringToFront(incoming + incomingFull)
                    onEnd()
                }
            )
        }
    }

    fun exitToRight(
        container: View,
        outgoing: List<View>,
        incoming: List<View>,
        outgoingFull: List<View> = emptyList(),
        incomingFull: List<View> = emptyList(),
        onEnd: () -> Unit
    ) {
        runWhenMeasured(container) {
            val width = container.width.toFloat()
            incoming.forEach {
                it.visibility = View.VISIBLE
                it.translationX = -width * UNDER_PAGE_SHIFT
            }
            incomingFull.forEach {
                it.visibility = View.VISIBLE
                it.translationX = -width
            }
            outgoing.forEach {
                it.visibility = View.VISIBLE
                it.translationX = 0f
            }
            outgoingFull.forEach {
                it.visibility = View.VISIBLE
                it.translationX = 0f
            }
            bringToFront(outgoing + outgoingFull)
            play(
                animators = outgoing.map { slide(it, 0f, width) } +
                    incoming.map { slide(it, -width * UNDER_PAGE_SHIFT, 0f) } +
                    outgoingFull.map { slide(it, 0f, width) } +
                    incomingFull.map { slide(it, -width, 0f) },
                onEnd = {
                    outgoing.forEach { it.translationX = 0f }
                    outgoingFull.forEach { it.translationX = 0f }
                    incoming.forEach { it.translationX = 0f }
                    incomingFull.forEach { it.translationX = 0f }
                    bringToFront(incoming + incomingFull)
                    onEnd()
                }
            )
        }
    }

    fun enterFromLeft(
        container: View,
        incoming: List<View>,
        outgoing: List<View>,
        incomingFull: List<View> = emptyList(),
        outgoingFull: List<View> = emptyList(),
        onEnd: () -> Unit
    ) {
        runWhenMeasured(container) {
            val width = container.width.toFloat()
            incoming.forEach {
                it.visibility = View.VISIBLE
                it.translationX = -width
            }
            incomingFull.forEach {
                it.visibility = View.VISIBLE
                it.translationX = -width
            }
            outgoing.forEach {
                it.visibility = View.VISIBLE
                it.translationX = 0f
            }
            outgoingFull.forEach {
                it.visibility = View.VISIBLE
                it.translationX = 0f
            }
            bringToFront(incoming + incomingFull)
            play(
                animators = incoming.map { slide(it, -width, 0f) } +
                    outgoing.map { slide(it, 0f, width * UNDER_PAGE_SHIFT) } +
                    incomingFull.map { slide(it, -width, 0f) } +
                    outgoingFull.map { slide(it, 0f, width) },
                onEnd = {
                    incoming.forEach { it.translationX = 0f }
                    incomingFull.forEach { it.translationX = 0f }
                    outgoing.forEach { it.translationX = 0f }
                    outgoingFull.forEach { it.translationX = 0f }
                    bringToFront(incoming + incomingFull)
                    onEnd()
                }
            )
        }
    }

    fun exitToLeft(
        container: View,
        outgoing: List<View>,
        incoming: List<View>,
        outgoingFull: List<View> = emptyList(),
        incomingFull: List<View> = emptyList(),
        onEnd: () -> Unit
    ) {
        runWhenMeasured(container) {
            val width = container.width.toFloat()
            incoming.forEach {
                it.visibility = View.VISIBLE
                it.translationX = width * UNDER_PAGE_SHIFT
            }
            incomingFull.forEach {
                it.visibility = View.VISIBLE
                it.translationX = width
            }
            outgoing.forEach {
                it.visibility = View.VISIBLE
                it.translationX = 0f
            }
            outgoingFull.forEach {
                it.visibility = View.VISIBLE
                it.translationX = 0f
            }
            bringToFront(outgoing + outgoingFull)
            play(
                animators = outgoing.map { slide(it, 0f, -width) } +
                    incoming.map { slide(it, width * UNDER_PAGE_SHIFT, 0f) } +
                    outgoingFull.map { slide(it, 0f, -width) } +
                    incomingFull.map { slide(it, width, 0f) },
                onEnd = {
                    outgoing.forEach { it.translationX = 0f }
                    outgoingFull.forEach { it.translationX = 0f }
                    incoming.forEach { it.translationX = 0f }
                    incomingFull.forEach { it.translationX = 0f }
                    bringToFront(incoming + incomingFull)
                    onEnd()
                }
            )
        }
    }

    fun replaceContentFromRight(
        container: View,
        page: View,
        updateContent: () -> Unit,
        onEnd: () -> Unit
    ) {
        replaceContent(
            container = container,
            page = page,
            incomingFrom = Direction.RIGHT,
            updateContent = updateContent,
            onEnd = onEnd
        )
    }

    fun replaceContentToRight(
        container: View,
        page: View,
        updateContent: () -> Unit,
        onEnd: () -> Unit
    ) {
        replaceContent(
            container = container,
            page = page,
            incomingFrom = Direction.LEFT,
            updateContent = updateContent,
            onEnd = onEnd
        )
    }

    private enum class Direction {
        LEFT,
        RIGHT
    }

    private fun replaceContent(
        container: View,
        page: View,
        incomingFrom: Direction,
        updateContent: () -> Unit,
        onEnd: () -> Unit
    ) {
        runWhenMeasured(container) {
            val parent = container as? ViewGroup
            val overlay = parent?.let { capturePageOverlay(page) }
            updateContent()
            page.visibility = View.VISIBLE
            if (overlay == null || container.width <= 0) {
                page.translationX = 0f
                onEnd()
                return@runWhenMeasured
            }

            val width = container.width.toFloat()
            val incomingStart = if (incomingFrom == Direction.RIGHT) width else -width * UNDER_PAGE_SHIFT
            val overlayEnd = if (incomingFrom == Direction.RIGHT) -width * UNDER_PAGE_SHIFT else width
            page.translationX = incomingStart
            overlay.translationX = 0f

            parent.addView(overlay)
            if (incomingFrom == Direction.RIGHT) page.bringToFront() else overlay.bringToFront()
            parent.invalidate()
            play(
                animators = listOf(
                    slide(page, incomingStart, 0f),
                    slide(overlay, 0f, overlayEnd)
                ),
                onEnd = {
                    page.translationX = 0f
                    parent.removeView(overlay)
                    (overlay.drawable as? android.graphics.drawable.BitmapDrawable)?.bitmap?.recycle()
                    page.bringToFront()
                    onEnd()
                }
            )
        }
    }

    private fun capturePageOverlay(page: View): ImageView? {
        val width = page.width
        val height = page.height
        if (width <= 0 || height <= 0) return null
        val bitmap = runCatching {
            Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888).also { bitmap ->
                page.draw(Canvas(bitmap))
            }
        }.getOrNull() ?: return null
        return ImageView(page.context).apply {
            setImageBitmap(bitmap)
            scaleType = ImageView.ScaleType.FIT_XY
            layoutParams = ViewGroup.LayoutParams(width, height)
            x = page.x
            y = page.y
            elevation = page.elevation + 1f
            visibility = View.VISIBLE
        }
    }

    private fun runWhenMeasured(view: View, block: () -> Unit) {
        if (view.width > 0) {
            block()
        } else {
            view.post(block)
        }
    }

    private fun play(animators: List<Animator>, onEnd: () -> Unit) {
        cancelActive()
        AnimatorSet().apply {
            activeAnimator = this
            duration = DURATION_MS
            interpolator = WechatPageTransition.interpolator
            playTogether(animators)
            addListener(object : AnimatorListenerAdapter() {
                private var cancelled = false

                override fun onAnimationCancel(animation: Animator) {
                    cancelled = true
                    if (activeAnimator === animation) activeAnimator = null
                }

                override fun onAnimationEnd(animation: Animator) {
                    if (activeAnimator === animation) activeAnimator = null
                    if (!cancelled) onEnd()
                }
            })
            start()
        }
    }

    private fun slide(view: View, from: Float, to: Float): Animator {
        return ObjectAnimator.ofFloat(view, View.TRANSLATION_X, from, to)
    }

    private fun bringToFront(views: List<View>) {
        views.forEach { view ->
            view.bringToFront()
            (view.parent as? View)?.invalidate()
        }
    }
}
