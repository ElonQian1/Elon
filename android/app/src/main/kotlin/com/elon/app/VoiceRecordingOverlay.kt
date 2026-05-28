package com.elon.app

import android.app.Activity
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView

/**
 * 仿微信"按住说话"的全屏录音遮罩。
 *
 * 三个区域，由触摸的 Y 位移决定（DOWN 时设零点）：
 *   - SEND：正常按住区域，松开 → 直接把转写文字作为消息发送
 *   - TRANSLATE：手指上滑 >= [DRAG_TRANSLATE_DY] 像素，松开 → 把转写文字回填到输入框给用户查看/编辑
 *   - CANCEL：手指继续上滑 >= [DRAG_CANCEL_DY] 像素，松开 → 取消并丢弃
 *
 * 仅渲染 UI，不持有 ASR 状态；由 [MainSpeechInputActions] 通过 [updatePartial] / [updateZone] 推送。
 */
internal class VoiceRecordingOverlay(private val activity: Activity) {

    enum class Zone { SEND, TRANSLATE, CANCEL }

    private val main = Handler(Looper.getMainLooper())
    private var root: FrameLayout? = null
    private var card: LinearLayout? = null
    private var iconView: ImageView? = null
    private var durationView: TextView? = null
    private var partialView: TextView? = null
    private var hintView: TextView? = null
    private var startedAt: Long = 0L
    private var zone: Zone = Zone.SEND
    private val durationTick = object : Runnable {
        override fun run() {
            val ms = SystemClock.elapsedRealtime() - startedAt
            durationView?.text = formatDuration(ms)
            main.postDelayed(this, 200L)
        }
    }

    val isShowing: Boolean get() = root != null

    fun show() {
        if (root != null) return
        val parent = activity.window.decorView as? ViewGroup ?: return

        val overlay = FrameLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            )
            setBackgroundColor(Color.parseColor("#88000000"))
            // 透明传递触摸：触摸事件仍由底下的"按住说话"按钮处理
            isClickable = false
            isFocusable = false
        }
        val container = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER_HORIZONTAL
            val pad = dp(20)
            setPadding(pad, pad, pad, pad)
            background = GradientDrawable().apply {
                setColor(Color.parseColor("#262626"))
                cornerRadius = dp(16).toFloat()
            }
            val lp = FrameLayout.LayoutParams(
                dp(280),
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { gravity = Gravity.CENTER }
            layoutParams = lp
        }
        val icon = ImageView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(56), dp(56)).apply {
                bottomMargin = dp(12)
            }
            setImageResource(android.R.drawable.ic_btn_speak_now)
            setColorFilter(Color.WHITE)
        }
        val duration = TextView(activity).apply {
            textSize = 14f
            setTextColor(Color.parseColor("#E0E0E0"))
            text = "0.0\""
            gravity = Gravity.CENTER
            val lp = LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { bottomMargin = dp(10) }
            layoutParams = lp
        }
        val partial = TextView(activity).apply {
            textSize = 14f
            setTextColor(Color.WHITE)
            gravity = Gravity.CENTER
            maxLines = 4
            text = "正在听…"
            val lp = LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT
            ).apply { bottomMargin = dp(14) }
            layoutParams = lp
        }
        val hint = TextView(activity).apply {
            textSize = 12f
            setTextColor(Color.parseColor("#C0C0C0"))
            gravity = Gravity.CENTER
            text = "松开发送 · 上滑查看文字 · 继续上滑取消"
        }
        container.addView(icon)
        container.addView(duration)
        container.addView(partial)
        container.addView(hint)
        overlay.addView(container)
        parent.addView(overlay)

        root = overlay
        card = container
        iconView = icon
        durationView = duration
        partialView = partial
        hintView = hint
        startedAt = SystemClock.elapsedRealtime()
        zone = Zone.SEND
        applyZone()
        main.post(durationTick)
    }

    /** 实时识别文字（部分结果） */
    fun updatePartial(text: String) {
        partialView?.text = if (text.isBlank()) "正在听…" else text
    }

    /** 根据手指 Y 位移更新区域 */
    fun updateZone(dyUp: Float) {
        val newZone = when {
            dyUp >= DRAG_CANCEL_DY -> Zone.CANCEL
            dyUp >= DRAG_TRANSLATE_DY -> Zone.TRANSLATE
            else -> Zone.SEND
        }
        if (newZone != zone) {
            zone = newZone
            applyZone()
        }
    }

    val currentZone: Zone get() = zone

    fun hide() {
        main.removeCallbacks(durationTick)
        root?.let { (it.parent as? ViewGroup)?.removeView(it) }
        root = null
        card = null
        iconView = null
        durationView = null
        partialView = null
        hintView = null
    }

    private fun applyZone() {
        when (zone) {
            Zone.SEND -> {
                iconView?.setColorFilter(Color.WHITE)
                hintView?.text = "松开发送 · 上滑查看文字 · 继续上滑取消"
                hintView?.setTextColor(Color.parseColor("#C0C0C0"))
            }
            Zone.TRANSLATE -> {
                iconView?.setColorFilter(Color.parseColor("#4FC3F7"))
                hintView?.text = "松开 → 查看转写文字（不直接发送）"
                hintView?.setTextColor(Color.parseColor("#4FC3F7"))
            }
            Zone.CANCEL -> {
                iconView?.setColorFilter(Color.parseColor("#FF6B6B"))
                hintView?.text = "松开 → 取消本次录音"
                hintView?.setTextColor(Color.parseColor("#FF6B6B"))
            }
        }
    }

    private fun formatDuration(ms: Long): String {
        val sec = ms / 1000.0
        return String.format("%.1f\"", sec)
    }

    private fun dp(v: Int): Int =
        (v * activity.resources.displayMetrics.density).toInt()

    private companion object {
        // Y 像素阈值，DOWN 时为零点，向上为正
        const val DRAG_TRANSLATE_DY = 120f
        const val DRAG_CANCEL_DY = 260f
    }
}
