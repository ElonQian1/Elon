package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.graphics.Typeface
import android.text.Editable
import android.text.InputFilter
import android.text.TextWatcher
import android.view.Gravity
import android.view.WindowManager
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.AppCompatImageButton
import androidx.appcompat.widget.TooltipCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

internal class WebChatTextBlockEditor(private val activity: AppCompatActivity, private val block: WebChatTextBlock) {
    private val density = activity.resources.displayMetrics.density
    private val title = block.title.ifBlank { if (block.kind == "code") "${block.language} 代码".trim() else "写作块" }
    private val status = TextView(activity).apply { textSize = 14f; contentDescription = "web-chat-text-block-status" }
    private var editing = false
    private var saving = false
    private var exported: String? = null
    private var child: AlertDialog? = null
    private val body = EditText(activity).apply {
        setText(block.content)
        setSingleLine(false)
        gravity = Gravity.TOP or Gravity.START
        textSize = 16f
        typeface = if (block.kind == "code") Typeface.MONOSPACE else Typeface.DEFAULT
        inputType = android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_FLAG_MULTI_LINE or
            android.text.InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
        setTextIsSelectable(true)
        isFocusableInTouchMode = false
        contentDescription = "web-chat-text-block-body"
        filters = arrayOf(InputFilter { _, from, to, dest, start, end ->
            if (dest.length - (end - start) + to - from > WebChatTextBlock.MAX_CONTENT) {
                Toast.makeText(activity, "内容超过编辑上限，本次输入未添加", Toast.LENGTH_SHORT).show()
                dest.subSequence(start, end)
            } else null
        })
        addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
            override fun afterTextChanged(s: Editable?) = render()
        })
    }
    private val editKeyListener = body.keyListener
    private val edit = icon(R.drawable.ic_project_action_rename, "编辑副本", "edit") {
        editing = !editing
        body.keyListener = if (editing) editKeyListener else null
        body.isCursorVisible = editing
        body.isFocusableInTouchMode = editing
        if (editing) body.requestFocus() else body.clearFocus()
        render()
    }
    private val copy = icon(R.drawable.ic_msg_copy, "复制正文", "copy") {
        activity.getSystemService(ClipboardManager::class.java).setPrimaryClip(ClipData.newPlainText(title, body.text.toString()))
        Toast.makeText(activity, "已复制", Toast.LENGTH_SHORT).show()
    }
    private val reset = icon(R.drawable.ic_popup_history, "恢复原文", "reset") {
        child = AlertDialog.Builder(activity).setTitle("恢复原文？").setMessage("本机尚未导出的修改将被替换。")
            .setPositiveButton("恢复") { _, _ -> body.setText(block.content) }.setNegativeButton("取消", null).show()
    }
    private val export = icon(android.R.drawable.stat_sys_download_done, "导出副本", "export", ::chooseExport)
    private val dialog = AlertDialog.Builder(activity).setTitle(title).setView(LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(16), dp(8), dp(16), dp(16))
        addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            addView(status, LinearLayout.LayoutParams(0, -2, 1f))
            for (button in listOf(edit, copy, reset, export)) addView(button, LinearLayout.LayoutParams(dp(48), dp(48)))
        })
        addView(body, LinearLayout.LayoutParams(-1, 0, 1f))
    }).setNegativeButton("返回", null).create()

    fun show() {
        body.keyListener = null
        body.isCursorVisible = false
        dialog.setOnShowListener {
            dialog.window?.setLayout(WindowManager.LayoutParams.MATCH_PARENT, WindowManager.LayoutParams.MATCH_PARENT)
            dialog.window?.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE or WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_HIDDEN)
            dialog.getButton(AlertDialog.BUTTON_NEGATIVE).setOnClickListener { close() }
            render()
        }
        dialog.setOnKeyListener { _, key, event ->
            if (key == android.view.KeyEvent.KEYCODE_BACK && event.action == android.view.KeyEvent.ACTION_UP) { close(); true } else false
        }
        dialog.setCanceledOnTouchOutside(false)
        dialog.setCancelable(false)
        dialog.setOnDismissListener { child?.dismiss() }
        dialog.show()
    }

    private fun render() {
        val changed = body.text.toString() != block.content
        status.text = when {
            saving -> "正在导出"
            !block.complete -> "未完整 · 只读"
            exported == body.text.toString() -> "副本已导出"
            changed -> "副本未导出"
            editing -> "本机副本"
            else -> "原文"
        }
        edit.isEnabled = block.complete && !saving
        reset.isEnabled = changed && !saving
        export.isEnabled = block.complete && !saving
        body.isEnabled = !saving
        edit.isSelected = editing
    }

    private fun chooseExport() {
        val formats = WebChatTextBlockExport.formats(block)
        child = AlertDialog.Builder(activity).setTitle("导出副本").setItems(formats.map { it.label }.toTypedArray()) { _, index ->
            val format = formats[index]
            val fileName = EditText(activity).apply { setSingleLine(true); setText(title); contentDescription = "web-chat-text-block-file-name" }
            child = AlertDialog.Builder(activity).setTitle("文件名").setView(fileName)
                .setPositiveButton("导出") { _, _ ->
                    val content = body.text.toString()
                    val stem = fileName.text.toString()
                    saving = true
                    render()
                    activity.lifecycleScope.launch {
                        val result = withContext(Dispatchers.IO) {
                            runCatching { WebChatTextBlockExport.save(activity.applicationContext, block, content, stem, format) }
                        }
                        saving = false
                        if (result.isSuccess) exported = content
                        if (dialog.isShowing) {
                            render()
                            Toast.makeText(activity, if (result.isSuccess) "副本已导出" else "导出失败，修改仍保留", Toast.LENGTH_LONG).show()
                        }
                    }
                }.setNegativeButton("取消", null).show()
        }.setNegativeButton("取消", null).show()
    }

    private fun close() {
        if (saving) return
        if (body.text.toString() == block.content || body.text.toString() == exported) { dialog.dismiss(); return }
        child = AlertDialog.Builder(activity).setTitle("修改尚未导出").setMessage("离开将丢弃本机修改，官网原文不受影响。")
            .setPositiveButton("继续编辑", null).setNegativeButton("丢弃修改") { _, _ -> dialog.dismiss() }.show()
    }

    private fun icon(resource: Int, label: String, id: String, action: () -> Unit) = AppCompatImageButton(activity).apply {
        setImageResource(resource)
        setPadding(dp(12), dp(12), dp(12), dp(12))
        background = null
        contentDescription = "web-chat-text-block-$id"
        TooltipCompat.setTooltipText(this, label)
        setOnClickListener { action() }
    }

    private fun dp(value: Int) = (value * density).toInt()
}
