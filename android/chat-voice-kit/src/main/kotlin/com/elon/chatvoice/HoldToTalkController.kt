package com.elon.chatvoice

import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.os.Handler
import android.os.Looper
import kotlin.math.abs

class HoldToTalkController(
    private val callbacks: Callbacks,
    private val cancelDistancePx: Float? = null,
    private val options: ChatVoiceHoldOptions = ChatVoiceInteractionContract.holdOptions,
    private val eventSink: ChatVoiceEventSink? = null,
    private val mode: ChatVoiceMode = ChatVoiceMode.FRIEND_CHAT,
) {
    interface Callbacks {
        fun onHoldPending() {}
        fun onHoldStart() {}
        fun onCancelZoneChanged(inCancelZone: Boolean) {}
        fun onHoldRelease() {}
        fun onHoldCancel() {}
    }

    private val main = Handler(Looper.getMainLooper())
    private var downY = 0f
    private var tracking = false
    private var started = false
    private var canceling = false
    private val startRunnable = Runnable { startHoldIfTracking() }

    fun attachTo(view: View) {
        val density = view.resources.displayMetrics.density
        val threshold = cancelDistancePx
            ?: (options.cancelDragUpDp * density).coerceAtLeast(ViewConfiguration.get(view.context).scaledTouchSlop * 2f)
        view.setOnTouchListener { _, event -> handleTouch(event, threshold) }
    }

    fun handleTouch(event: MotionEvent, threshold: Float): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                downY = event.rawY
                tracking = true
                started = false
                canceling = false
                callbacks.onHoldPending()
                if (options.longPressStartDelayMs <= 0L) {
                    startHoldIfTracking()
                } else {
                    main.postDelayed(startRunnable, options.longPressStartDelayMs)
                }
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                if (!tracking || !started) return true
                val nextCanceling = downY - event.rawY > threshold && abs(event.rawY - downY) > threshold
                if (nextCanceling != canceling) {
                    canceling = nextCanceling
                    val zone = if (canceling) ChatVoiceZone.CANCEL else ChatVoiceInteractionContract.defaultZone(mode)
                    callbacks.onCancelZoneChanged(canceling)
                    eventSink?.onVoiceEvent(
                        ChatVoiceEvent.ZoneChanged(
                            zone,
                            ChatVoiceInteractionContract.releaseText(mode, zone),
                        )
                    )
                }
                return true
            }
            MotionEvent.ACTION_UP -> {
                if (!tracking) return false
                val shouldCancel = canceling
                val wasStarted = started
                reset()
                if (!wasStarted) return true
                if (shouldCancel) {
                    eventSink?.onVoiceEvent(ChatVoiceEvent.Cancel)
                    callbacks.onHoldCancel()
                } else {
                    callbacks.onHoldRelease()
                }
                return true
            }
            MotionEvent.ACTION_CANCEL -> {
                if (!tracking) return false
                reset()
                eventSink?.onVoiceEvent(ChatVoiceEvent.Cancel)
                callbacks.onHoldCancel()
                return true
            }
        }
        return false
    }

    fun cancelActiveHold() {
        if (!tracking) return
        reset()
        eventSink?.onVoiceEvent(ChatVoiceEvent.Cancel)
        callbacks.onHoldCancel()
    }

    private fun startHoldIfTracking() {
        if (!tracking || started) return
        started = true
        eventSink?.onVoiceEvent(ChatVoiceEvent.Start)
        callbacks.onHoldStart()
    }

    private fun reset() {
        main.removeCallbacks(startRunnable)
        tracking = false
        started = false
        if (canceling) callbacks.onCancelZoneChanged(false)
        canceling = false
    }
}
