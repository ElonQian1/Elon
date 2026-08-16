package com.elon.app.chatgptweb

import android.content.Context
import android.graphics.Typeface
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.SeekBar
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import com.elon.app.WebChatConsumerControl
import java.math.BigDecimal
import kotlin.math.roundToInt

internal object ChatGptNativeSliderControlDialog {
    fun sliderSelector(controlId: String): String =
        "chatgpt-control-slider:${ChatGptNativeControlPresentation.stableContextId(controlId)}"

    fun valueSelector(controlId: String): String =
        "chatgpt-control-slider-value:${ChatGptNativeControlPresentation.stableContextId(controlId)}"

    fun commitSelector(controlId: String): String =
        "chatgpt-control-slider-commit:${ChatGptNativeControlPresentation.stableContextId(controlId)}"

    fun show(
        context: Context,
        control: WebChatConsumerControl,
        onSubmit: (String, Double) -> Unit,
    ): AlertDialog {
        require(control.supportsSliderValue) { "Control does not expose a writable native slider." }
        val slider = requireNotNull(control.slider)
        val valueLabel = TextView(context).apply {
            textSize = 22f
            gravity = Gravity.CENTER
            setTypeface(typeface, Typeface.BOLD)
            contentDescription = valueSelector(control.id)
        }
        val seekBar = SeekBar(context).apply {
            max = slider.stepCount
            progress = ((slider.value - slider.min) / slider.step).roundToInt().coerceIn(0, max)
            contentDescription = sliderSelector(control.id)
        }
        fun selectedValue(): Double = slider.min + seekBar.progress * slider.step
        fun renderValue() {
            valueLabel.text = format(selectedValue())
        }
        seekBar.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(seekBar: SeekBar?, progress: Int, fromUser: Boolean) = renderValue()
            override fun onStartTrackingTouch(seekBar: SeekBar?) = Unit
            override fun onStopTrackingTouch(seekBar: SeekBar?) = Unit
        })
        renderValue()
        val container = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(context, 24), dp(context, 8), dp(context, 24), dp(context, 4))
            addView(valueLabel, LinearLayout.LayoutParams.MATCH_PARENT, dp(context, 48))
            addView(seekBar, LinearLayout.LayoutParams.MATCH_PARENT, dp(context, 56))
        }
        return AlertDialog.Builder(context)
            .setTitle(control.label)
            .setView(container)
            .setNegativeButton(android.R.string.cancel, null)
            .setPositiveButton(android.R.string.ok, null)
            .create()
            .also { dialog ->
                dialog.setOnShowListener {
                    dialog.getButton(AlertDialog.BUTTON_POSITIVE).apply {
                        contentDescription = commitSelector(control.id)
                        setOnClickListener {
                            onSubmit(control.id, selectedValue())
                            dialog.dismiss()
                        }
                    }
                }
                dialog.show()
            }
    }

    private fun format(value: Double): String =
        BigDecimal.valueOf(value).stripTrailingZeros().toPlainString()

    private fun dp(context: Context, value: Int): Int =
        (value * context.resources.displayMetrics.density).roundToInt()
}
