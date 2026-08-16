package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.bottomsheet.BottomSheetBehavior
import com.google.android.material.bottomsheet.BottomSheetDialog

internal object WebChatProviderPickerSheet {
    fun show(
        activity: AppCompatActivity,
        options: List<WebChatProviderPickerOption>,
        onProviderSelected: (WebChatProviderId) -> Boolean,
        onModelOptions: () -> Unit,
        onOfficialPage: () -> Unit,
    ) {
        if (activity.isFinishing || activity.isDestroyed || options.isEmpty()) return
        val dialog = BottomSheetDialog(activity)
        val selectedProvider = options.firstOrNull(WebChatProviderPickerOption::selected)?.providerId
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(activity, 20), dp(activity, 12), dp(activity, 20), dp(activity, 20))
            background = roundedBackground(activity, PANEL_COLOR, 18)
            addView(dragHandle(activity))
            addView(title(activity))
            options.forEach { option ->
                addView(providerRow(activity, option) {
                    if (!option.selected) onProviderSelected(option.providerId)
                    dialog.dismiss()
                })
            }
            addView(actionRow(
                activity = activity,
                showModelAction = selectedProvider == WebChatProviderId.CHATGPT_WEB,
                onModelOptions = {
                    dialog.dismiss()
                    onModelOptions()
                },
                onOfficialPage = {
                    dialog.dismiss()
                    onOfficialPage()
                },
            ))
        }
        dialog.setContentView(root)
        dialog.setOnShowListener {
            dialog.findViewById<FrameLayout>(com.google.android.material.R.id.design_bottom_sheet)?.let { sheet ->
                sheet.setBackgroundColor(Color.TRANSPARENT)
                BottomSheetBehavior.from(sheet).apply {
                    state = BottomSheetBehavior.STATE_EXPANDED
                    skipCollapsed = true
                }
            }
        }
        dialog.show()
    }

    private fun dragHandle(activity: AppCompatActivity) = View(activity).apply {
        layoutParams = LinearLayout.LayoutParams(dp(activity, 36), dp(activity, 4)).apply {
            gravity = Gravity.CENTER_HORIZONTAL
            bottomMargin = dp(activity, 12)
        }
        background = roundedBackground(activity, HANDLE_COLOR, 2)
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }

    private fun title(activity: AppCompatActivity) = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(activity, 44),
        )
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        text = activity.getString(R.string.web_chat_provider_picker_title)
        textSize = 20f
        setTypeface(typeface, Typeface.BOLD)
        setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
    }

    private fun providerRow(
        activity: AppCompatActivity,
        option: WebChatProviderPickerOption,
        onClick: () -> Unit,
    ) = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(activity, 72),
        ).apply { bottomMargin = dp(activity, 6) }
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(activity, 12), 0, dp(activity, 12), 0)
        background = if (option.selected) {
            roundedBackground(activity, SELECTED_ROW_COLOR, 8)
        } else {
            roundedBackground(activity, PANEL_COLOR, 8)
        }
        isClickable = true
        isFocusable = true
        contentDescription = "web-chat-provider:${option.providerId.wireValue}:${if (option.selected) "selected" else "idle"}"
        setOnClickListener { onClick() }
        addView(ImageView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(activity, 42), dp(activity, 42)).apply {
                marginEnd = dp(activity, 14)
            }
            setImageResource(option.avatarResId)
            scaleType = ImageView.ScaleType.CENTER_CROP
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        })
        addView(LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            orientation = LinearLayout.VERTICAL
            addView(TextView(activity).apply {
                includeFontPadding = false
                maxLines = 1
                text = option.title
                textSize = 16f
                setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
            })
            addView(TextView(activity).apply {
                includeFontPadding = false
                maxLines = 1
                text = option.subtitle
                textSize = 12f
                setPadding(0, dp(activity, 5), 0, 0)
                setTextColor(Color.parseColor(SECONDARY_TEXT_COLOR))
            })
        })
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(activity, 30), dp(activity, 30))
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = if (option.selected) "✓" else ""
            textSize = 18f
            setTextColor(Color.parseColor(ACCENT_COLOR))
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        })
    }

    private fun actionRow(
        activity: AppCompatActivity,
        showModelAction: Boolean,
        onModelOptions: () -> Unit,
        onOfficialPage: () -> Unit,
    ) = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(activity, 52),
        ).apply { topMargin = dp(activity, 6) }
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        if (showModelAction) addView(actionButton(
            activity,
            activity.getString(R.string.web_chat_provider_model_action),
            "web-chat-provider-model-options",
            onModelOptions,
        ), LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
        addView(actionButton(
            activity,
            activity.getString(R.string.web_chat_open_official),
            "web-chat-provider-official",
            onOfficialPage,
        ), LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
    }

    private fun actionButton(
        activity: AppCompatActivity,
        label: String,
        description: String,
        onClick: () -> Unit,
    ) = TextView(activity).apply {
        gravity = Gravity.CENTER
        includeFontPadding = false
        text = label
        textSize = 14f
        setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
        contentDescription = description
        isClickable = true
        isFocusable = true
        setOnClickListener { onClick() }
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

    private const val PANEL_COLOR = "#17181B"
    private const val SELECTED_ROW_COLOR = "#2A2B30"
    private const val HANDLE_COLOR = "#5E6067"
    private const val PRIMARY_TEXT_COLOR = "#F8F7F4"
    private const val SECONDARY_TEXT_COLOR = "#8F9299"
    private const val ACCENT_COLOR = "#8EA7D5"
}
