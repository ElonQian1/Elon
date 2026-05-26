package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.app.AlertDialog
import android.content.Intent
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
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

internal class ChatSideMenuController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val activeConversation: () -> AppConversation,
    private val confirmLogout: () -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    private val interpolator = PathInterpolator(0.2f, 0f, 0f, 1f)
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

        overlay = FrameLayout(activity).apply {
            visibility = View.GONE
            alpha = 0f
            isClickable = true
            clipChildren = false
            clipToPadding = false
            setOnClickListener { eventlessCloseIfNeeded() }
        }

        panel = FrameLayout(activity).apply {
            background = GradientDrawable(
                GradientDrawable.Orientation.LEFT_RIGHT,
                intArrayOf(Color.parseColor("#172027"), Color.parseColor("#0F1012"))
            )
            clipChildren = false
            clipToPadding = false
            elevation = dp(8).toFloat()
        }
        overlay.addView(panel)
        binding.contentContainer.addView(
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
                startOutsidePanel = startInsideContent && overlay.visibility == View.VISIBLE && isOutsidePanel(event.rawX)
                touchTracking = startInsideContent
                consumingGesture = false
                return false
            }

            MotionEvent.ACTION_MOVE -> {
                if (!touchTracking || !startInsideContent) return false
                if (consumingGesture) return true
                val dx = event.rawX - startRawX
                val dy = event.rawY - startRawY
                val horizontalEnough = abs(dx) > dp(24) && abs(dx) > abs(dy) * 1.18f
                if (!horizontalEnough) return false

                if (overlay.visibility != View.VISIBLE && dx > 0f) {
                    show()
                    consumingGesture = true
                    return true
                }
                if (overlay.visibility == View.VISIBLE && dx < 0f) {
                    close()
                    consumingGesture = true
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
                if (overlay.visibility == View.VISIBLE && startOutsidePanel && abs(dx) < dp(8) && abs(dy) < dp(8)) {
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

    private fun show() {
        if (!isSetup || isAnimating || overlay.visibility == View.VISIBLE) return
        updateConversationSummaries()
        hideSettingsBubble(animate = false)
        applyPanelWidth()
        overlay.visibility = View.VISIBLE
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
                leftMargin = dp(28)
                topMargin = dp(54)
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
                gravity = Gravity.BOTTOM or Gravity.START
                leftMargin = dp(28)
                bottomMargin = dp(136)
            }
        )
        chatGroup.addView(sectionText("当前聊天"))
        repeat(4) {
            val row = menuText("聊天内容缩写").apply {
                layoutParams = LinearLayout.LayoutParams(dp(168), dp(26))
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
            FrameLayout.LayoutParams(dp(88), dp(30)).apply {
                gravity = Gravity.BOTTOM or Gravity.START
                leftMargin = dp(28)
                bottomMargin = dp(18)
            }
        )

        settingsBubble = buildSettingsBubble()
        panel.addView(
            settingsBubble,
            FrameLayout.LayoutParams(dp(210), ViewGroup.LayoutParams.WRAP_CONTENT).apply {
                gravity = Gravity.BOTTOM or Gravity.START
                leftMargin = dp(24)
                bottomMargin = dp(52)
            }
        )
    }

    private fun buildSettingsBubble(): FrameLayout {
        val bubble = FrameLayout(activity).apply {
            visibility = View.GONE
            alpha = 0f
            scaleX = 0.96f
            scaleY = 0.96f
            translationY = dp(6).toFloat()
            clipChildren = false
            clipToPadding = false
        }

        val panelBody = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = GradientDrawable().apply {
                cornerRadius = dp(7).toFloat()
                setColor(Color.parseColor(WECHAT_POPUP_PANEL_COLOR))
            }
            setPadding(0, dp(7), 0, dp(7))
        }
        bubble.addView(
            panelBody,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            ).apply {
                bottomMargin = dp(8)
            }
        )
        bubble.addView(
            TrianglePointerView(activity, Color.parseColor(WECHAT_POPUP_PANEL_COLOR), pointsDown = true),
            FrameLayout.LayoutParams(dp(18), dp(9)).apply {
                gravity = Gravity.BOTTOM or Gravity.START
                leftMargin = dp(34)
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
        settingsBubble.scaleX = 0.96f
        settingsBubble.scaleY = 0.96f
        settingsBubble.translationY = dp(6).toFloat()
        settingsBubble.animate()
            .alpha(1f)
            .scaleX(1f)
            .scaleY(1f)
            .translationY(0f)
            .setDuration(150L)
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
            .scaleX(0.96f)
            .scaleY(0.96f)
            .translationY(dp(6).toFloat())
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
                dp(25)
            )
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            setPadding(dp(16), 0, dp(16), 0)
            text = title
            setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
            textSize = 12.5f
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
            layoutParams = LinearLayout.LayoutParams(dp(150), dp(27))
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = title
            setTextColor(Color.parseColor("#B8B8B8"))
            textSize = 12.5f
        }
    }

    private fun sectionText(title: String): TextView {
        return menuText(title).apply {
            setTextColor(Color.parseColor("#C4C4C4"))
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
        return if (compact.length > 12) compact.take(12) + "…" else compact
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

    private fun eventlessCloseIfNeeded() {
        if (overlay.visibility == View.VISIBLE) close()
    }

    private fun applyPanelWidth() {
        val screenWidth = binding.contentContainer.width.takeIf { it > 0 }
            ?: activity.resources.displayMetrics.widthPixels
        val panelWidth = (screenWidth - dp(74)).coerceAtLeast(dp(252))
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
        val screenWidth = binding.contentContainer.width.takeIf { it > 0 }
            ?: activity.resources.displayMetrics.widthPixels
        return screenWidth.toFloat()
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

    private companion object {
        private const val DURATION_MS = 260L
    }
}
