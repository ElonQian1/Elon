package com.elon.app.chatgptweb

import android.graphics.Color
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.HorizontalScrollView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.AppCompatImageButton
import com.elon.app.R
import com.google.android.material.button.MaterialButton

internal class ChatGptNativeAdaptiveUiController(
    private val activity: AppCompatActivity,
    private val titleView: TextView,
    private val headerActionsScroll: HorizontalScrollView,
    private val headerActions: LinearLayout,
    private val suggestions: LinearLayout,
    private val onSuggestionsVisibleChanged: (Boolean) -> Unit,
    private val onInvoke: (String) -> Unit,
) {
    private var manifest: ChatGptWebUiManifest? = null

    fun render(value: ChatGptWebUiManifest) {
        manifest = value
        titleView.text = value.title.ifBlank { activity.getString(R.string.chatgpt_native_title) }
        val titleControl = value.controls.firstOrNull {
            it.region == ChatGptWebUiRegion.HEADER && it.semantic == "title"
        }
        titleView.isClickable = titleControl != null
        titleView.isFocusable = titleControl != null
        titleView.contentDescription = titleControl?.accessibilityLabel ?: titleView.text
        titleView.setOnClickListener(titleControl?.let { control -> View.OnClickListener { onInvoke(control.id) } })

        renderHeaderActions(value.controls)
        renderSuggestions(value.controls)
    }

    fun snapshot(): ChatGptWebUiManifest? = manifest

    private fun renderHeaderActions(controls: List<ChatGptWebUiControl>) {
        headerActions.removeAllViews()
        ChatGptNativeControlPresentation.headerActions(controls)
            .asSequence()
            .forEach { control -> headerActions.addView(headerActionView(control)) }
        headerActionsScroll.visibility = if (headerActions.childCount > 0) View.VISIBLE else View.GONE
    }

    private fun renderSuggestions(controls: List<ChatGptWebUiControl>) {
        suggestions.removeAllViews()
        ChatGptNativeControlPresentation.suggestions(controls)
            .asSequence()
            .forEach { control -> suggestions.addView(suggestionButton(control)) }
        val visible = suggestions.childCount > 0
        suggestions.visibility = if (visible) View.VISIBLE else View.GONE
        onSuggestionsVisibleChanged(visible)
    }

    private fun compactButton(control: ChatGptWebUiControl): MaterialButton =
        MaterialButton(activity, null, com.google.android.material.R.attr.materialButtonOutlinedStyle).apply {
            layoutParams = LinearLayout.LayoutParams(dp(76), dp(42))
            minWidth = dp(76)
            minimumWidth = 0
            insetTop = 0
            insetBottom = 0
            strokeWidth = 0
            setPadding(dp(10), 0, dp(10), 0)
            text = control.label.take(MAX_COMPACT_LABEL_LENGTH)
            textSize = 13f
            maxLines = 1
            ellipsize = TextUtils.TruncateAt.END
            isAllCaps = false
            isEnabled = control.enabled
            contentDescription = control.accessibilityLabel
            tag = control.id
            setOnClickListener { onInvoke(control.id) }
        }

    private fun headerActionView(control: ChatGptWebUiControl): View =
        if (ChatGptNativeControlPresentation.usesHeaderIcon(control)) {
            AppCompatImageButton(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(44), dp(42))
                background = null
                setImageResource(R.drawable.ic_side_menu_files)
                imageTintList = activity.getColorStateList(R.color.elon_icon_primary)
                setPadding(dp(10), dp(10), dp(10), dp(10))
                isEnabled = control.enabled
                contentDescription = control.accessibilityLabel
                tooltipText = control.label
                tag = control.id
                setOnClickListener { onInvoke(control.id) }
            }
        } else {
            compactButton(control)
        }

    private fun suggestionButton(control: ChatGptWebUiControl): MaterialButton =
        MaterialButton(activity, null, com.google.android.material.R.attr.materialButtonOutlinedStyle).apply {
            layoutParams = LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, dp(50)).apply {
                bottomMargin = dp(8)
            }
            gravity = Gravity.CENTER_VERTICAL or Gravity.START
            cornerRadius = dp(8)
            strokeColor = activity.getColorStateList(R.color.elon_border_primary)
            backgroundTintList = activity.getColorStateList(R.color.elon_surface_card)
            setTextColor(activity.getColor(R.color.elon_text_primary))
            rippleColor = android.content.res.ColorStateList.valueOf(Color.TRANSPARENT)
            text = control.label
            textSize = 14f
            isAllCaps = false
            isEnabled = control.enabled
            contentDescription = control.accessibilityLabel
            tag = control.id
            setOnClickListener { onInvoke(control.id) }
        }

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val MAX_COMPACT_LABEL_LENGTH = 8
    }
}
