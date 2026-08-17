package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.bottomsheet.BottomSheetBehavior
import com.google.android.material.bottomsheet.BottomSheetDialog

internal data class WebChatActionSheetItem(
    val id: String,
    val title: String,
    val subtitle: String? = null,
    val selected: Boolean = false,
    val enabled: Boolean = true,
    val contentDescription: String,
)

internal data class WebChatActionSheetFooterAction(
    val label: String,
    val contentDescription: String,
    val action: () -> Unit,
)

internal class WebChatActionSheetHandle(
    val dialog: BottomSheetDialog,
    private val replaceItems: (List<WebChatActionSheetItem>) -> Unit,
) {
    fun updateItems(items: List<WebChatActionSheetItem>) {
        if (items.isNotEmpty()) replaceItems(items)
    }

    fun dismiss() = dialog.dismiss()
}

internal object WebChatActionSheet {
    fun show(
        activity: AppCompatActivity,
        title: String,
        items: List<WebChatActionSheetItem>,
        footerActions: List<WebChatActionSheetFooterAction> = emptyList(),
        onCancelled: () -> Unit = {},
        onDismissed: () -> Unit = {},
        onSelected: (WebChatActionSheetItem) -> Unit,
    ): BottomSheetDialog? = showUpdatable(
        activity = activity,
        title = title,
        items = items,
        footerActions = footerActions,
        onCancelled = onCancelled,
        onDismissed = onDismissed,
        onSelected = onSelected,
    )?.dialog

    fun showUpdatable(
        activity: AppCompatActivity,
        title: String,
        items: List<WebChatActionSheetItem>,
        footerActions: List<WebChatActionSheetFooterAction> = emptyList(),
        onCancelled: () -> Unit = {},
        onDismissed: () -> Unit = {},
        onSelected: (WebChatActionSheetItem) -> Unit,
    ): WebChatActionSheetHandle? {
        if (activity.isFinishing || activity.isDestroyed || items.isEmpty()) return null
        val dialog = BottomSheetDialog(activity)
        var handled = false
        val itemContainer = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
        }
        val itemScroll = ScrollView(activity).apply {
            isVerticalScrollBarEnabled = false
            overScrollMode = View.OVER_SCROLL_IF_CONTENT_SCROLLS
            addView(itemContainer)
        }
        fun renderItems(updatedItems: List<WebChatActionSheetItem>) {
            itemScroll.layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(activity, (updatedItems.size * ITEM_HEIGHT_DP).coerceAtMost(MAX_LIST_HEIGHT_DP)),
            )
            itemContainer.removeAllViews()
            updatedItems.forEach { item ->
                itemContainer.addView(itemRow(activity, item) {
                    if (!item.enabled) return@itemRow
                    handled = true
                    onSelected(item)
                    dialog.dismiss()
                })
            }
        }
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(activity, 20), dp(activity, 12), dp(activity, 20), dp(activity, 18))
            background = roundedBackground(activity, PANEL_COLOR, 8)
            addView(dragHandle(activity))
            addView(sheetTitle(activity, title))
            addView(itemScroll)
            if (footerActions.isNotEmpty()) addView(footer(activity, footerActions) { action ->
                handled = true
                action.action()
                dialog.dismiss()
            })
        }
        renderItems(items)
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
        dialog.setOnDismissListener {
            if (!handled) onCancelled()
            onDismissed()
        }
        dialog.show()
        return WebChatActionSheetHandle(dialog) { updatedItems ->
            if (dialog.isShowing) itemScroll.post { renderItems(updatedItems) }
        }
    }

    private fun dragHandle(activity: AppCompatActivity) = View(activity).apply {
        layoutParams = LinearLayout.LayoutParams(dp(activity, 36), dp(activity, 4)).apply {
            gravity = Gravity.CENTER_HORIZONTAL
            bottomMargin = dp(activity, 12)
        }
        background = roundedBackground(activity, HANDLE_COLOR, 2)
        importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
    }

    private fun sheetTitle(activity: AppCompatActivity, value: String) = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(activity, 46),
        )
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        text = value
        textSize = 20f
        setTypeface(typeface, Typeface.BOLD)
        setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
    }

    private fun itemRow(
        activity: AppCompatActivity,
        item: WebChatActionSheetItem,
        onClick: () -> Unit,
    ) = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(activity, ITEM_HEIGHT_DP),
        ).apply { bottomMargin = dp(activity, 4) }
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        setPadding(dp(activity, 14), 0, dp(activity, 12), 0)
        background = roundedBackground(
            activity,
            if (item.selected) SELECTED_ROW_COLOR else PANEL_COLOR,
            8,
        )
        alpha = if (item.enabled) 1f else 0.45f
        isClickable = item.enabled
        isFocusable = item.enabled
        contentDescription = item.contentDescription
        setOnClickListener(if (item.enabled) View.OnClickListener { onClick() } else null)
        addView(LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
            orientation = LinearLayout.VERTICAL
            addView(TextView(activity).apply {
                includeFontPadding = false
                maxLines = 1
                text = item.title
                textSize = 16f
                setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
            })
            item.subtitle?.takeIf(String::isNotBlank)?.let { detail ->
                addView(TextView(activity).apply {
                    includeFontPadding = false
                    maxLines = 1
                    text = detail
                    textSize = 12f
                    setPadding(0, dp(activity, 5), 0, 0)
                    setTextColor(Color.parseColor(SECONDARY_TEXT_COLOR))
                })
            }
        })
        addView(TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(dp(activity, 32), dp(activity, 32))
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = if (item.selected) "✓" else ""
            textSize = 18f
            setTextColor(Color.parseColor(ACCENT_COLOR))
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        })
    }

    private fun footer(
        activity: AppCompatActivity,
        actions: List<WebChatActionSheetFooterAction>,
        onClick: (WebChatActionSheetFooterAction) -> Unit,
    ) = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(activity, 50),
        ).apply { topMargin = dp(activity, 6) }
        gravity = Gravity.CENTER_VERTICAL
        orientation = LinearLayout.HORIZONTAL
        actions.forEach { action ->
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
                gravity = Gravity.CENTER
                includeFontPadding = false
                text = action.label
                textSize = 14f
                setTextColor(Color.parseColor(PRIMARY_TEXT_COLOR))
                contentDescription = action.contentDescription
                isClickable = true
                isFocusable = true
                setOnClickListener { onClick(action) }
            })
        }
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

    private const val ITEM_HEIGHT_DP = 64
    private const val MAX_LIST_HEIGHT_DP = 448
    private const val PANEL_COLOR = "#17181B"
    private const val SELECTED_ROW_COLOR = "#2A2B30"
    private const val HANDLE_COLOR = "#5E6067"
    private const val PRIMARY_TEXT_COLOR = "#F8F7F4"
    private const val SECONDARY_TEXT_COLOR = "#8F9299"
    private const val ACCENT_COLOR = "#8EA7D5"
}
