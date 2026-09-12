package com.elon.app

import android.graphics.Typeface
import android.text.Editable
import android.text.InputFilter
import android.text.TextWatcher
import android.view.WindowManager
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.AppCompatButton
import androidx.appcompat.widget.AppCompatImageButton
import androidx.appcompat.widget.TooltipCompat
import com.elon.app.chatgptweb.ChatGptWebCanvasDocument
import com.elon.app.chatgptweb.ChatGptWebCanvasDocumentProtocol

internal class WebChatCanvasEditorView(
    private val activity: AppCompatActivity,
    private val draft: WebChatCanvasDraft,
    save: () -> Unit,
    check: () -> Unit,
    history: () -> Unit,
    share: () -> Unit,
    rename: () -> Unit,
    closed: () -> Unit,
) {
    private val padding = (16 * activity.resources.displayMetrics.density).toInt()
    private val status = TextView(activity).apply { textSize = 14f; contentDescription = "web-chat-canvas-editor-status" }
    private val comments = AppCompatButton(activity).apply {
        isAllCaps = false
        contentDescription = "web-chat-canvas-editor-comments"
        setOnClickListener { showComments() }
    }
    private val historyButton = actionIcon(R.drawable.ic_popup_history, "历史版本", "web-chat-canvas-editor-history", history)
    private val shareButton = actionIcon(R.drawable.ic_project_post_share, "分享画布", "web-chat-canvas-editor-share", share)
    private val renameButton = actionIcon(R.drawable.ic_project_action_rename, "重命名画布", "web-chat-canvas-editor-rename", rename)
    private var replacing = false
    private var start = 0
    private var removed = 0
    private val body = EditText(activity).apply {
        setSingleLine(false)
        gravity = android.view.Gravity.TOP or android.view.Gravity.START
        inputType = android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_FLAG_MULTI_LINE or
            android.text.InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
        textSize = 16f
        if (draft.base.documentType != "document") typeface = Typeface.MONOSPACE
        contentDescription = "web-chat-canvas-editor-body"
        setText(draft.content)
        filters = arrayOf(InputFilter { _, from, to, dest, dstart, dend ->
            if (dest.length - (dend - dstart) + (to - from) > ChatGptWebCanvasDocumentProtocol.MAX_CONTENT) {
                Toast.makeText(activity, "内容超过画布编辑上限，本次输入未添加", Toast.LENGTH_SHORT).show()
                dest.subSequence(dstart, dend)
            } else null
        })
        addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {
                this@WebChatCanvasEditorView.start = start
                removed = count
            }
            override fun onTextChanged(s: CharSequence?, at: Int, before: Int, count: Int) {
                if (!replacing && s != null) draft.replace(start, removed, s.subSequence(at, at + count).toString())
            }
            override fun afterTextChanged(s: Editable?) { if (!replacing) render("草稿未保存") }
        })
    }
    private val layout = LinearLayout(activity).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(padding, padding / 2, padding, padding)
        addView(status)
        addView(LinearLayout(activity).apply {
            gravity = android.view.Gravity.CENTER_VERTICAL
            val size = (48 * activity.resources.displayMetrics.density).toInt()
            addView(comments, LinearLayout.LayoutParams(0, -2, 1f))
            addView(renameButton, LinearLayout.LayoutParams(size, size))
            addView(historyButton, LinearLayout.LayoutParams(size, size))
            addView(shareButton, LinearLayout.LayoutParams(size, size))
        })
        addView(body, LinearLayout.LayoutParams(-1, 0, 1f))
    }
    private val dialog = AlertDialog.Builder(activity).setTitle(draft.base.title).setView(layout)
        .setPositiveButton("保存", null).setNeutralButton("核对版本", null).setNegativeButton("返回", null).create()
    private var busy = false
    private var writable = true
    private var child: AlertDialog? = null

    init {
        dialog.setOnDismissListener { child?.dismiss(); closed() }
        dialog.setOnShowListener {
            dialog.window?.setLayout(WindowManager.LayoutParams.MATCH_PARENT, WindowManager.LayoutParams.MATCH_PARENT)
            dialog.window?.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).apply {
                contentDescription = "web-chat-canvas-editor-save"
                setOnClickListener { save() }
            }
            dialog.getButton(AlertDialog.BUTTON_NEUTRAL).apply {
                contentDescription = "web-chat-canvas-editor-check"
                setOnClickListener { check() }
            }
            dialog.getButton(AlertDialog.BUTTON_NEGATIVE).setOnClickListener { dialog.dismiss() }
            render("版本 ${draft.base.documentVersion}")
        }
    }

    fun show() = dialog.show()
    fun dismiss() = dialog.dismiss()
    fun render(message: String, working: Boolean = false, allowed: Boolean = writable, reset: Boolean = false) {
        busy = working
        writable = allowed
        dialog.setTitle(draft.base.title)
        if (reset) {
            replacing = true
            body.setText(draft.content)
            replacing = false
        }
        status.text = if (draft.needsRepair.isNotEmpty()) "$message · ${draft.needsRepair.size} 条评论需重新关联" else message
        comments.text = "评论（${draft.comments.size}）"
        comments.isEnabled = !busy && writable && draft.comments.isNotEmpty()
        historyButton.isEnabled = !busy
        shareButton.isEnabled = !busy
        renameButton.isEnabled = !busy && writable
        body.isEnabled = !busy
        // A detached account/document draft remains selectable, but cannot be modified or saved.
        body.isFocusableInTouchMode = writable && !busy
        dialog.getButton(AlertDialog.BUTTON_POSITIVE)?.isEnabled = !busy && writable && draft.changed && draft.needsRepair.isEmpty()
        dialog.getButton(AlertDialog.BUTTON_NEUTRAL)?.isEnabled = !busy
    }

    private fun actionIcon(resource: Int, label: String, semanticId: String, action: () -> Unit) = AppCompatImageButton(activity).apply {
        setImageResource(resource)
        val inset = (12 * activity.resources.displayMetrics.density).toInt()
        setPadding(inset, inset, inset, inset)
        background = null
        contentDescription = semanticId
        TooltipCompat.setTooltipText(this, label)
        setOnClickListener { action() }
    }

    private fun showComments() {
        val selectionStart = body.selectionStart
        val selectionEnd = body.selectionEnd
        child = AlertDialog.Builder(activity).setTitle("画布评论")
            .setItems(draft.comments.map { (if (it.id in draft.needsRepair) "待关联 · " else "") + it.content }.toTypedArray()) { _, index ->
                val comment = draft.comments[index]
                child = AlertDialog.Builder(activity).setTitle("评论").setMessage(comment.content)
                    .setPositiveButton("关联选区") { _, _ ->
                        val accepted = draft.reanchor(comment.id, minOf(selectionStart, selectionEnd), maxOf(selectionStart, selectionEnd))
                        render(if (accepted) "评论位置已更新，尚未保存" else "请先选中正文，再关联评论")
                    }.setNegativeButton("返回", null).show()
            }.setNegativeButton("返回", null).show()
    }

    fun compare(server: ChatGptWebCanvasDocument, adopt: () -> Unit, rebase: () -> Unit) {
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(padding, padding, padding, padding)
            for ((label, text) in listOf("我的草稿" to draft.content, "官网版本 ${server.documentVersion}" to server.content)) {
                addView(TextView(activity).apply { this.text = label; textSize = 18f; setPadding(0, padding, 0, padding / 2) })
                addView(TextView(activity).apply { this.text = text; textSize = 15f; setTextIsSelectable(true) })
            }
        }
        val scroll = ScrollView(activity).apply { addView(content); contentDescription = "web-chat-canvas-version-comparison" }
        val frame = LinearLayout(activity).apply {
            addView(scroll, LinearLayout.LayoutParams(-1, (activity.resources.displayMetrics.heightPixels * 0.55).toInt()))
        }
        child = AlertDialog.Builder(activity).setTitle("版本核对").setView(frame)
            .setPositiveButton("继续编辑我的草稿") { _, _ -> rebase() }
            .setNeutralButton("采用官网版本") { _, _ ->
                child = AlertDialog.Builder(activity).setTitle("替换本机草稿？")
                    .setMessage("尚未保存的修改将被当前官网版本替换。")
                    .setPositiveButton("替换") { _, _ -> adopt() }.setNegativeButton("取消", null).show()
            }.setNegativeButton("保留草稿", null).show()
    }
}
