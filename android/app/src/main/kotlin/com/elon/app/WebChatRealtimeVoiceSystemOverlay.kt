package com.elon.app

import android.content.Context
import android.graphics.Color
import android.graphics.PixelFormat
import android.graphics.drawable.GradientDrawable
import android.provider.Settings
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import kotlin.math.abs

internal class WebChatRealtimeVoiceSystemOverlay(
    private val context: Context,
    private val onPauseResume: () -> Unit,
    private val onOpenApp: () -> Unit,
    private val onHangUp: () -> Unit,
) {
    private val windowManager = context.getSystemService(WindowManager::class.java)
    private val touchSlop = ViewConfiguration.get(context).scaledTouchSlop
    private val root = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
    }
    private val orb = FrameLayout(context).apply {
        isClickable = true
        isFocusable = true
        contentDescription = "web-chat-realtime-voice:system-overlay"
    }
    private val icon = ImageView(context).apply {
        setImageResource(R.drawable.ic_input_voice)
        setColorFilter(Color.WHITE)
        scaleType = ImageView.ScaleType.CENTER
    }
    private val statusDot = View(context)
    private val panel = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(8), dp(4), dp(8), dp(4))
        visibility = View.GONE
        background = rounded(Color.argb(238, 28, 29, 33), dp(22).toFloat())
    }
    private val pauseResume = actionButton(android.R.drawable.ic_media_pause, "暂停实时语音") {
        onPauseResume()
    }
    private val openApp = actionButton(android.R.drawable.ic_menu_view, "返回一龙语音会话") {
        onOpenApp()
    }
    private val hangUp = actionButton(R.drawable.ic_voice_call_hangup, "挂断实时语音") {
        onHangUp()
    }
    private val params = WindowManager.LayoutParams().apply {
        width = WindowManager.LayoutParams.WRAP_CONTENT
        height = WindowManager.LayoutParams.WRAP_CONTENT
        type = WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
        flags = WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
            WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS
        format = PixelFormat.TRANSLUCENT
        gravity = Gravity.TOP or Gravity.START
        x = context.resources.displayMetrics.widthPixels - dp(76)
        y = context.resources.displayMetrics.heightPixels / 3
    }
    private var attached = false
    private var expanded = false
    private var paused = false
    private var downX = 0f
    private var downY = 0f
    private var startX = 0
    private var startY = 0
    private var dragging = false

    init {
        orb.addView(icon, FrameLayout.LayoutParams(dp(36), dp(36), Gravity.CENTER))
        orb.addView(
            statusDot,
            FrameLayout.LayoutParams(dp(12), dp(12), Gravity.END or Gravity.BOTTOM).apply {
                marginEnd = dp(4)
                bottomMargin = dp(4)
            },
        )
        panel.addView(pauseResume, LinearLayout.LayoutParams(dp(46), dp(46)))
        panel.addView(openApp, LinearLayout.LayoutParams(dp(46), dp(46)))
        panel.addView(hangUp, LinearLayout.LayoutParams(dp(46), dp(46)))
        root.addView(panel)
        // Keep the orb anchored while the action panel expands to its left.
        root.addView(orb, LinearLayout.LayoutParams(dp(56), dp(56)))
        installDragAndExpand()
        update(WebChatRealtimeVoiceBackgroundStatus.CONNECTING, "正在连接语音")
    }

    fun show() {
        if (attached || !Settings.canDrawOverlays(context)) return
        runCatching { windowManager.addView(root, params) }
            .onSuccess { attached = true }
    }

    fun hide() {
        if (!attached) return
        runCatching { windowManager.removeView(root) }
        attached = false
        setExpanded(false)
    }

    fun update(status: WebChatRealtimeVoiceBackgroundStatus, detail: String) {
        paused = status == WebChatRealtimeVoiceBackgroundStatus.PAUSED
        val color = when (status) {
            WebChatRealtimeVoiceBackgroundStatus.CONNECTING -> Color.rgb(122, 137, 165)
            WebChatRealtimeVoiceBackgroundStatus.LISTENING -> Color.rgb(47, 128, 237)
            WebChatRealtimeVoiceBackgroundStatus.THINKING -> Color.rgb(255, 179, 71)
            WebChatRealtimeVoiceBackgroundStatus.SPEAKING -> Color.rgb(50, 205, 120)
            WebChatRealtimeVoiceBackgroundStatus.PAUSED -> Color.rgb(132, 136, 145)
            WebChatRealtimeVoiceBackgroundStatus.ERROR -> Color.rgb(229, 76, 83)
        }
        orb.background = oval(color)
        statusDot.background = oval(if (paused) Color.LTGRAY else Color.WHITE)
        pauseResume.setImageResource(
            if (paused) android.R.drawable.ic_media_play else android.R.drawable.ic_media_pause,
        )
        pauseResume.contentDescription = if (paused) "继续实时语音" else "暂停实时语音"
        orb.contentDescription = "web-chat-realtime-voice:system-overlay:$detail"
    }

    private fun installDragAndExpand() {
        orb.setOnTouchListener { _, event ->
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    downX = event.rawX
                    downY = event.rawY
                    startX = params.x
                    startY = params.y
                    dragging = false
                    true
                }
                MotionEvent.ACTION_MOVE -> {
                    val dx = event.rawX - downX
                    val dy = event.rawY - downY
                    if (!dragging && (abs(dx) > touchSlop || abs(dy) > touchSlop)) dragging = true
                    if (dragging && attached) {
                        params.x = (startX + dx.toInt()).coerceAtLeast(0)
                        params.y = (startY + dy.toInt()).coerceAtLeast(0)
                        windowManager.updateViewLayout(root, params)
                    }
                    true
                }
                MotionEvent.ACTION_UP -> {
                    if (!dragging) setExpanded(!expanded)
                    true
                }
                else -> false
            }
        }
    }

    private fun setExpanded(value: Boolean) {
        if (expanded == value) return
        if (attached) {
            val panelWidth = dp(46 * 3 + 16)
            params.x = if (value) {
                (params.x - panelWidth).coerceAtLeast(0)
            } else {
                (params.x + panelWidth).coerceAtMost(
                    context.resources.displayMetrics.widthPixels - dp(56),
                )
            }
        }
        expanded = value
        panel.visibility = if (value) View.VISIBLE else View.GONE
        if (attached) windowManager.updateViewLayout(root, params)
    }

    private fun actionButton(iconRes: Int, label: String, action: () -> Unit) =
        ImageButton(context).apply {
            setImageResource(iconRes)
            setColorFilter(Color.WHITE)
            background = null
            contentDescription = label
            setOnClickListener { action() }
        }

    private fun oval(color: Int) = GradientDrawable().apply {
        shape = GradientDrawable.OVAL
        setColor(color)
        setStroke(dp(2), Color.argb(110, 255, 255, 255))
    }

    private fun rounded(color: Int, radius: Float) = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = radius
        setColor(color)
        setStroke(dp(1), Color.argb(72, 255, 255, 255))
    }

    private fun dp(value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()
}
