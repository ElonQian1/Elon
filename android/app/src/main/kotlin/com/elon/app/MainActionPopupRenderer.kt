package com.elon.app

import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
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

internal const val WECHAT_POPUP_PANEL_COLOR = "#BDBDBD"
internal const val WECHAT_POPUP_TEXT_COLOR = "#242424"
internal const val WECHAT_POPUP_DIVIDER_COLOR = "#A8A8A8"
internal const val LEGACY_MESSAGE_POPUP_COLOR = "#3D3D3D"

internal class MainActionPopupRenderer(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val createDivider: (Int) -> View,
    private val createArrow: (Boolean, Int) -> View
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

        root.addView(createArrow(true, Color.parseColor(WECHAT_POPUP_PANEL_COLOR)), FrameLayout.LayoutParams(dp(16), arrowHeight).apply {
            gravity = Gravity.TOP or Gravity.END
            rightMargin = dp(20)
        })

        lateinit var popup: PopupWindow
        actions.forEachIndexed { index, action ->
            panel.addView(createTopActionRow(action) { popup.dismiss() })
            if (index < actions.lastIndex) {
                panel.addView(createDivider(dp(52)))
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

        val popupWidth = minOf(activity.resources.displayMetrics.widthPixels - dp(24), dp(282))
        val arrowHeight = dp(8)
        val panelHeight = dp(132)
        val totalHeight = panelHeight + arrowHeight
        val root = FrameLayout(activity).apply {
            layoutParams = ViewGroup.LayoutParams(popupWidth, totalHeight)
            alpha = 0f
            scaleX = 0.96f
            scaleY = 0.96f
        }
        val panel = GridLayout(activity).apply {
            columnCount = 5
            rowCount = 2
            background = GradientDrawable().apply {
                cornerRadius = dp(4).toFloat()
                setColor(Color.parseColor(LEGACY_MESSAGE_POPUP_COLOR))
            }
            setPadding(dp(10), dp(8), dp(10), dp(8))
        }
        root.addView(panel, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            panelHeight
        ))
        lateinit var popup: PopupWindow
        actions.forEach { action ->
            panel.addView(createMessageActionCell(action) { popup.dismiss() }, GridLayout.LayoutParams().apply {
                width = (popupWidth - dp(20)) / 5
                height = dp(58)
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
            createArrow(!showAbove, Color.parseColor(LEGACY_MESSAGE_POPUP_COLOR)),
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

    private fun createMessageActionCell(action: TopAction, dismissPopup: () -> Unit): View {
        return LinearLayout(activity).apply {
            gravity = Gravity.CENTER
            orientation = LinearLayout.VERTICAL
            isClickable = true
            foreground = selectableForeground()

            addView(ImageView(context).apply {
                layoutParams = LinearLayout.LayoutParams(dp(24), dp(24))
                setImageResource(action.iconRes)
            })
            addView(TextView(context).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply {
                    topMargin = dp(4)
                }
                includeFontPadding = false
                text = action.title
                setTextColor(Color.parseColor("#EAEAEA"))
                textSize = 13f
            })
            setOnClickListener {
                dismissPopup()
                action.action()
            }
        }
    }
}
