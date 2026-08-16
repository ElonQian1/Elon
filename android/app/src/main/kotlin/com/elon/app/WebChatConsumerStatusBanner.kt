package com.elon.app

import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal data class WebChatConsumerRecoveryState(
    val visible: Boolean,
    val message: String,
    val retryVisible: Boolean,
    val officialVisible: Boolean,
    val retryLabel: String = "重试",
    val officialLabel: String = "官网",
)

internal object WebChatConsumerRecoveryPolicy {
    fun resolve(
        provider: WebChatProviderIdentity,
        state: String,
        detail: String? = null,
        hasConversationContent: Boolean = false,
    ): WebChatConsumerRecoveryState = when (state) {
        "loading" -> if (hasConversationContent) {
            WebChatConsumerRecoveryState(
                visible = true,
                message = "正在重新连接${provider.displayName}，当前对话已保留",
                retryVisible = false,
                officialVisible = false,
            )
        } else {
            hidden()
        }
        "error" -> WebChatConsumerRecoveryState(
            visible = true,
            message = errorMessage(provider, detail),
            retryVisible = true,
            officialVisible = true,
        )
        "login_required" -> WebChatConsumerRecoveryState(
            visible = true,
            message = "可尝试免费访客聊天，或登录账号",
            retryVisible = true,
            officialVisible = true,
            retryLabel = "访客",
            officialLabel = "登录",
        )
        else -> hidden()
    }

    private fun errorMessage(provider: WebChatProviderIdentity, detail: String?): String {
        val normalized = detail.orEmpty().lowercase()
        return when {
            "err_name_not_resolved" in normalized || "err_internet_disconnected" in normalized ->
                "网络不可用，请检查加速网络后重试"
            "err_timed_out" in normalized || "timeout" in normalized || "超时" in normalized ->
                "${provider.displayName}连接超时，请重试"
            "webview" in normalized && ("不支持" in normalized || "unsupported" in normalized) ->
                "系统 WebView 版本不支持网页聊天，请更新后重试"
            "导航被拦截" in normalized || "blocked" in normalized ->
                "官网跳转未完成，请打开官网继续"
            else -> "${provider.displayName}连接异常"
        }
    }

    private fun hidden() = WebChatConsumerRecoveryState(
        visible = false,
        message = "",
        retryVisible = false,
        officialVisible = false,
    )
}

internal fun WebChatSocialController.consumerRecoveryState(
    provider: WebChatProviderIdentity,
): WebChatConsumerRecoveryState = WebChatConsumerRecoveryPolicy.resolve(
    provider = provider,
    state = stateWireValue(),
    detail = stateDetail(),
    hasConversationContent = currentMessages().any { it.webChatMessage != null },
)

internal class WebChatConsumerStatusBanner(
    activity: AppCompatActivity,
    private val onRetry: () -> Unit,
    private val onOfficialPage: () -> Unit,
) : LinearLayout(activity) {
    private val messageView = TextView(activity).apply {
        layoutParams = LayoutParams(0, LayoutParams.MATCH_PARENT, 1f)
        gravity = Gravity.CENTER_VERTICAL or Gravity.START
        includeFontPadding = false
        maxLines = 2
        textSize = 13f
        setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
    }
    private val retryButton = actionButton(activity, "重试", RETRY_SELECTOR, onRetry)
    private val officialButton = actionButton(activity, "官网", OFFICIAL_SELECTOR, onOfficialPage)

    init {
        layoutParams = LayoutParams(LayoutParams.MATCH_PARENT, dp(activity, 48)).apply {
            marginStart = dp(activity, 20)
            marginEnd = dp(activity, 20)
            bottomMargin = dp(activity, 6)
        }
        orientation = HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(activity, 14), 0, dp(activity, 6), 0)
        background = roundedBackground(activity, PANEL_COLOR, 8)
        contentDescription = STATUS_SELECTOR
        addView(messageView)
        addView(retryButton)
        addView(officialButton)
        visibility = View.GONE
    }

    fun render(state: WebChatConsumerRecoveryState) {
        visibility = if (state.visible) View.VISIBLE else View.GONE
        if (!state.visible) return
        messageView.text = state.message
        retryButton.text = state.retryLabel
        officialButton.text = state.officialLabel
        retryButton.visibility = if (state.retryVisible) View.VISIBLE else View.GONE
        officialButton.visibility = if (state.officialVisible) View.VISIBLE else View.GONE
    }

    fun hide() {
        visibility = View.GONE
    }

    private fun actionButton(
        activity: AppCompatActivity,
        label: String,
        description: String,
        action: () -> Unit,
    ) = TextView(activity).apply {
        layoutParams = LayoutParams(dp(activity, 56), LayoutParams.MATCH_PARENT)
        gravity = Gravity.CENTER
        includeFontPadding = false
        text = label
        textSize = 13f
        setTextColor(Color.parseColor(ACCENT_COLOR))
        contentDescription = description
        isClickable = true
        isFocusable = true
        setOnClickListener { action() }
    }

    private fun roundedBackground(
        activity: AppCompatActivity,
        color: String,
        radiusDp: Int,
    ) = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(activity, radiusDp).toFloat()
        setColor(Color.parseColor(color))
    }

    private fun dp(activity: AppCompatActivity, value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val STATUS_SELECTOR = "web-chat-consumer-status"
        const val RETRY_SELECTOR = "web-chat-consumer-retry"
        const val OFFICIAL_SELECTOR = "web-chat-consumer-open-official"
        const val PANEL_COLOR = "#1D1E22"
        const val PRIMARY_TEXT_COLOR = "#F8F7F4"
        const val ACCENT_COLOR = "#9EB6DE"
    }
}
