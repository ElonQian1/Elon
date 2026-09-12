package com.elon.app

import android.text.InputFilter
import android.widget.EditText
import android.widget.LinearLayout
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebCanvasDocumentProtocol
import com.elon.app.chatgptweb.ChatGptWebCanvasDocuments
import org.json.JSONObject

internal class WebChatCanvasGenerationCoordinator(
    private val activity: AppCompatActivity,
    private val draft: WebChatCanvasDraft,
    private val execute: (JSONObject, Boolean, (ChatGptWebCanvasDocuments) -> Unit, (String) -> Unit) -> Unit,
    private val state: (String, Boolean, Boolean) -> Unit,
    private val dispatched: () -> Unit,
) {
    private var epoch = 0
    private var dialog: AlertDialog? = null

    fun show(start: Int, end: Int) {
        cancel()
        if (draft.changed) {
            dialog = AlertDialog.Builder(activity).setTitle("草稿尚未保存")
                .setMessage("请先保存正文，再让 AI 改写。当前草稿会保留。")
                .setPositiveButton("返回编辑", null).show()
            return
        }
        if (start > end || !ChatGptWebCanvasDocumentProtocol.boundary(draft.content, start) ||
            !ChatGptWebCanvasDocumentProtocol.boundary(draft.content, end)) return
        val run = epoch
        val input = EditText(activity).apply {
            hint = "改写要求"
            minLines = 2
            maxLines = 6
            filters = arrayOf(InputFilter.LengthFilter(4000))
            contentDescription = "web-chat-canvas-generation-prompt"
        }
        val padding = (16 * activity.resources.displayMetrics.density).toInt()
        val frame = LinearLayout(activity).apply {
            setPadding(padding, 0, padding, 0)
            addView(input, LinearLayout.LayoutParams(-1, -2))
        }
        dialog = AlertDialog.Builder(activity).setTitle(if (start == end) "改写全文" else "改写选区")
            .setView(frame).setPositiveButton("开始改写", null).setNegativeButton("取消", null).show()
        dialog?.getButton(AlertDialog.BUTTON_NEGATIVE)?.contentDescription = "web-chat-canvas-generation-cancel"
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.apply {
            contentDescription = "web-chat-canvas-generation-confirm"
            setOnClickListener {
                val prompt = input.text.toString().trim()
                if (prompt.isEmpty()) { input.error = "请输入改写要求"; return@setOnClickListener }
                if (!active(run)) return@setOnClickListener
                isEnabled = false
                dialog?.dismiss(); dialog = null
                state("正在准备改写", true, false)
                execute(JSONObject().put("operation", "list").put("path", draft.path).put("force", true), false, { value ->
                    if (!active(run)) return@execute
                    val command = draft.generationRequest(value, prompt, start, end)
                    if (command == null) state("版本已变化，请先核对，原正文已保留", false, false)
                    else execute(command, true, {
                        if (active(run)) {
                            state("改写请求已提交，原正文已保留", false, false)
                            dispatched()
                        }
                    }, { reason -> if (active(run)) state("$reason，原正文已保留", false, false) })
                }, { reason -> if (active(run)) state("$reason，原正文已保留", false, false) })
            }
        }
    }

    private fun active(run: Int) = run == epoch && !activity.isFinishing && !activity.isDestroyed
    fun cancel() { epoch += 1; dialog?.dismiss(); dialog = null }
}
