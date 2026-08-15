package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal class SocialAiModeSegmentedControl(
    private val activity: AppCompatActivity,
    private val host: FrameLayout,
    private val onSelected: (SocialAiInteractionMode) -> Unit,
) {
    private val chatButton = segmentButton(
        title = activity.getString(R.string.social_ai_mode_chat_tab),
        mode = SocialAiInteractionMode.CHAT,
    )
    private val workButton = segmentButton(
        title = activity.getString(R.string.social_ai_mode_work_tab),
        mode = SocialAiInteractionMode.WORK,
    )

    init {
        host.addView(
            LinearLayout(activity).apply {
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER
                setPadding(dp(4), dp(4), dp(4), dp(4))
                background = roundedBackground(OUTER_COLOR, 20)
                addView(chatButton, segmentLayoutParams())
                addView(workButton, segmentLayoutParams())
            },
            FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT,
            ),
        )
    }

    fun show(mode: SocialAiInteractionMode) {
        host.visibility = View.VISIBLE
        render(mode)
    }

    fun hide() {
        host.visibility = View.GONE
    }

    fun render(mode: SocialAiInteractionMode) {
        renderButton(chatButton, SocialAiInteractionMode.CHAT, mode)
        renderButton(workButton, SocialAiInteractionMode.WORK, mode)
    }

    private fun renderButton(
        button: TextView,
        buttonMode: SocialAiInteractionMode,
        selectedMode: SocialAiInteractionMode,
    ) {
        val selected = buttonMode == selectedMode
        button.isSelected = selected
        button.isActivated = selected
        button.background = if (selected) roundedBackground(SELECTED_COLOR, 16) else null
        button.setTextColor(Color.parseColor(if (selected) SELECTED_TEXT_COLOR else TEXT_COLOR))
        button.setTypeface(button.typeface, Typeface.NORMAL)
        button.contentDescription = activity.getString(
            R.string.social_ai_mode_tab_description,
            button.text,
            if (selected) activity.getString(R.string.social_ai_mode_selected) else "",
        )
    }

    private fun segmentButton(title: String, mode: SocialAiInteractionMode) = TextView(activity).apply {
        gravity = Gravity.CENTER
        includeFontPadding = false
        text = title
        textSize = 15f
        isClickable = true
        isFocusable = true
        tag = "social_ai_mode_${mode.wireValue}"
        setOnClickListener { onSelected(mode) }
    }

    private fun segmentLayoutParams() = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)

    private fun roundedBackground(color: String, radiusDp: Int) = GradientDrawable().apply {
        shape = GradientDrawable.RECTANGLE
        cornerRadius = dp(radiusDp).toFloat()
        setColor(Color.parseColor(color))
    }

    private fun dp(value: Int): Int = (value * activity.resources.displayMetrics.density).toInt()

    private companion object {
        const val OUTER_COLOR = "#3A3A3A"
        const val SELECTED_COLOR = "#202126"
        const val SELECTED_TEXT_COLOR = "#F8F7F4"
        const val TEXT_COLOR = "#B3DDDBD5"
    }
}
