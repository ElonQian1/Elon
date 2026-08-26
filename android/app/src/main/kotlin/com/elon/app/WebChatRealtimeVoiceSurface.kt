package com.elon.app

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import kotlin.math.abs

internal interface WebChatRealtimeVoiceSurface {
    fun show(
        onClose: () -> Unit,
        onRetry: () -> Unit,
        onOfficialFallback: () -> Unit,
        onOpenConversation: () -> Unit,
    )

    fun render(state: WebChatRealtimeVoiceState)
    fun setHostVisible(visible: Boolean)
    fun ensureVisibleOnTop()
    fun hide()
    fun isVisible(): Boolean
}

internal class WebChatRealtimeVoiceOverlay(
    private val activity: AppCompatActivity,
    private val host: FrameLayout = WebChatRealtimeVoiceOverlayHost.resolve(activity),
) : WebChatRealtimeVoiceSurface {
    private val root = FrameLayout(activity).apply {
        setBackgroundColor(Color.TRANSPARENT)
        visibility = View.GONE
        isClickable = false
        isFocusable = false
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        clipChildren = false
        clipToPadding = false
    }
    private val panel = FrameLayout(activity).apply {
        isClickable = false
        isFocusable = false
        elevation = dp(10).toFloat()
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_SURFACE
    }
    private val collapsedOrb = FrameLayout(activity).apply {
        isClickable = true
        isFocusable = true
    }
    private val collapsedIcon = ImageView(activity).apply {
        setImageResource(R.drawable.ic_input_voice)
        scaleType = ImageView.ScaleType.CENTER
        isClickable = false
        isFocusable = false
    }
    private val collapsedStatus = View(activity).apply {
        isClickable = false
        isFocusable = false
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }
    private val expandedCard = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(12), dp(10), dp(10), dp(10))
        visibility = View.GONE
        background = rounded(color(R.color.elon_surface_float), 18)
    }
    private val expandedHeader = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
    }
    private val expandedIcon = ImageView(activity).apply {
        setImageResource(R.drawable.ic_input_voice)
        scaleType = ImageView.ScaleType.CENTER
        isClickable = true
        isFocusable = true
    }
    private val textColumn = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(10), 0, dp(8), 0)
    }
    private val status = textView(15f, color(R.color.elon_text_primary), bold = true).apply {
        maxLines = 1
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_STATUS
    }
    private val detail = textView(12f, color(R.color.elon_text_secondary)).apply {
        maxLines = 2
    }
    private val conversationTarget = textView(13f, color(R.color.elon_text_primary), bold = true).apply {
        gravity = Gravity.CENTER_VERTICAL
        minHeight = dp(48)
        maxLines = 1
        ellipsize = TextUtils.TruncateAt.END
        setPadding(dp(4), dp(6), dp(4), dp(6))
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_OPEN_CONVERSATION
    }
    private val close = ImageButton(activity).apply {
        setImageResource(R.drawable.ic_voice_call_hangup)
        scaleType = ImageView.ScaleType.CENTER
        setPadding(dp(11), dp(11), dp(11), dp(11))
        background = oval(color(R.color.elon_status_danger))
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_CLOSE
    }
    private val failureActions = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(0, dp(10), 0, 0)
        visibility = View.GONE
    }
    private val retry = actionButton("重试").apply {
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_RETRY
    }
    private val officialFallback = actionButton("官网语音", secondary = true).apply {
        contentDescription = WebChatProductionSelectors.REALTIME_VOICE_OFFICIAL_FALLBACK
    }
    private val touchSlop = ViewConfiguration.get(activity).scaledTouchSlop
    private var metrics = WebChatRealtimeVoiceFloatingLayoutPolicy.resolve(
        activity.resources.displayMetrics.widthPixels,
        activity.resources.displayMetrics.density,
    )
    private var expanded = false
    private var positioned = false
    private var dragging = false
    private var requestedVisible = false
    private var hostVisible = true
    private var downRawX = 0f
    private var downRawY = 0f
    private var downLeft = 0f
    private var downTop = 0f
    private var lastVisibleState: WebChatRealtimeVoiceVisibleState? = null

    init {
        buildContent()
        installInteraction()
        host.addOnLayoutChangeListener { _, _, _, _, _, _, _, _, _ ->
            if (requestedVisible) ensureVisibleOnTop()
        }
    }

    override fun show(
        onClose: () -> Unit,
        onRetry: () -> Unit,
        onOfficialFallback: () -> Unit,
        onOpenConversation: () -> Unit,
    ) {
        close.setOnClickListener { onClose() }
        retry.setOnClickListener { onRetry() }
        officialFallback.setOnClickListener { onOfficialFallback() }
        conversationTarget.setOnClickListener { onOpenConversation() }
        requestedVisible = true
        setExpanded(false)
        ensureVisibleOnTop()
    }

    override fun render(state: WebChatRealtimeVoiceState) {
        val visibleState = WebChatRealtimeVoiceStatePolicy.visibleState(state)
        status.text = "语音 AI · ${visibleState.label}"
        detail.text = state.detail
        val context = state.context
        conversationTarget.text = context?.let { "记录到：${it.label}" } ?: "记录到：当前 ChatGPT 会话"
        conversationTarget.isEnabled = context?.openable == true
        conversationTarget.alpha = if (conversationTarget.isEnabled) 1f else 0.72f
        conversationTarget.contentDescription = buildString {
            append(WebChatProductionSelectors.REALTIME_VOICE_OPEN_CONVERSATION)
            append('：')
            append(context?.label ?: "当前 ChatGPT 会话")
        }
        val stageColor = when (visibleState) {
            WebChatRealtimeVoiceVisibleState.CONNECTING -> color(R.color.elon_signal_mist)
            WebChatRealtimeVoiceVisibleState.IDLE -> color(R.color.elon_status_success)
            WebChatRealtimeVoiceVisibleState.LISTENING -> color(R.color.elon_signal_mist)
            WebChatRealtimeVoiceVisibleState.THINKING -> color(R.color.elon_status_info)
            WebChatRealtimeVoiceVisibleState.SPEAKING -> color(R.color.elon_status_success)
            WebChatRealtimeVoiceVisibleState.SYNCING -> color(R.color.elon_status_info)
            WebChatRealtimeVoiceVisibleState.PAUSED -> color(R.color.elon_text_secondary)
            WebChatRealtimeVoiceVisibleState.ENDING -> color(R.color.elon_status_info)
            WebChatRealtimeVoiceVisibleState.HANGUP_UNCONFIRMED -> color(R.color.elon_status_info)
            WebChatRealtimeVoiceVisibleState.FAILED -> color(R.color.elon_status_danger)
        }
        applyVoiceIconStyle(collapsedOrb, collapsedIcon, stageColor, strokeWidth = 2)
        applyVoiceIconStyle(expandedIcon, expandedIcon, stageColor, strokeWidth = 1)
        collapsedStatus.background = oval(stageColor, color(R.color.elon_surface_float), 2)
        close.isEnabled = visibleState != WebChatRealtimeVoiceVisibleState.ENDING
        close.alpha = if (close.isEnabled) 1f else 0.4f
        val actionRequired = visibleState == WebChatRealtimeVoiceVisibleState.FAILED ||
            visibleState == WebChatRealtimeVoiceVisibleState.HANGUP_UNCONFIRMED
        retry.text = if (visibleState == WebChatRealtimeVoiceVisibleState.HANGUP_UNCONFIRMED) {
            "再次挂断"
        } else {
            "重试"
        }
        failureActions.visibility = if (actionRequired) {
            View.VISIBLE
        } else {
            View.GONE
        }
        when (WebChatRealtimeVoiceStatePolicy.expansionDecision(visibleState)) {
            WebChatRealtimeVoiceExpansionDecision.PRESERVE -> Unit
            WebChatRealtimeVoiceExpansionDecision.EXPAND -> setExpanded(true)
            WebChatRealtimeVoiceExpansionDecision.COLLAPSE -> setExpanded(false)
        }
        panel.contentDescription = buildString {
            append(WebChatProductionSelectors.REALTIME_VOICE_SURFACE)
            append('：')
            append(visibleState.label)
            append("，记录到")
            append(context?.label ?: "当前 ChatGPT 会话")
            append("，点按展开")
        }
        if (lastVisibleState != visibleState) panel.announceForAccessibility(status.text)
        lastVisibleState = visibleState
    }

    override fun ensureVisibleOnTop() {
        if (!requestedVisible || !hostVisible) return
        if (root.parent !== host) {
            (root.parent as? ViewGroup)?.removeView(root)
            host.addView(
                root,
                FrameLayout.LayoutParams(
                    ViewGroup.LayoutParams.MATCH_PARENT,
                    ViewGroup.LayoutParams.MATCH_PARENT,
                ),
            )
        }
        root.visibility = View.VISIBLE
        if (host.indexOfChild(root) != host.childCount - 1) root.bringToFront()
        root.post(::positionPanel)
    }

    override fun setHostVisible(visible: Boolean) {
        hostVisible = visible
        if (visible) {
            ensureVisibleOnTop()
        } else {
            root.visibility = View.GONE
        }
    }

    override fun hide() {
        requestedVisible = false
        root.visibility = View.GONE
        close.setOnClickListener(null)
        retry.setOnClickListener(null)
        officialFallback.setOnClickListener(null)
        conversationTarget.setOnClickListener(null)
    }

    override fun isVisible(): Boolean = requestedVisible

    private fun buildContent() {
        collapsedOrb.addView(
            collapsedIcon,
            FrameLayout.LayoutParams(dp(34), dp(34), Gravity.CENTER),
        )
        collapsedOrb.addView(
            collapsedStatus,
            FrameLayout.LayoutParams(dp(12), dp(12), Gravity.END or Gravity.BOTTOM).apply {
                marginEnd = dp(5)
                bottomMargin = dp(5)
            },
        )
        panel.addView(
            collapsedOrb,
            FrameLayout.LayoutParams(metrics.collapsedSize, metrics.collapsedSize),
        )
        textColumn.addView(status)
        textColumn.addView(detail, LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
        ).apply { topMargin = dp(3) })
        expandedHeader.addView(expandedIcon, LinearLayout.LayoutParams(dp(44), dp(44)))
        expandedHeader.addView(
            textColumn,
            LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f),
        )
        expandedHeader.addView(close, LinearLayout.LayoutParams(dp(48), dp(48)))
        expandedCard.addView(expandedHeader)
        expandedCard.addView(
            conversationTarget,
            LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ).apply { topMargin = dp(4) },
        )
        failureActions.addView(
            retry,
            LinearLayout.LayoutParams(0, dp(48), 1f).apply { marginEnd = dp(6) },
        )
        failureActions.addView(
            officialFallback,
            LinearLayout.LayoutParams(0, dp(48), 1f).apply { marginStart = dp(6) },
        )
        expandedCard.addView(failureActions)
        panel.addView(
            expandedCard,
            FrameLayout.LayoutParams(metrics.expandedWidth, ViewGroup.LayoutParams.WRAP_CONTENT),
        )
        root.addView(
            panel,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
                Gravity.TOP or Gravity.START,
            ),
        )
        root.addOnLayoutChangeListener { _, left, top, right, bottom, _, _, _, _ ->
            if (right > left && bottom > top) root.post(::positionPanel)
        }
        panel.addOnLayoutChangeListener { _, _, _, _, _, _, _, _, _ ->
            if (root.visibility == View.VISIBLE) root.post(::positionPanel)
        }
        render(WebChatRealtimeVoiceState(
            lifecycle = WebChatRealtimeVoiceLifecycle.CONNECTING,
            detail = "正在恢复本机网页会话",
        ))
    }

    private fun installInteraction() {
        installDragInteraction(collapsedOrb) { setExpanded(true) }
        installDragInteraction(expandedIcon) { setExpanded(false) }
    }

    private fun installDragInteraction(view: View, onClick: () -> Unit) {
        view.setOnClickListener { onClick() }
        view.setOnTouchListener { target, event ->
            when (event.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    dragging = false
                    downRawX = event.rawX
                    downRawY = event.rawY
                    downLeft = panel.x
                    downTop = panel.y
                    true
                }
                MotionEvent.ACTION_MOVE -> {
                    val deltaX = event.rawX - downRawX
                    val deltaY = event.rawY - downRawY
                    if (!dragging && (abs(deltaX) > touchSlop || abs(deltaY) > touchSlop)) {
                        dragging = true
                    }
                    if (dragging) movePanel(downLeft + deltaX, downTop + deltaY)
                    true
                }
                MotionEvent.ACTION_UP -> {
                    if (!dragging) target.performClick()
                    dragging = false
                    true
                }
                MotionEvent.ACTION_CANCEL -> {
                    dragging = false
                    true
                }
                else -> false
            }
        }
    }

    private fun setExpanded(value: Boolean) {
        if (expanded == value && panel.isLaidOut) return
        expanded = value
        collapsedOrb.visibility = if (value) View.GONE else View.VISIBLE
        expandedCard.visibility = if (value) View.VISIBLE else View.GONE
        panel.isSelected = value
        panel.post(::positionPanel)
    }

    private fun positionPanel() {
        if (root.width <= 0 || root.height <= 0 || panel.width <= 0 || panel.height <= 0) return
        metrics = WebChatRealtimeVoiceFloatingLayoutPolicy.resolve(
            root.width,
            activity.resources.displayMetrics.density,
        )
        if (expandedCard.layoutParams.width != metrics.expandedWidth) {
            expandedCard.layoutParams = (expandedCard.layoutParams as FrameLayout.LayoutParams).apply {
                width = metrics.expandedWidth
            }
        }
        val position = if (!positioned) {
            positioned = true
            WebChatRealtimeVoiceFloatingLayoutPolicy.initialPosition(
                root.width,
                root.height,
                panel.width,
                panel.height,
                metrics.edgeInset,
            )
        } else {
            WebChatRealtimeVoiceFloatingLayoutPolicy.clamp(
                panel.x,
                panel.y,
                root.width,
                root.height,
                panel.width,
                panel.height,
                metrics.edgeInset,
            )
        }
        panel.x = position.left
        panel.y = position.top
    }

    private fun movePanel(left: Float, top: Float) {
        val position = WebChatRealtimeVoiceFloatingLayoutPolicy.clamp(
            left,
            top,
            root.width,
            root.height,
            panel.width,
            panel.height,
            metrics.edgeInset,
        )
        panel.x = position.left
        panel.y = position.top
        positioned = true
    }

    private fun applyVoiceIconStyle(
        container: View,
        icon: ImageView,
        stageColor: Int,
        strokeWidth: Int,
    ) {
        container.background = oval(color(R.color.elon_surface_float), stageColor, strokeWidth)
        icon.imageTintList = ColorStateList.valueOf(stageColor)
    }

    private fun actionButton(label: String, secondary: Boolean = false): TextView =
        textView(
            14f,
            if (secondary) color(R.color.elon_text_primary) else color(R.color.elon_titanium_ink),
            true,
        ).apply {
            text = label
            gravity = Gravity.CENTER
            background = rounded(
                if (secondary) color(R.color.elon_surface_soft) else color(R.color.elon_titanium),
                24,
            )
        }

    private fun textView(size: Float, textColor: Int, bold: Boolean = false): TextView =
        TextView(activity).apply {
            textSize = size
            setTextColor(textColor)
            includeFontPadding = false
            if (bold) typeface = Typeface.DEFAULT_BOLD
        }

    private fun rounded(fillColor: Int, radiusDp: Int): GradientDrawable = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(radiusDp).toFloat()
        setColor(fillColor)
        setStroke(dp(1), color(R.color.elon_border_subtle))
    }

    private fun oval(fillColor: Int, strokeColor: Int? = null, strokeWidth: Int = 0): GradientDrawable =
        GradientDrawable().apply {
            shape = GradientDrawable.OVAL
            setColor(fillColor)
            if (strokeColor != null && strokeWidth > 0) setStroke(dp(strokeWidth), strokeColor)
        }

    private fun color(resource: Int): Int = ContextCompat.getColor(activity, resource)

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()
}
