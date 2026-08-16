package com.elon.app

import android.graphics.Color
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.button.MaterialButton

internal data class WebChatProductionSuggestion(
    val controlId: String,
    val label: String,
    val requiresUserConfirmation: Boolean,
    val nativeSelector: String?,
)

internal object WebChatProductionSuggestionParser {
    fun parse(descriptors: List<WebChatConsumerControlDescriptor>): List<WebChatProductionSuggestion> =
        descriptors.asSequence()
            .filter { descriptor ->
                val control = descriptor.control
                control.enabled &&
                    control.region == SUGGESTIONS_REGION &&
                    control.semantic in SUGGESTION_SEMANTICS &&
                    descriptor.presentation == WebChatConsumerControlPresentation.DIRECT
            }
            .mapNotNull { descriptor ->
                val label = descriptor.control.label.trim()
                if (label.isBlank()) return@mapNotNull null
                WebChatProductionSuggestion(
                    controlId = descriptor.control.id,
                    label = label,
                    requiresUserConfirmation = descriptor.requiresUserConfirmation,
                    nativeSelector = descriptor.nativeSelector?.trim()?.takeIf(String::isNotBlank),
                )
            }
            .distinctBy(WebChatProductionSuggestion::controlId)
            .take(MAX_SUGGESTIONS)
            .toList()

    private const val SUGGESTIONS_REGION = "suggestions"
    private const val MAX_SUGGESTIONS = 4
    private val SUGGESTION_SEMANTICS = setOf("suggestion", "project")
}

internal class WebChatProductionSuggestionsCoordinator(
    private val activity: AppCompatActivity,
) {
    private val row = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
    }
    val view = HorizontalScrollView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(SUGGESTION_STRIP_HEIGHT_DP),
        ).apply {
            marginStart = dp(20)
            marginEnd = dp(20)
            bottomMargin = dp(6)
        }
        isHorizontalScrollBarEnabled = false
        overScrollMode = View.OVER_SCROLL_NEVER
        clipToPadding = false
        contentDescription = WebChatProductionSelectors.SUGGESTIONS
        visibility = View.GONE
        addView(row)
    }
    private var renderKey: String? = null

    fun attach(parent: ViewGroup, index: Int) {
        if (view.parent === parent) return
        (view.parent as? ViewGroup)?.removeView(view)
        parent.addView(view, index.coerceIn(0, parent.childCount))
    }

    fun render(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort?,
    ) {
        if (!provider.supports(WebChatProviderCapability.PROMPT_SUGGESTIONS) || port == null) {
            hide()
            return
        }
        val suggestions = WebChatProductionSuggestionParser.parse(port.state().controls)
        val nextKey = buildString {
            append(provider.id.wireValue)
            suggestions.forEach { suggestion ->
                append('|').append(suggestion.controlId)
                append(':').append(suggestion.label)
                append(':').append(suggestion.requiresUserConfirmation)
            }
        }
        if (renderKey == nextKey) {
            view.visibility = if (suggestions.isEmpty()) View.GONE else View.VISIBLE
            return
        }
        renderKey = nextKey
        row.removeAllViews()
        suggestions.forEach { suggestion ->
            row.addView(suggestionButton(provider, port, suggestion))
        }
        view.visibility = if (suggestions.isEmpty()) View.GONE else View.VISIBLE
    }

    fun hide() {
        renderKey = null
        row.removeAllViews()
        view.visibility = View.GONE
    }

    private fun suggestionButton(
        provider: WebChatProviderIdentity,
        port: WebChatConsumerPort,
        suggestion: WebChatProductionSuggestion,
    ) = MaterialButton(
        activity,
        null,
        com.google.android.material.R.attr.materialButtonOutlinedStyle,
    ).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.WRAP_CONTENT,
            dp(SUGGESTION_HEIGHT_DP),
        ).apply { marginEnd = dp(8) }
        minWidth = 0
        minimumWidth = 0
        maxWidth = dp(MAX_SUGGESTION_WIDTH_DP)
        insetTop = 0
        insetBottom = 0
        cornerRadius = dp(8)
        strokeWidth = dp(1)
        strokeColor = activity.getColorStateList(R.color.elon_border_primary)
        backgroundTintList = activity.getColorStateList(R.color.elon_surface_card)
        rippleColor = android.content.res.ColorStateList.valueOf(Color.TRANSPARENT)
        setTextColor(activity.getColor(R.color.elon_text_primary))
        setPadding(dp(14), 0, dp(14), 0)
        text = suggestion.label
        textSize = 14f
        isAllCaps = false
        maxLines = 1
        contentDescription = suggestion.nativeSelector
            ?: WebChatProductionSelectors.suggestion(provider.id, suggestion.controlId)
        setOnClickListener {
            if (suggestion.requiresUserConfirmation) {
                confirmSuggestion(port, suggestion)
            } else {
                invokeSuggestion(port, suggestion, userConfirmed = false)
            }
        }
    }

    private fun confirmSuggestion(
        port: WebChatConsumerPort,
        suggestion: WebChatProductionSuggestion,
    ) {
        AlertDialog.Builder(activity)
            .setTitle(suggestion.label)
            .setMessage("此操作需要你的确认，是否继续？")
            .setPositiveButton("继续") { _, _ ->
                invokeSuggestion(port, suggestion, userConfirmed = true)
            }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun invokeSuggestion(
        port: WebChatConsumerPort,
        suggestion: WebChatProductionSuggestion,
        userConfirmed: Boolean,
    ) {
        val result = port.invokeControl(suggestion.controlId, userConfirmed)
        if (result.accepted) return
        if (result.error == "stale_control_id") port.requestControls()
        val message = when (result.error) {
            "stale_control_id" -> "建议内容已更新，请稍后重试"
            "user_confirmation_required" -> "需要确认后才能继续"
            "bridge_not_ready", "adapter_not_current" -> "网页正在恢复，请稍后重试"
            else -> "当前建议暂时不可用"
        }
        Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val SUGGESTION_STRIP_HEIGHT_DP = 50
        const val SUGGESTION_HEIGHT_DP = 44
        const val MAX_SUGGESTION_WIDTH_DP = 260
    }
}
