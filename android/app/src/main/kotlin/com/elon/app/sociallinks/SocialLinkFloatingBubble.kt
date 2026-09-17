package com.elon.app.sociallinks

import android.app.Activity
import android.content.Context
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import kotlin.math.abs

/**
 * In-app floating bubble (no system overlay permission) shown over the chat while reading
 * sessions are minimised. Tap reopens the latest session, long press closes them all.
 */
internal class SocialLinkFloatingBubble private constructor(private val activity: Activity) {
    private val prefs = activity.getSharedPreferences("social-link-bubble", Context.MODE_PRIVATE)
    private val host = activity.window.decorView as ViewGroup
    private fun dp(n: Int) = (n * activity.resources.displayMetrics.density).toInt()
    private val size = dp(56)
    private val badge = TextView(activity).apply {
        textSize = 15f; gravity = Gravity.CENTER; setTextColor(Color.parseColor("#F8F7F4")); includeFontPadding = false
    }
    private val count = TextView(activity).apply {
        textSize = 10f; gravity = Gravity.CENTER; setTextColor(Color.BLACK); includeFontPadding = false
        background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(Color.WHITE) }
    }
    private val spinner = ProgressBar(activity).apply { isIndeterminate = true; visibility = View.GONE }
    val root = FrameLayout(activity).apply {
        tag = "social-link-bubble"; visibility = View.GONE; elevation = dp(40).toFloat()
        background = GradientDrawable().apply { shape = GradientDrawable.OVAL; setColor(Color.parseColor("#1F2023")); setStroke(dp(1), Color.parseColor("#3A3B40")) }
        addView(spinner, FrameLayout.LayoutParams(size, size))
        addView(badge, FrameLayout.LayoutParams(size, size))
        addView(count, FrameLayout.LayoutParams(dp(16), dp(16), Gravity.END or Gravity.TOP))
        isClickable = true; isFocusable = true
    }
    private val listener: () -> Unit = { refresh() }
    private val slop = ViewConfiguration.get(activity).scaledTouchSlop
    private var downX = 0f; private var downY = 0f; private var startX = 0f; private var startY = 0f; private var dragging = false
    private var longPressed = false
    private val longPress = Runnable {
        longPressed = true
        SocialLinkReaderSessions.destroyAll()
        Toast.makeText(activity, "已关闭阅读浮窗", Toast.LENGTH_SHORT).show()
    }

    init {
        host.addView(root, FrameLayout.LayoutParams(size, size, Gravity.TOP or Gravity.START))
        root.setOnTouchListener { view, event -> touch(view, event) }
        root.setOnClickListener { open() }
        root.post { place(prefs.getFloat("x", -1f), prefs.getFloat("y", -1f)) }
        SocialLinkReaderSessions.addListener(listener)
        refresh()
    }

    private fun place(x: Float, y: Float) {
        val maxX = (host.width - size).coerceAtLeast(0).toFloat(); val maxY = (host.height - size - dp(96)).coerceAtLeast(0).toFloat()
        root.x = if (x < 0) maxX - dp(12) else x.coerceIn(0f, maxX)
        root.y = if (y < 0) maxY * 0.62f else y.coerceIn(dp(48).toFloat(), maxY)
    }

    private fun touch(view: View, event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                downX = event.rawX; downY = event.rawY; startX = view.x; startY = view.y; dragging = false; longPressed = false
                view.isPressed = true; view.postDelayed(longPress, ViewConfiguration.getLongPressTimeout().toLong()); return true
            }
            MotionEvent.ACTION_MOVE -> {
                val dx = event.rawX - downX; val dy = event.rawY - downY
                if (!dragging && (abs(dx) > slop || abs(dy) > slop)) { dragging = true; view.removeCallbacks(longPress) }
                if (dragging) place(startX + dx, startY + dy)
                return true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                view.isPressed = false; view.removeCallbacks(longPress)
                if (dragging) {
                    // Snap to the nearest horizontal edge like WeChat's 浮窗.
                    val maxX = (host.width - size).toFloat()
                    view.animate().x(if (view.x + size / 2f < host.width / 2f) dp(12).toFloat() else maxX - dp(12)).setDuration(160).withEndAction { remember() }.start()
                } else if (!longPressed && event.actionMasked == MotionEvent.ACTION_UP) view.performClick()
                return true
            }
        }
        return false
    }

    private fun remember() { prefs.edit().putFloat("x", root.x).putFloat("y", root.y).apply() }

    private fun open() {
        val session = SocialLinkReaderSessions.minimized().firstOrNull() ?: return
        SocialLinkBrowserActivity.open(activity, session.link)
    }

    fun refresh() {
        val minimized = SocialLinkReaderSessions.minimized()
        if (minimized.isEmpty()) { root.visibility = View.GONE; return }
        val latest = minimized.first()
        badge.text = SocialLinkPresentation.badge(latest.link.site)
        spinner.visibility = if (latest.loading) View.VISIBLE else View.GONE
        badge.alpha = if (latest.loading) 0.55f else 1f
        count.text = minimized.size.toString(); count.visibility = if (minimized.size > 1) View.VISIBLE else View.GONE
        root.contentDescription = "继续阅读 ${latest.title}${if (minimized.size > 1) "，共 ${minimized.size} 篇" else ""}，长按关闭"
        root.visibility = View.VISIBLE
    }

    fun dispose() { SocialLinkReaderSessions.removeListener(listener); host.removeView(root) }

    companion object {
        /** Attach once per chat Activity; the bubble follows minimised sessions automatically. */
        fun install(activity: Activity): SocialLinkFloatingBubble = SocialLinkFloatingBubble(activity)

        /** Installs on every chat screen without touching the (oversized) MainActivity itself. */
        fun installForChat(app: android.app.Application, chatActivity: Class<out Activity>) {
            val bubbles = HashMap<Activity, SocialLinkFloatingBubble>()
            app.registerActivityLifecycleCallbacks(object : android.app.Application.ActivityLifecycleCallbacks {
                override fun onActivityResumed(activity: Activity) {
                    if (chatActivity.isInstance(activity) && activity !in bubbles) bubbles[activity] = install(activity)
                    bubbles[activity]?.refresh()
                }
                override fun onActivityDestroyed(activity: Activity) { bubbles.remove(activity)?.dispose() }
                override fun onActivityCreated(activity: Activity, savedInstanceState: android.os.Bundle?) {}
                override fun onActivityStarted(activity: Activity) {}
                override fun onActivityPaused(activity: Activity) {}
                override fun onActivityStopped(activity: Activity) {}
                override fun onActivitySaveInstanceState(activity: Activity, outState: android.os.Bundle) {}
            })
        }
    }
}
