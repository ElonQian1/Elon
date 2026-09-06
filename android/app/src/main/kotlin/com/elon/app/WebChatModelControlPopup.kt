package com.elon.app

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.ScrollView
import android.widget.SeekBar
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat

internal class WebChatModelControlPopupHandle(
    private val renderer: WebChatModelControlPopupRenderer,
) {
    fun update(options: List<WebChatConsumerOption>, currentModel: String) {
        renderer.render(options, currentModel)
    }

    fun dismiss() = renderer.dismiss()
}

internal object WebChatModelControlPopup {
    fun show(
        activity: AppCompatActivity,
        anchor: View,
        options: List<WebChatConsumerOption>,
        currentModel: String,
        onOptionSelected: (WebChatConsumerOption) -> Unit,
        onProviderSwitch: () -> Unit,
        onDismissed: () -> Unit,
    ): WebChatModelControlPopupHandle? {
        if (activity.isFinishing || activity.isDestroyed || !anchor.isAttachedToWindow) return null
        val renderer = WebChatModelControlPopupRenderer(
            activity = activity,
            anchor = anchor,
            onOptionSelected = onOptionSelected,
            onProviderSwitch = onProviderSwitch,
            onDismissed = onDismissed,
        )
        renderer.show(options, currentModel)
        return WebChatModelControlPopupHandle(renderer)
    }
}

internal class WebChatModelControlPopupRenderer(
    private val activity: AppCompatActivity,
    private val anchor: View,
    private val onOptionSelected: (WebChatConsumerOption) -> Unit,
    private val onProviderSwitch: () -> Unit,
    private val onDismissed: () -> Unit,
) {
    private val popupWidth = dp(280)
    private val panel = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(12), dp(10), dp(12), dp(10))
        background = roundedBackground(R.color.elon_project_space_info_bg, 16)
        contentDescription = MODEL_CONTROL_SELECTOR
        elevation = dp(8).toFloat()
    }
    private val scroll = ScrollView(activity).apply {
        isFillViewport = true
        addView(panel)
    }
    private val popup = PopupWindow(
        scroll,
        popupWidth,
        ViewGroup.LayoutParams.WRAP_CONTENT,
        true,
    ).apply {
        isOutsideTouchable = true
        elevation = dp(10).toFloat()
        setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
        setOnDismissListener { onDismissed() }
    }

    fun show(options: List<WebChatConsumerOption>, currentModel: String) {
        render(options, currentModel)
        positionPopup(show = true)
        panel.alpha = 0f
        panel.scaleX = 0.97f
        panel.scaleY = 0.97f
        panel.pivotX = popupWidth.toFloat()
        panel.pivotY = popup.height.toFloat()
        panel.animate().alpha(1f).scaleX(1f).scaleY(1f).setDuration(120L).start()
    }

    private fun positionPopup(show: Boolean) {
        panel.measure(
            View.MeasureSpec.makeMeasureSpec(popupWidth, View.MeasureSpec.EXACTLY),
            View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED),
        )
        val location = IntArray(2)
        anchor.getLocationOnScreen(location)
        val screenWidth = activity.resources.displayMetrics.widthPixels
        val x = (location[0] + anchor.width - popupWidth)
            .coerceIn(dp(12), screenWidth - popupWidth - dp(12))
        val height = panel.measuredHeight.coerceAtMost((location[1] - dp(82)).coerceAtLeast(dp(100)))
        val y = (location[1] - height - dp(10)).coerceAtLeast(dp(72))
        if (show) {
            popup.height = height
            popup.showAtLocation(anchor, Gravity.NO_GRAVITY, x, y)
        } else popup.update(x, y, popupWidth, height)
    }

    fun render(options: List<WebChatConsumerOption>, currentModel: String) {
        panel.removeAllViews()
        val presentation = WebChatModelControlPolicy.resolve(options, currentModel)
        presentation.advanced?.let { advanced ->
            panel.addView(actionRow(
                label = advanced.label,
                selector = ADVANCED_SELECTOR,
                showChevron = true,
            ) {
                onOptionSelected(advanced)
            })
        }
        when {
            presentation.usesLevelSlider -> panel.addView(levelSlider(presentation))
            presentation.listOptions.isNotEmpty() -> presentation.listOptions.forEach { option ->
                panel.addView(optionRow(option, currentModel))
            }
            presentation.advanced == null -> panel.addView(presetRow())
        }
        panel.addView(divider())
        panel.addView(actionRow(
            label = "切换网页 AI",
            selector = PROVIDER_SWITCH_SELECTOR,
            showChevron = true,
        ) {
            popup.dismiss()
            onProviderSwitch()
        })
        scroll.scrollTo(0, 0)
        if (popup.isShowing) positionPopup(show = false)
    }

    fun dismiss() = popup.dismiss()

    private fun levelSlider(presentation: WebChatModelControlPresentation): View {
        val levels = presentation.levels
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(8), dp(2), dp(8), dp(6))
            addView(SeekBar(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    dp(52),
                )
                max = levels.lastIndex
                progress = presentation.selectedLevelIndex.coerceIn(0, max)
                splitTrack = false
                progressTintList = colorStateList(R.color.elon_accent_primary)
                progressBackgroundTintList = colorStateList(R.color.elon_text_tertiary)
                thumbTintList = colorStateList(R.color.elon_text_primary)
                tickMark = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    setSize(dp(4), dp(4))
                    setColor(ContextCompat.getColor(activity, R.color.elon_text_secondary))
                }
                tickMarkTintList = colorStateList(R.color.elon_text_secondary)
                contentDescription = LEVEL_SLIDER_SELECTOR
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    stateDescription = WebChatModelControlPolicy.compactLabel(levels[progress].label)
                }
                setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
                    override fun onProgressChanged(bar: SeekBar?, value: Int, fromUser: Boolean) {
                        val selected = levels.getOrNull(value) ?: return
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                            stateDescription = WebChatModelControlPolicy.compactLabel(selected.label)
                        }
                    }

                    override fun onStartTrackingTouch(bar: SeekBar?) = Unit

                    override fun onStopTrackingTouch(bar: SeekBar?) {
                        levels.getOrNull(progress)?.let(onOptionSelected)
                    }
                })
            })
        }
    }

    private fun optionRow(option: WebChatConsumerOption, currentModel: String): View = actionRow(
        label = option.label,
        selector = option.nativeSelector.ifBlank { "web-chat-model-option:${option.id}" },
        trailing = if (WebChatModelControlPolicy.isSelected(option, currentModel)) "✓" else null,
        showChevron = option.opensSubmenu,
    ) {
        onOptionSelected(option)
    }

    private fun actionRow(
        label: String,
        selector: String,
        trailing: String? = null,
        showChevron: Boolean,
        action: () -> Unit,
    ) = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(50),
        )
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(8), 0, dp(8), 0)
        isClickable = true
        isFocusable = true
        contentDescription = selector
        foreground = activity.obtainStyledAttributes(intArrayOf(android.R.attr.selectableItemBackground))
            .let { values -> values.getDrawable(0).also { values.recycle() } }
        setOnClickListener { action() }
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            includeFontPadding = false
            maxLines = 1
            text = label
            textSize = 16f
            setTextColor(ContextCompat.getColor(activity, R.color.elon_text_primary))
        })
        when {
            trailing != null -> addView(TextView(activity).apply {
                includeFontPadding = false
                text = trailing
                textSize = 17f
                setTextColor(ContextCompat.getColor(activity, R.color.elon_accent_primary))
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
            })
            showChevron -> addView(ImageView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(18), dp(18))
                setImageResource(R.drawable.ic_input_chevron_new)
                imageTintList = colorStateList(R.color.elon_text_secondary)
                rotation = -90f
                importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
            })
        }
    }

    private fun presetRow() = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(58),
        )
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(8), 0, dp(8), 0)
        includeFontPadding = false
        text = "自动"
        textSize = 15f
        setTextColor(ContextCompat.getColor(activity, R.color.elon_text_secondary))
        contentDescription = "web-chat-model-preset:auto"
    }

    private fun divider() = View(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(1),
        ).apply {
            marginStart = dp(8)
            marginEnd = dp(8)
        }
        setBackgroundColor(ContextCompat.getColor(activity, R.color.elon_store_detail_divider))
        alpha = 0.45f
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }

    private fun roundedBackground(colorRes: Int, radiusDp: Int) = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(radiusDp).toFloat()
        setColor(ContextCompat.getColor(activity, colorRes))
    }

    private fun colorStateList(colorRes: Int) = ColorStateList.valueOf(
        ContextCompat.getColor(activity, colorRes),
    )

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val MODEL_CONTROL_SELECTOR = "web-chat-model-control"
        const val ADVANCED_SELECTOR = "web-chat-model-advanced"
        const val LEVEL_SLIDER_SELECTOR = "web-chat-model-level-slider"
        const val PROVIDER_SWITCH_SELECTOR = "web-chat-provider-switch"
    }
}
