package com.elon.app

import android.graphics.drawable.GradientDrawable
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.RadioGroup
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal class UiDesignRequestOptionsDialog(
    private val activity: AppCompatActivity,
    private val dp: (Int) -> Int
) {
    fun show(
        current: UiDesignRequestSelection,
        onApply: (UiDesignRequestSelection) -> Unit
    ) {
        val modeGroup = optionGroup(
            modeOptions,
            current.mode.wireValue,
            "任务方式"
        )
        val intentGroup = optionGroup(
            intentOptions,
            current.imageIntent.wireValue,
            "图片用途"
        )
        val screenInput = input(
            hint = "页面 ID（可选，例如 checkout）",
            value = current.screenId.orEmpty(),
            singleLine = true
        )
        val behaviorInput = input(
            hint = "业务说明（可选，例如主按钮进入支付页）",
            value = current.behaviorNotes.joinToString("\n"),
            singleLine = false
        )
        val content = ScrollView(activity).apply {
            isFillViewport = true
            addView(LinearLayout(activity).apply {
                orientation = LinearLayout.VERTICAL
                setPadding(dp(20), dp(4), dp(20), dp(8))
                addView(description())
                addView(modeGroup)
                addView(intentGroup)
                addView(sectionLabel("页面与业务"))
                addView(screenInput)
                addView(behaviorInput)
            })
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle("UI 设计任务")
            .setView(content)
            .setNegativeButton("关闭", null)
            .setPositiveButton("应用到本次发送", null)
            .create()
        dialog.setOnShowListener {
            dialog.window?.setBackgroundDrawable(dialogBackground())
            dialog.getButton(AlertDialog.BUTTON_NEGATIVE)?.setTextColor(
                activity.elonColor(R.color.elon_text_secondary)
            )
            dialog.getButton(AlertDialog.BUTTON_POSITIVE)?.apply {
                setTextColor(activity.elonColor(R.color.elon_button_primary_text))
                background = actionBackground()
                setOnClickListener {
                    val mode = selectedMode(modeGroup)
                    val intent = selectedIntent(intentGroup)
                    val notes = behaviorInput.text.toString()
                        .lineSequence()
                        .map(String::trim)
                        .filter(String::isNotEmpty)
                        .take(32)
                        .toList()
                    onApply(
                        UiDesignRequestSelection(
                            enabled = true,
                            mode = mode,
                            imageIntent = intent,
                            screenId = screenInput.text.toString().trim().takeIf(String::isNotEmpty),
                            behaviorNotes = notes
                        )
                    )
                    dialog.dismiss()
                }
            }
        }
        dialog.show()
    }

    private fun description(): TextView = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ).apply { bottomMargin = dp(12) }
        text = "选择图片的真实用途。全新创建会先生成可运行页面，再使用真实 Android Renderer 拟合。"
        setTextColor(activity.elonColor(R.color.elon_text_secondary))
        textSize = 14f
        setLineSpacing(0f, 1.2f)
    }

    private fun optionGroup(
        options: List<OptionItem>,
        selected: String,
        title: String
    ): RadioGroup = RadioGroup(activity).apply {
        orientation = RadioGroup.VERTICAL
        addView(sectionLabel(title))
        options.forEach { option ->
            addView(RadioButton(activity).apply {
                id = View.generateViewId()
                tag = option.value
                text = "${option.label}  ·  ${option.description}"
                setTextColor(activity.elonColor(R.color.elon_text_primary))
                textSize = 15f
                minimumHeight = dp(48)
                gravity = Gravity.CENTER_VERTICAL
                isChecked = option.value == selected
                buttonTintList = android.content.res.ColorStateList.valueOf(
                    activity.elonColor(R.color.elon_button_primary_bg)
                )
            })
        }
    }

    private fun sectionLabel(value: String): TextView = TextView(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT
        ).apply {
            topMargin = dp(8)
            bottomMargin = dp(4)
        }
        text = value
        setTextColor(activity.elonColor(R.color.elon_text_primary))
        textSize = 17f
    }

    private fun input(hint: String, value: String, singleLine: Boolean): EditText = EditText(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            if (singleLine) dp(52) else dp(96)
        ).apply { bottomMargin = dp(10) }
        background = inputBackground()
        setPadding(dp(14), dp(10), dp(14), dp(10))
        setText(value)
        this.hint = hint
        setTextColor(activity.elonColor(R.color.elon_text_primary))
        setHintTextColor(activity.elonColor(R.color.elon_text_placeholder))
        textSize = 15f
        setSingleLine(singleLine)
        inputType = if (singleLine) {
            InputType.TYPE_CLASS_TEXT
        } else {
            InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        }
        gravity = if (singleLine) Gravity.CENTER_VERTICAL else Gravity.TOP
    }

    private fun selectedMode(group: RadioGroup): UiDesignRequestMode {
        val value = group.findViewById<RadioButton>(group.checkedRadioButtonId)?.tag as? String
        return UiDesignRequestMode.values().firstOrNull { it.wireValue == value }
            ?: UiDesignRequestMode.AUTO
    }

    private fun selectedIntent(group: RadioGroup): UiDesignImageIntent {
        val value = group.findViewById<RadioButton>(group.checkedRadioButtonId)?.tag as? String
        return UiDesignImageIntent.values().firstOrNull { it.wireValue == value }
            ?: UiDesignImageIntent.AUTO
    }

    private fun dialogBackground() = GradientDrawable().apply {
        setColor(activity.elonColor(R.color.elon_surface_card))
        cornerRadius = dp(20).toFloat()
    }

    private fun inputBackground() = GradientDrawable().apply {
        setColor(activity.elonColor(R.color.elon_surface_search))
        cornerRadius = dp(12).toFloat()
        setStroke(dp(1), activity.elonColor(R.color.elon_border_subtle))
    }

    private fun actionBackground() = GradientDrawable().apply {
        setColor(activity.elonColor(R.color.elon_button_primary_bg))
        cornerRadius = dp(18).toFloat()
    }

    private data class OptionItem(
        val value: String,
        val label: String,
        val description: String
    )

    private val modeOptions = listOf(
        OptionItem("AUTO", "自动判断", "根据项目和运行证据选择"),
        OptionItem("MODIFY_EXISTING", "修改现有", "已有页面和源码"),
        OptionItem("EXTEND_EXISTING", "扩展现有", "现有页面增加结构"),
        OptionItem("CREATE_NEW", "全新创建", "项目中还没有相关源码")
    )

    private val intentOptions = listOf(
        OptionItem("AUTO", "自动判断", "根据图片和标注选择"),
        OptionItem("TARGET_DESIGN", "目标设计图", "可用于像素和几何拟合"),
        OptionItem("ANNOTATED_CHANGE_REQUEST", "标注修改图", "框、箭头和文字是要求"),
        OptionItem("REFERENCE_STYLE", "风格参考图", "只参考视觉语言"),
        OptionItem("CURRENT_SCREENSHOT", "当前截图", "表示项目现状而非目标")
    )
}
