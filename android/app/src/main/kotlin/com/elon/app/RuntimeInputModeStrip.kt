package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal class RuntimeInputModeStrip(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int,
    private val onModeSelected: (RunningInputMode) -> Unit
) {
    val view: LinearLayout = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(46)
        ).apply {
            marginStart = dp(18)
            marginEnd = dp(18)
            bottomMargin = dp(8)
        }
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(0, 0, 0, 0)
        background = null
        visibility = View.GONE
    }

    private val buttons = RunningInputMode.values().associateWith { mode ->
        TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                when (mode) {
                    RunningInputMode.REMIND_CURRENT -> dp(88)
                    else -> dp(76)
                },
                dp(42)
            ).apply {
                marginEnd = dp(10)
            }
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = mode.label
            textSize = 15f
            setTextColor(Color.parseColor("#D6D6D6"))
            setOnClickListener { onModeSelected(mode) }
        }
    }

    init {
        buttons.values.forEach(view::addView)
        setSelected(RunningInputMode.REMIND_CURRENT)
    }

    fun refresh(visible: Boolean, mode: RunningInputMode) {
        view.visibility = if (visible) View.VISIBLE else View.GONE
        setSelected(mode)
    }

    private fun setSelected(selected: RunningInputMode) {
        buttons.forEach { (mode, button) ->
            val active = mode == selected
            button.setTypeface(Typeface.DEFAULT, Typeface.NORMAL)
            button.setTextColor(Color.parseColor(if (active) "#101010" else "#D6D6D6"))
            button.background = if (active) {
                roundedBg("#D6D6D6", "#D6D6D6")
            } else {
                activity.getDrawable(R.drawable.bg_bottom_mode_pill_new)
            }
        }
    }

    private fun roundedBg(fill: String, stroke: String): GradientDrawable {
        return GradientDrawable().apply {
            shape = GradientDrawable.RECTANGLE
            cornerRadius = dp(16).toFloat()
            setColor(Color.parseColor(fill))
            setStroke(dp(1), Color.parseColor(stroke))
        }
    }
}
