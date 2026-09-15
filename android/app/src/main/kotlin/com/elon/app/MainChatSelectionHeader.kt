package com.elon.app

import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import com.elon.app.databinding.ActivityMainBinding

internal class MainChatSelectionHeader(private val binding: ActivityMainBinding, cancel: () -> Unit) {
    private val context = binding.root.context
    private val count = TextView(context).apply {
        textSize = 16f
        gravity = Gravity.CENTER
        setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
        contentDescription = "ai-conversation-share-selection-count"
        maxLines = 2
    }
    private val surface = LinearLayout(context).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        isClickable = true
        setBackgroundColor(ContextCompat.getColor(context, R.color.elon_bg_app))
        addView(TextView(context).apply {
            text = "取消"
            textSize = 16f
            gravity = Gravity.CENTER
            setTextColor(ContextCompat.getColor(context, R.color.elon_text_primary))
            contentDescription = "ai-conversation-share-selection-cancel"
            setOnClickListener { cancel() }
        }, LinearLayout.LayoutParams(dp(64), dp(56)))
        addView(count, LinearLayout.LayoutParams(0, -1, 1f))
        addView(View(context), LinearLayout.LayoutParams(dp(64), 1))
    }
    private val hiddenAccessibility = linkedMapOf<View, Int>()

    fun show() {
        val parent = binding.backButton.parent as? FrameLayout ?: return
        if (surface.parent === parent) return
        for (i in 0 until parent.childCount) {
            val child = parent.getChildAt(i)
            hiddenAccessibility[child] = child.importantForAccessibility
            child.importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO_HIDE_DESCENDANTS
        }
        parent.addView(surface, FrameLayout.LayoutParams(-1, -1))
    }

    fun update(value: Int) { count.text = "已选择 $value 条消息" }

    fun hide() {
        (surface.parent as? FrameLayout)?.removeView(surface)
        hiddenAccessibility.forEach { (view, importance) -> view.importantForAccessibility = importance }
        hiddenAccessibility.clear()
    }

    private fun dp(value: Int) = (value * context.resources.displayMetrics.density).toInt()
}
