package com.elon.app

import android.text.InputType
import android.widget.ArrayAdapter
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.Spinner
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal object ProjectPlazaInstallDialog {
    private data class Industry(val key: String, val label: String)

    private val industries = listOf(
        Industry("local_retail", "本地零售"),
        Industry("coffee", "咖啡与饮品"),
        Industry("restaurant", "餐饮外卖"),
        Industry("convenience", "便利店")
    )

    fun show(
        activity: AppCompatActivity,
        sourceProject: StoreProject,
        onCreate: (projectName: String, industry: String) -> Unit
    ) {
        val nameInput = EditText(activity).apply {
            setText("我的店铺")
            selectAll()
            hint = "店铺名称"
            maxLines = 1
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
            contentDescription = "店铺名称"
        }
        val industrySpinner = Spinner(activity).apply {
            adapter = ArrayAdapter(
                activity,
                android.R.layout.simple_spinner_dropdown_item,
                industries.map(Industry::label)
            )
            contentDescription = "经营类型"
        }
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            val padding = dp(activity, 20)
            setPadding(padding, dp(activity, 8), padding, 0)
            addView(label(activity, "店铺名称"))
            addView(nameInput)
            addView(label(activity, "经营类型"), margins(activity, top = 14))
            addView(industrySpinner)
            addView(TextView(activity).apply {
                text = "系统会创建一个只属于你的独立项目。平台登录状态和经营数据不会写入公开模板项目。"
                textSize = 13f
            }, margins(activity, top = 16))
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle(sourceProject.installAction?.label ?: "创建我的店铺")
            .setView(content)
            .setNegativeButton("取消", null)
            .setPositiveButton("创建并进入", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val projectName = nameInput.text.toString().trim()
                if (projectName.isBlank()) {
                    nameInput.error = "请输入店铺名称"
                    return@setOnClickListener
                }
                val industry = industries.getOrElse(industrySpinner.selectedItemPosition) { industries.first() }
                dialog.dismiss()
                onCreate(projectName, industry.key)
            }
            nameInput.requestFocus()
        }
        dialog.show()
    }

    private fun label(activity: AppCompatActivity, text: String) = TextView(activity).apply {
        this.text = text
        textSize = 13f
    }

    private fun margins(activity: AppCompatActivity, top: Int) =
        LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply { topMargin = dp(activity, top) }

    private fun dp(activity: AppCompatActivity, value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()
}
