package com.elon.app

import android.content.res.ColorStateList
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal class WebChatProductionActiveToolChip(
    activity: AppCompatActivity,
    private val dp: (Int) -> Int,
) : LinearLayout(activity) {
    private val icon = ImageView(activity).apply {
        layoutParams = LayoutParams(dp(24), dp(24)).apply { marginEnd = dp(7) }
        scaleType = ImageView.ScaleType.FIT_CENTER
        imageTintList = ColorStateList.valueOf(Color.parseColor(ACTIVE_FOREGROUND))
    }
    private val label = TextView(activity).apply {
        layoutParams = LayoutParams(LayoutParams.WRAP_CONTENT, LayoutParams.MATCH_PARENT)
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        maxLines = 1
        textSize = 15f
        setTextColor(Color.parseColor(ACTIVE_FOREGROUND))
    }
    private val close = TextView(activity).apply {
        layoutParams = LayoutParams(dp(32), LayoutParams.MATCH_PARENT)
        gravity = Gravity.CENTER
        includeFontPadding = false
        text = "×"
        textSize = 24f
        setTextColor(Color.parseColor(ACTIVE_FOREGROUND))
        contentDescription = "关闭当前网页工具"
        isClickable = true
        isFocusable = true
    }

    init {
        layoutParams = LayoutParams(LayoutParams.WRAP_CONTENT, dp(48)).apply {
            marginEnd = dp(4)
        }
        minimumWidth = dp(116)
        gravity = Gravity.CENTER_VERTICAL
        orientation = HORIZONTAL
        setPadding(dp(14), 0, dp(2), 0)
        background = GradientDrawable().apply {
            cornerRadius = dp(16).toFloat()
            setColor(Color.parseColor(ACTIVE_BACKGROUND))
        }
        addView(icon)
        addView(label)
        addView(close)
        visibility = View.GONE
    }

    fun render(
        action: WebChatProductionQuickComposerAction?,
        onClear: (WebChatProductionQuickComposerAction) -> Unit,
    ) {
        if (action == null) {
            visibility = View.GONE
            close.setOnClickListener(null)
            return
        }
        icon.setImageResource(
            when (action) {
                WebChatProductionQuickComposerAction.IMAGE_GENERATION -> R.drawable.ic_attach_function
                WebChatProductionQuickComposerAction.WEB_SEARCH -> R.drawable.ic_search_simple
            },
        )
        label.text = when (action) {
            WebChatProductionQuickComposerAction.IMAGE_GENERATION -> "创建图片"
            WebChatProductionQuickComposerAction.WEB_SEARCH -> "搜索"
        }
        contentDescription = "已启用${action.label}"
        close.contentDescription = "关闭${action.label}"
        close.setOnClickListener { onClear(action) }
        visibility = View.VISIBLE
    }

    private companion object {
        const val ACTIVE_BACKGROUND = "#075A9C"
        const val ACTIVE_FOREGROUND = "#C8E4FF"
    }
}
