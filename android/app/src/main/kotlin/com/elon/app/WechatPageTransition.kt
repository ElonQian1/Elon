package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.AnimatorSet
import android.animation.ObjectAnimator
import android.view.View
import android.view.animation.PathInterpolator

internal object WechatPageTransition {
    private const val DURATION_MS = 260L
    private const val UNDER_PAGE_SHIFT = 0.28f
    private val interpolator = PathInterpolator(0.2f, 0f, 0f, 1f)

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
                    onEnd()
                }
            )
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
        AnimatorSet().apply {
            duration = DURATION_MS
            interpolator = WechatPageTransition.interpolator
            playTogether(animators)
            addListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    onEnd()
                }
            })
            start()
        }
    }

    private fun slide(view: View, from: Float, to: Float): Animator {
        return ObjectAnimator.ofFloat(view, View.TRANSLATION_X, from, to)
    }
}
