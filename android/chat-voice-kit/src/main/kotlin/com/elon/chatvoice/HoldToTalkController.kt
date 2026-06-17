package com.elon.chatvoice

import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import kotlin.math.abs

class HoldToTalkController(
    private val callbacks: Callbacks,
    private val cancelDistancePx: Float? = null,
) {
    interface Callbacks {
        fun onHoldStart() {}
        fun onCancelZoneChanged(inCancelZone: Boolean) {}
        fun onHoldRelease() {}
        fun onHoldCancel() {}
    }

    private var downY = 0f
    private var tracking = false
    private var canceling = false

    fun attachTo(view: View) {
        val threshold = cancelDistancePx ?: ViewConfiguration.get(view.context).scaledTouchSlop * 8f
        view.setOnTouchListener { _, event -> handleTouch(event, threshold) }
    }

    fun handleTouch(event: MotionEvent, threshold: Float): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                downY = event.rawY
                tracking = true
                canceling = false
                callbacks.onHoldStart()
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                if (!tracking) return false
                val nextCanceling = downY - event.rawY > threshold && abs(event.rawY - downY) > threshold
                if (nextCanceling != canceling) {
                    canceling = nextCanceling
                    callbacks.onCancelZoneChanged(canceling)
                }
                return true
            }
            MotionEvent.ACTION_UP -> {
                if (!tracking) return false
                val shouldCancel = canceling
                reset()
                if (shouldCancel) callbacks.onHoldCancel() else callbacks.onHoldRelease()
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                if (!tracking) return false
                reset()
                callbacks.onHoldCancel()
                return true
            }
        }
        return false
    }

    fun cancelActiveHold() {
        if (!tracking) return
        reset()
        callbacks.onHoldCancel()
    }

    private fun reset() {
        tracking = false
        if (canceling) callbacks.onCancelZoneChanged(false)
        canceling = false
    }
}
