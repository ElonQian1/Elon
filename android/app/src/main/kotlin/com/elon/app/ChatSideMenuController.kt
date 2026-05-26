package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.ColorFilter
import android.graphics.LinearGradient
import android.graphics.Paint
import android.graphics.Path
import android.graphics.PixelFormat
import android.graphics.Shader
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.animation.PathInterpolator
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import kotlin.math.abs
import kotlin.math.min

internal class ChatSideMenuController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val activeConversation: () -> AppConversation,
    private val confirmLogout: () -> Unit,
    private val dismissActionPopup: () -> Unit,
    private val cancelChildTouch: (MotionEvent) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    private val interpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
    private lateinit var overlayHost: ViewGroup
    private lateinit var overlay: FrameLayout
    private lateinit var panel: FrameLayout
    private lateinit var settingsBubble: FrameLayout
    private val summaryRows = mutableListOf<TextView>()
    private var isSetup = false
    private var isAnimating = false
    private var touchTracking = false
    private var consumingGesture = false
    private var startRawX = 0f
    private var startRawY = 0f
    private var startInsideContent = false
    private var startOutsidePanel = false

    val isOpen: Boolean
        get() = isSetup && overlay.visibility == View.VISIBLE && !isAnimating

    fun setup() {
        if (isSetup) return
        isSetup = true
        overlayHost = activity.window.decorView as ViewGroup

        overlay = FrameLayout(activity).apply {
            visibility = View.GONE
            alpha = 0f
            isClickable = true
            clipChildren = false
            clipToPadding = false
            elevation = dp(48).toFloat()
            translationZ = dp(48).toFloat()
            setOnClickListener { close() }
        }

        panel = FrameLayout(activity).apply {
            background = SmoothSideMenuBackgroundDrawable()
            clipChildren = false
            clipToPadding = false
            elevation = dp(8).toFloat()
        }
        overlay.addView(panel)
        overlayHost.addView(
            overlay,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
        )

        buildPanelContent()
        overlay.post { applyPanelWidth() }
    }

    fun close(animate: Boolean = true) {
        if (!isSetup || overlay.visibility != View.VISIBLE) return
        hideSettingsBubble(animate = false)
        if (!animate || panel.width == 0) {
            overlay.visibility = View.GONE
            overlay.alpha = 0f
            panel.translationX = closedTranslation()
            isAnimating = false
            return
        }
        isAnimating = true
        panel.animate().cancel()
        overlay.animate().cancel()
        panel.animate()
            .translationX(closedTranslation())
            .setDuration(DURATION_MS)
            .setInterpolator(interpolator)
            .start()
        overlay.animate()
            .alpha(0f)
            .setDuration(DURATION_MS)
            .setInterpolator(interpolator)
            .setListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    overlay.visibility = View.GONE
                    panel.translationX = closedTranslation()
                    isAnimating = false
                    overlay.animate().setListener(null)
                }
            })
            .start()
    }

    fun handleDispatchTouchEvent(event: MotionEvent): Boolean {
        if (!isSetup || binding.chatPage.visibility != View.VISIBLE) return false

        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                startRawX = event.rawX
                startRawY = event.rawY
                startInsideContent = isInsideContent(event.rawX, event.rawY)
                startOutsidePanel = startInsideContent &&
                    overlay.visibility == View.VISIBLE &&
                    isOutsidePanel(event.rawX)
                touchTracking = startInsideContent
                consumingGesture = false
                return false
            }

            MotionEvent.ACTION_MOVE -> {
                if (!touchTracking || !startInsideContent) return false
                if (consumingGesture) return true
                val dx = event.rawX - startRawX
                val dy = event.rawY - startRawY
                val horizontalEnough = abs(dx) > dp(9) && abs(dx) > abs(dy) * 1.08f
                if (!horizontalEnough) return false

                if (overlay.visibility != View.VISIBLE && dx > 0f) {
                    beginConsumingDrawerGesture(event)
                    show()
                    return true
                }
                if (overlay.visibility == View.VISIBLE && dx < 0f) {
                    beginConsumingDrawerGesture(event)
                    close()
                    return true
                }
                touchTracking = false
                return false
            }

            MotionEvent.ACTION_UP -> {
                val wasConsuming = consumingGesture
                val dx = event.rawX - startRawX
                val dy = event.rawY - startRawY
                touchTracking = false
                consumingGesture = false
                if (wasConsuming) return true
                if (overlay.visibility == View.VISIBLE &&
                    startOutsidePanel &&
                    abs(dx) < dp(8) &&
                    abs(dy) < dp(8)
                ) {
                    close()
                    return true
                }
                return false
            }

            MotionEvent.ACTION_CANCEL -> {
                val wasConsuming = consumingGesture
                touchTracking = false
                consumingGesture = false
                return wasConsuming
            }
        }
        return consumingGesture
    }

    private fun beginConsumingDrawerGesture(event: MotionEvent) {
        if (consumingGesture) return
        consumingGesture = true
        dismissActionPopup()
        cancelChildTouch(event)
    }

    private fun show() {
        if (!isSetup || isAnimating || overlay.visibility == View.VISIBLE) return
        dismissActionPopup()
        updateConversationSummaries()
        hideSettingsBubble(animate = false)
        applyPanelWidth()
        overlay.visibility = View.VISIBLE
        overlay.bringToFront()
        overlay.alpha = 0f
        panel.translationX = closedTranslation()
        isAnimating = true
        panel.animate().cancel()
        overlay.animate().cancel()
        panel.animate()
            .translationX(0f)
            .setDuration(DURATION_MS)
            .setInterpolator(interpolator)
            .start()
        overlay.animate()
            .alpha(1f)
            .setDuration(DURATION_MS)
            .setInterpolator(interpolator)
            .setListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    isAnimating = false
                    overlay.animate().setListener(null)
                }
            })
            .start()
    }

    private fun buildPanelContent() {
        val topMenu = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
        }
        panel.addView(
            topMenu,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP or Gravity.START
                leftMargin = dp(32)
                topMargin = dp(106)
            }
        )
        listOf("项目", "文件库", "设备").forEach { title ->
            topMenu.addView(menuText(title).apply {
                setOnClickListener {
                    Toast.makeText(activity, "$title 功能准备中", Toast.LENGTH_SHORT).show()
                }
            })
        }

        val chatGroup = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
        }
        panel.addView(
            chatGroup,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.WRAP_CONTENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                gravity = Gravity.TOP or Gravity.START
                leftMargin = dp(32)
                topMargin = dp(388)
            }
        )
        chatGroup.addView(sectionText("当前聊天"))
        repeat(4) {
            val row = menuText("聊天内容缩写").apply {
                layoutParams = LinearLayout.LayoutParams(dp(210), dp(40))
            }
            summaryRows += row
            chatGroup.addView(row)
        }

        val settingsLabel = menuText("设置").apply {
            isClickable = true
            foreground = selectableForeground()
            setPadding(0, 0, dp(8), 0)
            setOnClickListener { toggleSettingsBubble() }
        }
        panel.addView(
            settingsLabel,
            FrameLayout.LayoutParams(dp(110), dp(40)).apply {
                gravity = Gravity.BOTTOM or Gravity.START
                leftMargin = dp(32)
                bottomMargin = dp(18)
            }
        )

        settingsBubble = buildSettingsBubble()
        panel.addView(
            settingsBubble,
            FrameLayout.LayoutParams(dp(266), ViewGroup.LayoutParams.WRAP_CONTENT).apply {
                gravity = Gravity.BOTTOM or Gravity.START
                leftMargin = dp(26)
                bottomMargin = dp(58)
            }
        )
    }

    private fun buildSettingsBubble(): FrameLayout {
        val bubble = FrameLayout(activity).apply {
            visibility = View.GONE
            alpha = 0f
            scaleX = 0.98f
            scaleY = 0.98f
            translationY = dp(5).toFloat()
            clipChildren = false
            clipToPadding = false
        }

        val panelBody = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = GradientDrawable().apply {
                cornerRadius = dp(10).toFloat()
                setColor(Color.parseColor(WECHAT_POPUP_PANEL_COLOR))
            }
            setPadding(0, dp(11), 0, dp(11))
        }
        bubble.addView(
            panelBody,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                bottomMargin = dp(9)
            }
        )
        bubble.addView(
            TrianglePointerView(activity, Color.parseColor(WECHAT_POPUP_PANEL_COLOR), pointsDown = true),
            FrameLayout.LayoutParams(dp(20), dp(10)).apply {
                gravity = Gravity.BOTTOM or Gravity.START
                leftMargin = dp(22)
            }
        )

        panelBody.addView(settingsRow("账号信息") { showAccountInfo() })
        panelBody.addView(settingsRow("个人账户") { openAccountEntry() })
        panelBody.addView(settingsRow("剩余用量") { showUsageHint() })
        panelBody.addView(settingsRow("退出登录") { confirmLogout() })
        return bubble
    }

    private fun toggleSettingsBubble() {
        if (settingsBubble.visibility == View.VISIBLE) {
            hideSettingsBubble()
        } else {
            showSettingsBubble()
        }
    }

    private fun showSettingsBubble() {
        settingsBubble.visibility = View.VISIBLE
        settingsBubble.animate().cancel()
        settingsBubble.alpha = 0f
        settingsBubble.scaleX = 0.98f
        settingsBubble.scaleY = 0.98f
        settingsBubble.translationY = dp(5).toFloat()
        settingsBubble.animate()
            .alpha(1f)
            .scaleX(1f)
            .scaleY(1f)
            .translationY(0f)
            .setDuration(160L)
            .setInterpolator(interpolator)
            .start()
    }

    private fun hideSettingsBubble(animate: Boolean = true) {
        if (!isSetup || settingsBubble.visibility != View.VISIBLE) return
        settingsBubble.animate().cancel()
        if (!animate) {
            settingsBubble.visibility = View.GONE
            settingsBubble.alpha = 0f
            return
        }
        settingsBubble.animate()
            .alpha(0f)
            .scaleX(0.98f)
            .scaleY(0.98f)
            .translationY(dp(5).toFloat())
            .setDuration(120L)
            .setInterpolator(interpolator)
            .setListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    settingsBubble.visibility = View.GONE
                    settingsBubble.animate().setListener(null)
                }
            })
            .start()
    }

    private fun settingsRow(title: String, action: () -> Unit): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(38)
            )
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            setPadding(dp(22), 0, dp(22), 0)
            text = title
            setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
            textSize = 17f
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener {
                hideSettingsBubble()
                action()
            }
        }
    }

    private fun menuText(title: String): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(190), dp(42))
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = title
            setTextColor(Color.parseColor("#C9C9C9"))
            textSize = 17.5f
        }
    }

    private fun sectionText(title: String): TextView {
        return menuText(title).apply {
            layoutParams = LinearLayout.LayoutParams(dp(210), dp(44))
            setTextColor(Color.parseColor("#D0D0D0"))
        }
    }

    private fun updateConversationSummaries() {
        val snippets = activeConversation().messages
            .asReversed()
            .mapNotNull { message -> summarizeForMenu(message.content) }
            .take(summaryRows.size)
            .asReversed()
        summaryRows.forEachIndexed { index, row ->
            row.text = snippets.getOrNull(index) ?: "聊天内容缩写"
        }
    }

    private fun summarizeForMenu(raw: String): String? {
        val compact = raw
            .replace('\n', ' ')
            .replace(Regex("\\s+"), " ")
            .trim()
        if (compact.isEmpty()) return null
        return if (compact.length > 12) compact.take(12) + "..." else compact
    }

    private fun showAccountInfo() {
        AlertDialog.Builder(activity)
            .setTitle("账号信息")
            .setMessage(accountInfoText(activity))
            .setPositiveButton("知道了", null)
            .show()
    }

    private fun openAccountEntry() {
        if (AuthManager.isLoggedIn(activity)) {
            showAccountInfo()
        } else {
            activity.startActivity(Intent(activity, LoginActivity::class.java))
        }
    }

    private fun showUsageHint() {
        Toast.makeText(activity, "剩余用量统计准备中", Toast.LENGTH_SHORT).show()
    }

    private fun applyPanelWidth() {
        val screenWidth = binding.contentContainer.width.takeIf { it > 0 }
            ?: overlayHost.width.takeIf { it > 0 }
            ?: activity.resources.displayMetrics.widthPixels
        val maxWidth = (screenWidth - dp(56)).coerceAtLeast(1)
        val minWidth = min(dp(280), maxWidth)
        val panelWidth = (screenWidth - dp(84)).coerceIn(minWidth, maxWidth)
        val params = panel.layoutParams as FrameLayout.LayoutParams
        if (params.width != panelWidth) {
            params.width = panelWidth
            params.height = FrameLayout.LayoutParams.MATCH_PARENT
            params.gravity = Gravity.START
            panel.layoutParams = params
        }
        if (overlay.visibility != View.VISIBLE) {
            panel.translationX = closedTranslation()
        }
    }

    private fun closedTranslation(): Float {
        val width = panel.width.takeIf { it > 0 }
            ?: ((overlayHost.width.takeIf { it > 0 }
                ?: binding.contentContainer.width.takeIf { it > 0 }
                ?: activity.resources.displayMetrics.widthPixels) - dp(84)).coerceAtLeast(dp(1))
        return -width.toFloat()
    }

    private fun isInsideContent(rawX: Float, rawY: Float): Boolean {
        val location = IntArray(2)
        binding.contentContainer.getLocationOnScreen(location)
        return rawX >= location[0] &&
            rawX <= location[0] + binding.contentContainer.width &&
            rawY >= location[1] &&
            rawY <= location[1] + binding.contentContainer.height
    }

    private fun isOutsidePanel(rawX: Float): Boolean {
        val location = IntArray(2)
        binding.contentContainer.getLocationOnScreen(location)
        val localX = rawX - location[0]
        return localX > panel.width
    }

    private class TrianglePointerView(
        context: android.content.Context,
        color: Int,
        private val pointsDown: Boolean
    ) : View(context) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            this.color = color
            style = Paint.Style.FILL
        }

        override fun onDraw(canvas: Canvas) {
            super.onDraw(canvas)
            val path = Path().apply {
                if (pointsDown) {
                    moveTo(0f, 0f)
                    lineTo(width.toFloat(), 0f)
                    lineTo(width / 2f, height.toFloat())
                } else {
                    moveTo(width / 2f, 0f)
                    lineTo(width.toFloat(), height.toFloat())
                    lineTo(0f, height.toFloat())
                }
                close()
            }
            canvas.drawPath(path, paint)
        }
    }

    private class SmoothSideMenuBackgroundDrawable : Drawable() {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            isDither = true
            style = Paint.Style.FILL
        }
        private var shaderWidth = -1

        override fun draw(canvas: Canvas) {
            val currentBounds = bounds
            if (currentBounds.width() <= 0 || currentBounds.height() <= 0) return
            if (shaderWidth != currentBounds.width()) {
                shaderWidth = currentBounds.width()
                paint.shader = LinearGradient(
                    0f,
                    0f,
                    currentBounds.width().toFloat(),
                    0f,
                    intArrayOf(
                        Color.parseColor("#1B2025"),
                        Color.parseColor("#191D21"),
                        Color.parseColor("#171A1D"),
                        Color.parseColor("#141719"),
                        Color.parseColor("#111213"),
                        Color.parseColor("#101010")
                    ),
                    floatArrayOf(0f, 0.18f, 0.38f, 0.62f, 0.84f, 1f),
                    Shader.TileMode.CLAMP
                )
            }
            canvas.drawRect(currentBounds, paint)
        }

        override fun setAlpha(alpha: Int) {
            paint.alpha = alpha
            invalidateSelf()
        }

        override fun setColorFilter(colorFilter: ColorFilter?) {
            paint.colorFilter = colorFilter
            invalidateSelf()
        }

        @Deprecated("Deprecated in Java")
        override fun getOpacity(): Int = PixelFormat.OPAQUE
    }

    private companion object {
        private const val DURATION_MS = 260L
    }
}
