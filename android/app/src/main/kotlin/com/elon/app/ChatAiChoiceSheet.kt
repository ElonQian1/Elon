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
import android.widget.ScrollView
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.bottomsheet.BottomSheetBehavior
import com.google.android.material.bottomsheet.BottomSheetDialog
import com.google.android.material.materialswitch.MaterialSwitch

internal data class ChatAiChoice(val id: String, val title: String, val subtitle: String,
    val avatarResId: Int, val selected: Boolean, val selector: String, val enabled: Boolean = true)
internal data class ChatAiSheetAction(val label: String, val selector: String, val invoke: () -> Unit)
internal data class ChatAiSheetToggle(val label: String, val selector: String, val checked: Boolean,
    val change: (Boolean) -> Unit)
internal class ChatAiChoiceSheetHandle(val dialog: BottomSheetDialog, val update: (List<ChatAiChoice>) -> Unit)

/** Presentation only: callers own provider identities, scope, persistence and execution. */
internal object ChatAiChoiceSheet {
    fun show(
        activity: AppCompatActivity,
        title: String,
        options: List<ChatAiChoice>,
        onSelected: (String) -> Unit,
        actions: List<ChatAiSheetAction> = emptyList(),
        toggle: ChatAiSheetToggle? = null,
    ): ChatAiChoiceSheetHandle? {
        if (activity.isFinishing || activity.isDestroyed || options.isEmpty()) return null
        val dialog = BottomSheetDialog(activity)
        val rows = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL }
        val update: (List<ChatAiChoice>) -> Unit = { choices ->
            rows.removeAllViews()
            choices.forEach { option -> rows.addView(providerRow(activity, option) {
                dialog.dismiss()
                if (option.enabled) onSelected(option.id)
            }) }
        }
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(activity, 20), dp(activity, 12), dp(activity, 20), dp(activity, 20))
            background = roundedBackground(activity, PANEL_COLOR, 18)
            addView(dragHandle(activity))
            addView(title(activity, title))
            addView(rows)
            update(options)
            toggle?.let { setting ->
                addView(MaterialSwitch(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(-1, -2)
                    minHeight = dp(activity, 52)
                    text = setting.label
                    textSize = 14f
                    setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
                    contentDescription = setting.selector
                    isChecked = setting.checked
                    setOnCheckedChangeListener { _, checked -> setting.change(checked) }
                })
            }
            if (actions.isNotEmpty()) addView(actionRow(activity, actions) { action ->
                dialog.dismiss()
                action.invoke()
            })
        }
        dialog.setContentView(ScrollView(activity).apply { isFillViewport = true; addView(root) })
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
        return ChatAiChoiceSheetHandle(dialog, update)
    }

    private fun dragHandle(activity: AppCompatActivity) = View(activity).apply {
        layoutParams = LinearLayout.LayoutParams(dp(activity, 36), dp(activity, 4)).apply {
            gravity = Gravity.CENTER_HORIZONTAL
            bottomMargin = dp(activity, 12)
        }
        background = roundedBackground(activity, HANDLE_COLOR, 2)
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }

    private fun title(activity: AppCompatActivity, label: String) = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(activity, 44),
        )
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        text = label
        textSize = 20f
        setTypeface(typeface, Typeface.BOLD)
        setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
    }

    private fun providerRow(
        activity: AppCompatActivity,
        option: ChatAiChoice,
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
        isClickable = option.enabled
        isFocusable = option.enabled
        isEnabled = option.enabled
        alpha = if (option.enabled) 1f else 0.6f
        contentDescription = option.selector
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
                ellipsize = android.text.TextUtils.TruncateAt.END
                text = option.title
                textSize = 16f
                setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
            })
            addView(TextView(activity).apply {
                includeFontPadding = false
                maxLines = 1
                ellipsize = android.text.TextUtils.TruncateAt.END
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
        actions: List<ChatAiSheetAction>,
        invoke: (ChatAiSheetAction) -> Unit,
    ) = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(activity, 52),
        ).apply { topMargin = dp(activity, 6) }
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        actions.forEach { action ->
            addView(actionButton(activity, action.label, action.selector) { invoke(action) },
                LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f))
        }
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
