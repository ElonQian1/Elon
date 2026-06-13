package com.elon.app

import android.graphics.Color
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.GridLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.PopupWindow
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal const val WECHAT_POPUP_PANEL_COLOR = "#242424"
internal const val WECHAT_POPUP_TEXT_COLOR = "#D6D6D6"
internal const val WECHAT_POPUP_DIVIDER_COLOR = "#2E2E2E"
internal const val LEGACY_MESSAGE_POPUP_COLOR = "#242424"

private const val MESSAGE_POPUP_DEFAULT_COLUMNS = 5
private const val MESSAGE_POPUP_CELL_WIDTH_DP = 54
private const val MESSAGE_POPUP_COMPACT_CELL_WIDTH_DP = 62
private const val MESSAGE_POPUP_CELL_HEIGHT_DP = 52
private const val MESSAGE_POPUP_HORIZONTAL_PADDING_DP = 9
private const val MESSAGE_POPUP_VERTICAL_PADDING_DP = 8
private const val MESSAGE_POPUP_ICON_SIZE_DP = 20
private const val MESSAGE_POPUP_LABEL_TOP_MARGIN_DP = 5
private const val MESSAGE_POPUP_LABEL_TEXT_SIZE_DP = 11.5f

internal class MainActionPopupRenderer(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    fun showTopActionPopup(anchor: View, previousPopup: PopupWindow?, actions: List<TopAction>): PopupWindow {
        previousPopup?.dismiss()

        val popupWidth = dp(168)
        val arrowHeight = dp(8)
        val root = FrameLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(popupWidth, ViewGroup.LayoutParams.WRAP_CONTENT)
            alpha = 0f
            scaleX = 0.98f
            scaleY = 0.98f
        }

        val panel = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            background = GradientDrawable().apply {
                cornerRadius = dp(10).toFloat()
                setColor(Color.parseColor(WECHAT_POPUP_PANEL_COLOR))
            }
        }
        root.addView(panel, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = arrowHeight
        })

        root.addView(createPopupArrowView(), FrameLayout.LayoutParams(dp(16), arrowHeight).apply {
            gravity = Gravity.TOP or Gravity.END
            rightMargin = dp(20)
        })

        lateinit var popup: PopupWindow
        actions.forEachIndexed { index, action ->
            panel.addView(createTopActionRow(action) { popup.dismiss() })
            if (index < actions.lastIndex) {
                panel.addView(createPopupDivider(dp(52)))
            }
        }

        popup = PopupWindow(
            root,
            popupWidth,
            ViewGroup.LayoutParams.WRAP_CONTENT,
            true
        ).apply {
            isOutsideTouchable = true
            elevation = dp(8).toFloat()
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            showAsDropDown(anchor, anchor.width - popupWidth + dp(2), -dp(2))
        }
        root.pivotX = (popupWidth - dp(28)).toFloat()
        root.pivotY = 0f
        root.animate()
            .alpha(1f)
            .scaleX(1f)
            .scaleY(1f)
            .setDuration(120L)
            .start()
        return popup
    }

    fun showMessageActionPopup(anchor: View, previousPopup: PopupWindow?, actions: List<TopAction>): PopupWindow {
        previousPopup?.dismiss()

        val compact = actions.size <= 3
        val columnCount = if (compact) actions.size.coerceAtLeast(1) else MESSAGE_POPUP_DEFAULT_COLUMNS
        val rowCount = ((actions.size + columnCount - 1) / columnCount).coerceAtLeast(1)
        val horizontalPadding = dp(MESSAGE_POPUP_HORIZONTAL_PADDING_DP)
        val verticalPadding = dp(MESSAGE_POPUP_VERTICAL_PADDING_DP)
        val targetCellWidth = dp(
            if (compact) MESSAGE_POPUP_COMPACT_CELL_WIDTH_DP else MESSAGE_POPUP_CELL_WIDTH_DP
        )
        val screenMaxWidth = activity.resources.displayMetrics.widthPixels - dp(32)
        val popupWidth = if (compact) {
            minOf(screenMaxWidth, targetCellWidth * columnCount + horizontalPadding * 2)
        } else {
            minOf(screenMaxWidth, targetCellWidth * columnCount + horizontalPadding * 2)
        }
        val arrowHeight = dp(8)
        val cellHeight = dp(MESSAGE_POPUP_CELL_HEIGHT_DP)
        val panelHeight = verticalPadding * 2 + cellHeight * rowCount
        val totalHeight = panelHeight + arrowHeight
        val root = FrameLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(popupWidth, totalHeight)
            alpha = 0f
            scaleX = 0.96f
            scaleY = 0.96f
        }
        val panel = GridLayout(activity).apply {
            this.columnCount = columnCount
            this.rowCount = rowCount
            background = GradientDrawable().apply {
                cornerRadius = dp(4).toFloat()
                setColor(Color.parseColor(LEGACY_MESSAGE_POPUP_COLOR))
            }
            setPadding(horizontalPadding, verticalPadding, horizontalPadding, verticalPadding)
        }
        root.addView(panel, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            panelHeight
        ))
        lateinit var popup: PopupWindow
        val cellWidth = (popupWidth - horizontalPadding * 2) / columnCount
        actions.forEach { action ->
            panel.addView(createMessageActionCell(action) { popup.dismiss() }, GridLayout.LayoutParams().apply {
                width = cellWidth
                height = cellHeight
            })
        }

        val anchorLocation = IntArray(2)
        anchor.getLocationOnScreen(anchorLocation)
        val anchorCenterX = anchorLocation[0] + anchor.width / 2
        val aboveY = anchorLocation[1] - totalHeight - dp(8)
        val showAbove = aboveY > dp(76)
        val popupX = (anchorCenterX - popupWidth / 2)
            .coerceIn(dp(12), activity.resources.displayMetrics.widthPixels - popupWidth - dp(12))
        val popupY = if (showAbove) aboveY else anchorLocation[1] + anchor.height + dp(8)
        val arrowX = (anchorCenterX - popupX - dp(9)).coerceIn(dp(18), popupWidth - dp(36))

        root.addView(
            createPopupArrowView(pointsUp = !showAbove, color = Color.parseColor(LEGACY_MESSAGE_POPUP_COLOR)),
            FrameLayout.LayoutParams(dp(18), arrowHeight).apply {
                gravity = if (showAbove) Gravity.BOTTOM or Gravity.START else Gravity.TOP or Gravity.START
                leftMargin = arrowX
            }
        )
        if (!showAbove) {
            (panel.layoutParams as FrameLayout.LayoutParams).topMargin = arrowHeight
        }

        popup = PopupWindow(root, popupWidth, totalHeight, true).apply {
            isOutsideTouchable = true
            elevation = dp(8).toFloat()
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            showAtLocation(anchor, Gravity.NO_GRAVITY, popupX, popupY)
        }
        root.animate()
            .alpha(1f)
            .scaleX(1f)
            .scaleY(1f)
            .setDuration(120L)
            .start()
        return popup
    }

    fun showProjectCardActionPopup(anchor: View, previousPopup: PopupWindow?, actions: List<TopAction>): PopupWindow {
        previousPopup?.dismiss()

        val visibleActions = actions.take(3).ifEmpty { actions }
        val actionWidth = dp(58)
        val popupWidth = (actionWidth * visibleActions.size + dp(20))
            .coerceAtMost(activity.resources.displayMetrics.widthPixels - dp(24))
        val popupHeight = dp(46)
        val root = FrameLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(popupWidth, popupHeight)
            alpha = 0f
            scaleX = 0.98f
            scaleY = 0.82f
            translationY = dp(8).toFloat()
        }
        val panel = LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER
            background = createProjectCardPopupBackground()
            setPadding(dp(10), 0, dp(10), 0)
        }
        root.addView(
            panel,
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
        )

        lateinit var popup: PopupWindow
        visibleActions.forEachIndexed { index, action ->
            panel.addView(
                createProjectCardActionCell(action) { popup.dismiss() },
                LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
            )
            if (index < visibleActions.lastIndex) {
                panel.addView(createProjectCardDivider())
            }
        }

        val anchorLocation = IntArray(2)
        anchor.getLocationOnScreen(anchorLocation)
        val popupX = (anchorLocation[0] + dp(16))
            .coerceIn(dp(12), activity.resources.displayMetrics.widthPixels - popupWidth - dp(12))
        val popupY = (anchorLocation[1] - popupHeight).coerceAtLeast(dp(76))

        popup = PopupWindow(root, popupWidth, popupHeight, true).apply {
            isOutsideTouchable = true
            elevation = dp(8).toFloat()
            setBackgroundDrawable(ColorDrawable(Color.TRANSPARENT))
            showAtLocation(anchor, Gravity.NO_GRAVITY, popupX, popupY)
        }
        root.pivotX = dp(28).toFloat()
        root.pivotY = popupHeight.toFloat()
        root.animate()
            .alpha(1f)
            .scaleX(1f)
            .scaleY(1f)
            .translationY(0f)
            .setDuration(120L)
            .start()
        return popup
    }

    private fun createTopActionRow(action: TopAction, dismissPopup: () -> Unit): View {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(46)
            )
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            setPadding(dp(16), 0, dp(12), 0)
            isClickable = true
            foreground = selectableForeground()

            addView(ImageView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(22), dp(22))
                setImageResource(action.iconRes)
                setColorFilter(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    marginStart = dp(13)
                }
                includeFontPadding = false
                text = action.title
                setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
                textSize = 15.5f
            })
            setOnClickListener {
                dismissPopup()
                action.action()
            }
        }
    }

    private fun createPopupDivider(marginStart: Int = 0): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                1
            ).apply {
                this.marginStart = marginStart
            }
            alpha = 0.55f
            setBackgroundColor(Color.parseColor(WECHAT_POPUP_DIVIDER_COLOR))
        }
    }

    private fun createPopupArrowView(
        pointsUp: Boolean = true,
        color: Int = Color.parseColor(WECHAT_POPUP_PANEL_COLOR)
    ): View {
        val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            this.color = color
            style = Paint.Style.FILL
        }
        return object : View(activity) {
            override fun onDraw(canvas: Canvas) {
                super.onDraw(canvas)
                val path = Path().apply {
                    if (pointsUp) {
                        moveTo(width / 2f, 0f)
                        lineTo(width.toFloat(), height.toFloat())
                        lineTo(0f, height.toFloat())
                    } else {
                        moveTo(0f, 0f)
                        lineTo(width.toFloat(), 0f)
                        lineTo(width / 2f, height.toFloat())
                    }
                    close()
                }
                canvas.drawPath(path, paint)
            }
        }
    }

    private fun createMessageActionCell(action: TopAction, dismissPopup: () -> Unit): View {
        return LinearLayout(activity).apply {
            gravity = Gravity.CENTER
            orientation = LinearLayout.VERTICAL
            isClickable = true
            foreground = selectableForeground()
            setPadding(0, dp(2), 0, 0)

            addView(ImageView(context).apply {
                val iconSize = dp(MESSAGE_POPUP_ICON_SIZE_DP)
                layoutParams = LinearLayout.LayoutParams(iconSize, iconSize)
                setImageResource(action.iconRes)
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(MESSAGE_POPUP_LABEL_TOP_MARGIN_DP)
                }
                includeFontPadding = false
                text = action.title
                setTextColor(Color.parseColor("#D6D6D6"))
                setTextSize(TypedValue.COMPLEX_UNIT_DIP, MESSAGE_POPUP_LABEL_TEXT_SIZE_DP)
                gravity = Gravity.CENTER
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            setOnClickListener {
                dismissPopup()
                action.action()
            }
        }
    }

    private fun createProjectCardActionCell(action: TopAction, dismissPopup: () -> Unit): View {
        return TextView(activity).apply {
            gravity = Gravity.CENTER
            includeFontPadding = false
            isClickable = true
            foreground = selectableForeground()
            text = action.title
            setTextColor(Color.parseColor(WECHAT_POPUP_TEXT_COLOR))
            textSize = 15.5f
            setOnClickListener {
                dismissPopup()
                action.action()
            }
        }
    }

    private fun createProjectCardDivider(): View {
        return View(activity).apply {
            layoutParams = LinearLayout.LayoutParams(1, dp(28))
            alpha = 0.55f
            setBackgroundColor(Color.parseColor(WECHAT_POPUP_DIVIDER_COLOR))
        }
    }

    private fun createProjectCardPopupBackground(): GradientDrawable {
        val radius = dp(10).toFloat()
        return GradientDrawable().apply {
            setColor(Color.parseColor(WECHAT_POPUP_PANEL_COLOR))
            cornerRadii = floatArrayOf(
                radius, radius,
                radius, radius,
                0f, 0f,
                0f, 0f
            )
        }
    }
}
