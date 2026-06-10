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
            dp(42)
        ).apply {
            marginStart = dp(18)
            marginEnd = dp(18)
            topMargin = dp(4)
        }
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(dp(4), dp(4), dp(4), dp(4))
        background = roundedBg("#181B20", "#283140")
        visibility = View.GONE
    }

    private val labelView = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.MATCH_PARENT, 1f)
        gravity = Gravity.CENTER_VERTICAL
        includeFontPadding = false
        text = "运行中输入"
        setTextColor(Color.parseColor("#A6AFBD"))
        textSize = 12.5f
        setPadding(dp(10), 0, dp(6), 0)
    }

    private val buttons = RunningInputMode.values().associateWith { mode ->
        TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.MATCH_PARENT
            ).apply {
                marginStart = dp(4)
            }
            minWidth = dp(68)
            gravity = Gravity.CENTER
            includeFontPadding = false
            text = mode.label
            textSize = 12f
            setTextColor(Color.parseColor("#F2F5FA"))
            setPadding(dp(10), 0, dp(10), 0)
            setOnClickListener { onModeSelected(mode) }
        }
    }

    init {
        view.addView(labelView)
        buttons.values.forEach(view::addView)
        setSelected(RunningInputMode.REMIND_CURRENT)
    }

    fun refresh(visible: Boolean, mode: RunningInputMode) {
        view.visibility = if (visible) View.VISIBLE else View.GONE
        setSelected(mode)
    }

    private fun setSelected(selected: RunningInputMode) {
        labelView.text = selected.activeHint
        buttons.forEach { (mode, button) ->
            val active = mode == selected
            button.setTypeface(Typeface.DEFAULT, if (active) Typeface.BOLD else Typeface.NORMAL)
            button.setTextColor(Color.parseColor(if (active) "#101010" else "#F2F5FA"))
            button.background = if (active) {
                roundedBg("#F2F5FA", "#F2F5FA")
            } else {
                roundedBg("#283140", "#24282F")
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
