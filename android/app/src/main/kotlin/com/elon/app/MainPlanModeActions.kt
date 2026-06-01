package com.elon.app

import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.widget.EditText
import android.widget.TextView

internal class MainPlanModeActions(
    private val inputEdit: () -> EditText,
    private val dp: (Int) -> Int
) {
    private var enabled = false
    private var button: TextView? = null

    fun bind(planButton: TextView) {
        button = planButton
        planButton.setOnClickListener { togglePlanMode() }
        refresh()
    }

    fun enableWithStarterPrompt() {
        setEnabled(true)
        val input = inputEdit()
        if (input.text.isBlank()) {
            input.setText("请先给我一个开发计划，不要直接改代码：")
            input.setSelection(input.text.length)
        }
    }

    fun consumeForSend(): ProjectRequestExecutionMode {
        if (!enabled) return ProjectRequestExecutionMode.Execute
        setEnabled(false)
        return ProjectRequestExecutionMode.Plan
    }

    fun prepareImplementationPrompt() {
        val input = inputEdit()
        if (input.text.isNotBlank()) return
        input.setText("按这个计划开始实现。")
        input.setSelection(input.text.length)
    }

    fun togglePlanMode() {
        setEnabled(!enabled)
    }

    private fun setEnabled(value: Boolean) {
        enabled = value
        refresh()
    }

    private fun refresh() {
        val view = button ?: return
        view.text = if (enabled) "规划中" else "先规划"
        view.contentDescription = if (enabled) {
            "已开启先规划，下次发送只生成计划"
        } else {
            "开启先规划"
        }
        view.setTextColor(Color.parseColor(if (enabled) "#101010" else "#D7D7D7"))
        view.background = GradientDrawable().apply {
            cornerRadius = dp(16).toFloat()
            setColor(Color.parseColor(if (enabled) "#BDEFD3" else "#2A2A2A"))
            setStroke(dp(1), Color.parseColor(if (enabled) "#D9FFE8" else "#3A3A3A"))
        }
    }
}
