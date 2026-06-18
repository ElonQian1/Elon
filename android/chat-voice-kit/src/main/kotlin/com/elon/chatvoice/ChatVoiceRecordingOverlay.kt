package com.elon.chatvoice

import android.content.Context
import android.graphics.Color
import android.text.SpannableStringBuilder
import android.text.Spanned
import android.text.style.ForegroundColorSpan
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ScrollView
import android.widget.TextView

class ChatVoiceRecordingOverlay(
    private val context: Context,
    private var config: VoiceComposerConfig = VoiceComposerConfig(),
) {
    private var root: FrameLayout? = null
    private var bubbleView: ChatVoiceWaveBubbleView? = null
    private var partialScroll: ScrollView? = null
    private var partialView: TextView? = null
    private var trayView: ChatVoiceActionTrayView? = null
    private var partialText = ""
    private var historyText = ""
    private var state = ChatVoiceListeningState.PREPARING
    private var zone: ChatVoiceZone = ChatVoiceInteractionContract.defaultZone(config.chatMode)
    private var initialTouchRawY: Float? = null

    val isShowing: Boolean get() = root != null
    val currentZone: ChatVoiceZone get() = zone

    fun applyConfig(next: VoiceComposerConfig) {
        config = next
        zone = ChatVoiceInteractionContract.defaultZone(next.chatMode)
        trayView?.mode = next.chatMode
        render()
    }

    fun show(parent: ViewGroup) {
        if (root != null) return
        val overlay = FrameLayout(context).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            )
            setBackgroundColor(Color.parseColor(ChatVoiceInteractionContract.tokens.overlayScrim))
            isClickable = false
            isFocusable = false
        }
        val bubble = ChatVoiceWaveBubbleView(context).apply {
            layoutParams = FrameLayout.LayoutParams(dp(192), dp(88)).apply {
                gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
                bottomMargin = dp(240)
            }
        }
        val text = TextView(context).apply {
            gravity = Gravity.CENTER
            includeFontPadding = false
            textSize = 15f
            setLineSpacing(0f, 1.25f)
            setTextColor(Color.parseColor(ChatVoiceInteractionContract.tokens.textDefault))
        }
        val partial = ScrollView(context).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(112),
            ).apply {
                gravity = Gravity.BOTTOM or Gravity.CENTER_HORIZONTAL
                leftMargin = dp(36)
                rightMargin = dp(36)
                bottomMargin = dp(206)
            }
            isVerticalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_NEVER
            addView(text)
        }
        val tray = ChatVoiceActionTrayView(context, config.chatMode).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(194),
                Gravity.BOTTOM,
            )
        }
        overlay.addView(bubble)
        overlay.addView(partial)
        overlay.addView(tray)
        parent.addView(overlay)
        root = overlay
        bubbleView = bubble
        partialScroll = partial
        partialView = text
        trayView = tray
        partialText = ""
        historyText = ""
        state = ChatVoiceListeningState.PREPARING
        initialTouchRawY = null
        zone = ChatVoiceInteractionContract.defaultZone(config.chatMode)
        render()
    }

    fun updateTouch(rawX: Float, rawY: Float): ChatVoiceZone {
        val overlay = root ?: return zone
        val startY = initialTouchRawY ?: rawY.also { initialTouchRawY = it }
        val location = IntArray(2)
        overlay.getLocationOnScreen(location)
        val nextZone = ChatVoiceInteractionContract.zoneFromOverlayTouch(
            mode = config.chatMode,
            localX = rawX - location[0],
            localY = rawY - location[1],
            widthPx = overlay.width,
            heightPx = overlay.height,
            initialRawY = startY,
            currentRawY = rawY,
            density = overlay.resources.displayMetrics.density,
            options = config.holdOptions,
        )
        setZone(nextZone)
        return zone
    }

    fun updatePartial(text: String) {
        partialText = text.trim()
        render()
    }

    fun appendHistory(text: String) {
        val clean = text.trim()
        if (clean.isBlank()) return
        historyText = if (historyText.isBlank()) clean else "$historyText\n$clean"
        partialText = ""
        render()
    }

    fun setListeningState(next: ChatVoiceListeningState) {
        state = next
        if (next == ChatVoiceListeningState.HEARD) bubbleView?.playHeardAnimation()
        render()
    }

    fun setVolume(value: Float) {
        bubbleView?.setVolume(value)
    }

    fun hide() {
        bubbleView?.stopCountdown()
        root?.let { (it.parent as? ViewGroup)?.removeView(it) }
        root = null
        bubbleView = null
        partialScroll = null
        partialView = null
        trayView = null
        partialText = ""
        historyText = ""
        initialTouchRawY = null
        zone = ChatVoiceInteractionContract.defaultZone(config.chatMode)
    }

    private fun setZone(next: ChatVoiceZone) {
        if (next == zone) return
        zone = next
        render()
    }

    private fun render() {
        trayView?.zone = zone
        bubbleView?.isCanceling = zone == ChatVoiceZone.CANCEL
        partialView?.setTextColor(textColorForZone(zone))
        partialView?.text = renderText()
        partialScroll?.post { partialScroll?.fullScroll(View.FOCUS_DOWN) }
    }

    private fun renderText(): CharSequence {
        if (historyText.isNotBlank()) {
            val builder = SpannableStringBuilder(historyText)
            if (partialText.isNotBlank()) builder.append("\n").append(partialText)
            builder.setSpan(
                ForegroundColorSpan(Color.parseColor("#99EDEDED")),
                0,
                historyText.length,
                Spanned.SPAN_EXCLUSIVE_EXCLUSIVE,
            )
            return builder
        }
        if (partialText.isNotBlank()) return partialText
        val defaultZone = ChatVoiceInteractionContract.defaultZone(config.chatMode)
        return if (zone == defaultZone) {
            ChatVoiceInteractionContract.stateText(state)
        } else {
            ChatVoiceInteractionContract.releaseText(config.chatMode, zone)
        }
    }

    private fun textColorForZone(next: ChatVoiceZone): Int =
        Color.parseColor(
            when (next) {
                ChatVoiceZone.TRANSCRIBE -> ChatVoiceInteractionContract.tokens.textTranscribe
                ChatVoiceZone.CANCEL -> ChatVoiceInteractionContract.tokens.textCancel
                else -> ChatVoiceInteractionContract.tokens.textDefault
            }
        )

    private fun dp(value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()
}
