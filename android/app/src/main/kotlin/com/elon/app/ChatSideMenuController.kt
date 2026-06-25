package com.elon.app

import android.animation.Animator
import android.animation.AnimatorListenerAdapter
import android.animation.ArgbEvaluator
import android.animation.ValueAnimator
import android.app.AlertDialog
import android.content.Context
import android.content.Intent
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.DragEvent
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.ViewGroup
import android.view.animation.PathInterpolator
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import kotlin.math.abs
import kotlin.math.min
import kotlin.math.roundToInt

internal class ChatSideMenuController(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val activeConversation: () -> AppConversation,
    private val conversations: () -> List<AppConversation>,
    private val activeConversationIndex: () -> Int,
    private val openConversation: (Int) -> Unit,
    private val copyConversationIdentity: (Int) -> Unit,
    private val isConversationWorking: (Int) -> Boolean,
    private val showProjectShareSideMenu: () -> Boolean,
    private val projects: () -> List<AppProject>,
    private val activeProjectIndex: () -> Int,
    private val openPersonalProject: (Int) -> Unit,
    private val openJointProject: (Int) -> Unit,
    private val openRecentConversation: (Int, Int) -> Unit,
    private val openProjectManagement: () -> Unit,
    private val sendProjectShare: (ChatProjectShare) -> Unit,
    private val showCreateConversationDialog: () -> Unit,
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
    private lateinit var accountNameText: TextView
    private lateinit var accountAccountText: TextView
    private lateinit var handleToggleSwitchTrack: FrameLayout
    private lateinit var handleToggleKnob: View
    private lateinit var handleToggleKnobBackground: GradientDrawable
    private lateinit var usageDropdown: SideMenuUsageDropdown
    private lateinit var aiMenuView: ChatAiSideMenuView
    private lateinit var projectMenuView: ChatProjectSideMenuView
    private var isSetup = false
    private var isAnimating = false
    private var touchTracking = false
    private var consumingGesture = false
    private var startRawX = 0f
    private var startRawY = 0f
    private var startInsideContent = false
    private var startOutsidePanel = false
    private val sideMenuHandlePrefs by lazy {
        activity.getSharedPreferences(SIDE_MENU_HANDLE_PREFS, Context.MODE_PRIVATE)
    }
    private val handleTouchSlop by lazy { ViewConfiguration.get(activity).scaledTouchSlop }
    private var handleStartRawX = 0f
    private var handleStartRawY = 0f
    private var handleStartBottomMargin = 0
    private var handleDragging = false
    private var handleMovedBeyondTapSlop = false
    private var sideMenuHandleEnabled = true
    private var handleToggleKnobColor = Color.WHITE
    private var handleToggleColorAnimator: ValueAnimator? = null

    val isOpen: Boolean
        get() = isSetup && overlay.visibility == View.VISIBLE && !isAnimating

    fun setup() {
        if (isSetup) return
        isSetup = true
        overlayHost = activity.window.decorView as ViewGroup
        sideMenuHandleEnabled = sideMenuHandlePrefs.getBoolean(SIDE_MENU_HANDLE_VISIBLE_KEY, true)
        if (!sideMenuHandlePrefs.getBoolean(SIDE_MENU_HANDLE_DEFAULT_VISIBLE_MIGRATION_KEY, false)) {
            sideMenuHandleEnabled = true
            sideMenuHandlePrefs.edit()
                .putBoolean(SIDE_MENU_HANDLE_VISIBLE_KEY, true)
                .putBoolean(SIDE_MENU_HANDLE_DEFAULT_VISIBLE_MIGRATION_KEY, true)
                .apply()
        }

        overlay = FrameLayout(activity).apply {
            visibility = View.GONE
            alpha = 0f
            isClickable = true
            clipChildren = false
            clipToPadding = false
            elevation = dp(48).toFloat()
            translationZ = dp(48).toFloat()
            setOnClickListener { close() }
            setOnDragListener { _, event -> handleProjectDrag(event) }
        }

        panel = FrameLayout(activity).apply {
            background = GradientDrawable().apply {
                setColor(activity.getColor(R.color.elon_side_menu_bg))
            }
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
        binding.chatSideMenuHandleButton.apply {
            bringToFront()
            setOnClickListener { openFromHandle() }
            setOnTouchListener { view, event -> handleSideMenuHandleTouch(view, event) }
        }
        applySideMenuHandleAvailability(animated = false)
        overlay.post { applyPanelWidth() }
    }

    fun close(animate: Boolean = true) {
        if (!isSetup || overlay.visibility != View.VISIBLE) return
        hideSettingsBubble(animate = false)
        aiMenuView.stopAnimations()
        if (!animate || panel.width == 0) {
            overlay.visibility = View.GONE
            overlay.alpha = 0f
            panel.translationX = closedTranslation()
            isAnimating = false
            revealSideMenuHandle(animated = false)
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
                    revealSideMenuHandle(animated = true)
                }
            })
            .start()
    }

    fun openFromHandle() {
        if (!isSetup || !sideMenuHandleEnabled || binding.chatPage.visibility != View.VISIBLE) return
        show()
    }

    private fun handleSideMenuHandleTouch(view: View, event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                handleStartRawX = event.rawX
                handleStartRawY = event.rawY
                handleStartBottomMargin =
                    (view.layoutParams as? FrameLayout.LayoutParams)?.bottomMargin ?: 0
                handleDragging = false
                handleMovedBeyondTapSlop = false
                view.isPressed = true
                view.parent?.requestDisallowInterceptTouchEvent(true)
                return true
            }

            MotionEvent.ACTION_MOVE -> {
                val deltaX = event.rawX - handleStartRawX
                val deltaY = event.rawY - handleStartRawY
                if (abs(deltaX) > handleTouchSlop || abs(deltaY) > handleTouchSlop) {
                    handleMovedBeyondTapSlop = true
                }
                if (!handleDragging &&
                    abs(deltaY) > handleTouchSlop &&
                    abs(deltaY) >= abs(deltaX)
                ) {
                    handleDragging = true
                }
                if (handleDragging) {
                    updateSideMenuHandleBottomMargin(
                        view,
                        handleStartBottomMargin - deltaY.roundToInt()
                    )
                }
                return true
            }

            MotionEvent.ACTION_UP -> {
                val shouldOpen = !handleMovedBeyondTapSlop
                handleDragging = false
                handleMovedBeyondTapSlop = false
                view.isPressed = false
                view.parent?.requestDisallowInterceptTouchEvent(false)
                if (shouldOpen) view.performClick()
                return true
            }

            MotionEvent.ACTION_CANCEL -> {
                handleDragging = false
                handleMovedBeyondTapSlop = false
                view.isPressed = false
                view.parent?.requestDisallowInterceptTouchEvent(false)
                return true
            }
        }
        return true
    }

    private fun updateSideMenuHandleBottomMargin(view: View, requestedBottomMargin: Int) {
        val params = view.layoutParams as? FrameLayout.LayoutParams ?: return
        val parentHeight = (view.parent as? View)?.height ?: 0
        val fallbackHeight = dp(56)
        val viewHeight = view.height.takeIf { it > 0 }
            ?: params.height.takeIf { it > 0 }
            ?: fallbackHeight
        val minBottom = dp(16)
        val maxBottom = (parentHeight - viewHeight - dp(16)).coerceAtLeast(minBottom)
        params.bottomMargin = requestedBottomMargin.coerceIn(minBottom, maxBottom)
        view.layoutParams = params
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
        applyContentMode()
        hideSettingsBubble(animate = false)
        applyPanelWidth()
        retractSideMenuHandle(animated = true)
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

    private fun handleProjectDrag(event: DragEvent): Boolean {
        if (!showProjectShareSideMenu()) return false
        val share = event.localState as? ChatProjectShare ?: return false
        return when (event.action) {
            DragEvent.ACTION_DRAG_STARTED -> true
            DragEvent.ACTION_DROP -> {
                if (event.x > panel.width) {
                    showChatProjectDropRipple(overlay, binding.contentContainer, share, event.x, event.y)
                    close(animate = true)
                    overlay.postDelayed({ sendProjectShare(share) }, 140L)
                }
                true
            }
            DragEvent.ACTION_DRAG_ENDED -> true
            else -> true
        }
    }

    private fun buildPanelContent() {
        aiMenuView = ChatAiSideMenuView(
            context = activity,
            conversations = conversations,
            activeConversationIndex = activeConversationIndex,
            projects = projects,
            activeProjectIndex = activeProjectIndex,
            openConversation = openConversation,
            openPersonalProject = openPersonalProject,
            openJointProject = openJointProject,
            openProjectSpace = openProjectManagement,
            copyConversationIdentity = copyConversationIdentity,
            isConversationWorking = isConversationWorking,
            showCreateConversationDialog = showCreateConversationDialog,
            requestClose = { animate -> close(animate) },
            bottomReservedHeightDp = SETTINGS_DOCK_HEIGHT_DP,
            dp = dp,
            selectableForeground = selectableForeground
        )
        panel.addView(
            aiMenuView,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
        )

        projectMenuView = ChatProjectSideMenuView(
            context = activity,
            projects = projects,
            activeProjectIndex = activeProjectIndex,
            activeConversationIndex = activeConversationIndex,
            openPersonalProject = openPersonalProject,
            openJointProject = openJointProject,
            openRecentConversation = openRecentConversation,
            requestClose = { animate -> close(animate) },
            dp = dp,
            selectableForeground = selectableForeground
        )
        panel.addView(
            projectMenuView,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            ).apply {
                gravity = Gravity.TOP or Gravity.START
                bottomMargin = dp(SETTINGS_DOCK_HEIGHT_DP)
            }
        )
        projectMenuView.visibility = View.GONE

        val settingsLabel = menuText("设置").apply {
            isClickable = true
            foreground = selectableForeground()
            setPadding(0, 0, dp(8), 0)
            setOnClickListener { toggleSettingsBubble() }
        }
        val settingsDock = FrameLayout(activity).apply {
            isClickable = true
            background = GradientDrawable().apply {
                setColor(activity.getColor(R.color.elon_side_menu_bg))
            }
            addView(
                settingsLabel,
                FrameLayout.LayoutParams(dp(110), dp(40)).apply {
                    gravity = Gravity.BOTTOM or Gravity.START
                    leftMargin = dp(32)
                    bottomMargin = dp(18)
                }
            )
        }
        panel.addView(
            settingsDock,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                dp(SETTINGS_DOCK_HEIGHT_DP)
            ).apply {
                gravity = Gravity.BOTTOM or Gravity.START
            }
        )

        settingsBubble = buildSettingsBubble()
        panel.addView(
            settingsBubble,
            FrameLayout.LayoutParams(dp(232), ViewGroup.LayoutParams.WRAP_CONTENT).apply {
                gravity = Gravity.BOTTOM or Gravity.START
                leftMargin = dp(26)
                bottomMargin = dp(58)
            }
        )
    }

    private fun applyContentMode() {
        if (showProjectShareSideMenu()) {
            aiMenuView.visibility = View.GONE
            aiMenuView.stopAnimations()
            projectMenuView.visibility = View.VISIBLE
            projectMenuView.render()
        } else {
            projectMenuView.visibility = View.GONE
            aiMenuView.visibility = View.VISIBLE
            aiMenuView.render()
        }
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

        panelBody.addView(accountSummaryRow())
        panelBody.addView(settingsRow("个人账户") { openAccountEntry() })
        usageDropdown = SideMenuUsageDropdown(activity, dp, selectableForeground)
        panelBody.addView(usageDropdown.rowView)
        panelBody.addView(usageDropdown.detailsView)
        panelBody.addView(sideMenuHandleToggleRow())
        panelBody.addView(settingsRow("退出登录") { confirmLogout() })
        return bubble
    }

    private fun updateSettingsBubbleBounds() {
        if (!::settingsBubble.isInitialized) return
        val panelWidth = panel.layoutParams?.width?.takeIf { it > 0 }
            ?: panel.width.takeIf { it > 0 }
            ?: return
        val desiredLeft = dp(26)
        val desiredRight = dp(22)
        val maxWidth = (panelWidth - desiredLeft - desiredRight).coerceAtLeast(dp(188))
        val targetWidth = min(dp(232), maxWidth)
        val targetLeft = desiredLeft
            .coerceAtMost((panelWidth - targetWidth - desiredRight).coerceAtLeast(0))
        val params = settingsBubble.layoutParams as? FrameLayout.LayoutParams ?: return
        if (params.width != targetWidth || params.leftMargin != targetLeft) {
            params.width = targetWidth
            params.leftMargin = targetLeft
            params.gravity = Gravity.BOTTOM or Gravity.START
            settingsBubble.layoutParams = params
        }
    }

    private fun toggleSettingsBubble() {
        if (settingsBubble.visibility == View.VISIBLE) {
            hideSettingsBubble()
        } else {
            showSettingsBubble()
        }
    }

    private fun showSettingsBubble() {
        updateSettingsBubbleBounds()
        updateAccountSummary()
        updateSideMenuHandleToggleText()
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
        if (::usageDropdown.isInitialized) usageDropdown.collapse()
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

    private fun accountSummaryRow(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(54)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.VERTICAL
            setPadding(dp(22), 0, dp(22), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener {
                hideSettingsBubble()
                showAccountInfo()
            }
            accountNameText = TextView(activity).apply {
                includeFontPadding = false
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
                textSize = 17f
            }
            accountAccountText = TextView(activity).apply {
                includeFontPadding = false
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
                setTextColor(Color.parseColor("#777777"))
                textSize = 12.5f
                setPadding(0, dp(6), 0, 0)
            }
            addView(accountNameText)
            addView(accountAccountText)
            updateAccountSummary()
        }
    }

    private fun updateAccountSummary() {
        if (!::accountNameText.isInitialized || !::accountAccountText.isInitialized) return
        accountNameText.text = accountMenuTitle()
        accountAccountText.text = "账号：${accountMenuAccount()}"
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

    private fun sideMenuHandleToggleRow(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(38)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(22), 0, dp(22), 0)
            isClickable = true
            foreground = selectableForeground()
            val titleText = TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
                gravity = Gravity.CENTER_VERTICAL or Gravity.START
                includeFontPadding = false
                text = "悬浮按钮"
                setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
                textSize = 17f
            }
            handleToggleSwitchTrack = FrameLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    dp(30),
                    dp(16)
                )
                background = GradientDrawable().apply {
                    shape = GradientDrawable.RECTANGLE
                    cornerRadius = dp(8).toFloat()
                    setColor(Color.parseColor("#343434"))
                    setStroke(dp(1), Color.parseColor("#A8A8A8"))
                }
                isClickable = false
                clipChildren = false
                clipToPadding = false
                handleToggleKnobBackground = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setColor(handleToggleKnobColor)
                }
                handleToggleKnob = View(activity).apply {
                    background = handleToggleKnobBackground
                }
                addView(
                    handleToggleKnob,
                    FrameLayout.LayoutParams(dp(8), dp(8)).apply {
                        gravity = Gravity.START or Gravity.CENTER_VERTICAL
                        leftMargin = dp(4)
                    }
                )
            }
            addView(titleText)
            addView(handleToggleSwitchTrack)
            updateSideMenuHandleToggleText()
            setOnClickListener { toggleSideMenuHandleAvailability() }
        }
    }

    private fun toggleSideMenuHandleAvailability() {
        sideMenuHandleEnabled = !sideMenuHandleEnabled
        sideMenuHandlePrefs.edit()
            .putBoolean(SIDE_MENU_HANDLE_VISIBLE_KEY, sideMenuHandleEnabled)
            .apply()
        updateSideMenuHandleToggleText(animated = true)
        applySideMenuHandleAvailability(animated = true)
    }

    private fun updateSideMenuHandleToggleText(animated: Boolean = false) {
        if (!::handleToggleKnob.isInitialized || !::handleToggleSwitchTrack.isInitialized) return
        val targetTranslation = if (sideMenuHandleEnabled) 0f else dp(14).toFloat()
        val targetColor = if (sideMenuHandleEnabled) {
            Color.WHITE
        } else {
            Color.parseColor("#E62129")
        }
        handleToggleSwitchTrack.contentDescription =
            if (sideMenuHandleEnabled) "悬浮按钮已打开" else "悬浮按钮已隐藏"
        handleToggleKnob.animate().cancel()
        handleToggleColorAnimator?.cancel()
        if (!animated) {
            handleToggleKnob.translationX = targetTranslation
            handleToggleKnobColor = targetColor
            handleToggleKnobBackground.setColor(targetColor)
            return
        }
        handleToggleKnob.animate()
            .translationX(targetTranslation)
            .setDuration(170L)
            .setInterpolator(interpolator)
            .start()
        handleToggleColorAnimator = ValueAnimator.ofObject(
            ArgbEvaluator(),
            handleToggleKnobColor,
            targetColor
        ).apply {
            duration = 170L
            interpolator = this@ChatSideMenuController.interpolator
            addUpdateListener { animator ->
                handleToggleKnobColor = animator.animatedValue as Int
                handleToggleKnobBackground.setColor(handleToggleKnobColor)
            }
            start()
        }
    }

    private fun applySideMenuHandleAvailability(animated: Boolean) {
        if (!sideMenuHandleEnabled) {
            hideSideMenuHandle(animated)
            return
        }
        if (overlay.visibility == View.VISIBLE) {
            retractSideMenuHandle(animated = false)
        } else {
            revealSideMenuHandle(animated)
        }
    }

    private fun sideMenuHandleTravel(): Float {
        return (binding.chatSideMenuHandleButton.width.takeIf { it > 0 } ?: dp(48)).toFloat()
    }

    private fun hideSideMenuHandle(animated: Boolean) {
        val handle = binding.chatSideMenuHandleButton
        handle.animate().cancel()
        val finish = {
            handle.visibility = View.GONE
            handle.translationX = 0f
            handle.animate().setListener(null)
        }
        if (!animated || handle.visibility != View.VISIBLE) {
            finish()
            return
        }
        handle.animate()
            .translationX(-sideMenuHandleTravel())
            .setDuration(130L)
            .setInterpolator(interpolator)
            .setListener(object : AnimatorListenerAdapter() {
                override fun onAnimationEnd(animation: Animator) {
                    finish()
                }
            })
            .start()
    }

    private fun retractSideMenuHandle(animated: Boolean) {
        val handle = binding.chatSideMenuHandleButton
        handle.animate().cancel()
        if (!sideMenuHandleEnabled) {
            handle.visibility = View.GONE
            handle.translationX = 0f
            return
        }
        handle.visibility = View.VISIBLE
        handle.alpha = 1f
        handle.animate().setListener(null)
        val target = -sideMenuHandleTravel()
        if (!animated) {
            handle.translationX = target
            return
        }
        handle.animate()
            .translationX(target)
            .setDuration(130L)
            .setInterpolator(interpolator)
            .start()
    }

    private fun revealSideMenuHandle(animated: Boolean) {
        val handle = binding.chatSideMenuHandleButton
        handle.animate().cancel()
        if (!sideMenuHandleEnabled) {
            handle.visibility = View.GONE
            handle.translationX = 0f
            return
        }
        handle.visibility = View.VISIBLE
        handle.alpha = 1f
        handle.animate().setListener(null)
        if (binding.chatPage.visibility != View.VISIBLE || !animated) {
            handle.translationX = 0f
            return
        }
        handle.translationX = -sideMenuHandleTravel()
        handle.animate()
            .translationX(0f)
            .setDuration(170L)
            .setInterpolator(interpolator)
            .start()
    }

    private fun menuText(title: String): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(190), dp(42))
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            includeFontPadding = false
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            text = title
            setTextColor(Color.parseColor("#A8A8A8"))
            textSize = 17.5f
        }
    }

    private fun showAccountInfo() {
        AlertDialog.Builder(activity)
            .setTitle("账号信息")
            .setMessage(accountInfoText(activity))
            .setPositiveButton("知道了", null)
            .show()
    }

    private fun accountMenuTitle(): String =
        if (AuthManager.isLoggedIn(activity)) AuthManager.displayName(activity) else "未登录"

    private fun accountMenuAccount(): String {
        if (!AuthManager.isLoggedIn(activity)) return "未登录"
        return AuthManager.account(activity) ?: UserProfileStore.load(activity).wechatId
    }

    private fun openAccountEntry() {
        if (AuthManager.isLoggedIn(activity)) {
            activity.startActivity(Intent(activity, PersonalProfileActivity::class.java))
        } else {
            activity.startActivity(Intent(activity, LoginActivity::class.java))
        }
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
        updateSettingsBubbleBounds()
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

    private companion object {
        private const val DURATION_MS = 260L
        private const val SETTINGS_DOCK_HEIGHT_DP = 88
        private const val SIDE_MENU_HANDLE_PREFS = "chat_side_menu_handle"
        private const val SIDE_MENU_HANDLE_VISIBLE_KEY = "visible"
        private const val SIDE_MENU_HANDLE_DEFAULT_VISIBLE_MIGRATION_KEY = "default_visible_20260625"
    }
}
