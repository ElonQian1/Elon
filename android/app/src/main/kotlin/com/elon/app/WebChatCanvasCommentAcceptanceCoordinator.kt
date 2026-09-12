package com.elon.app

import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebCanvasDocuments
import org.json.JSONObject

internal class WebChatCanvasCommentAcceptanceCoordinator(
    private val activity: AppCompatActivity,
    private val draft: WebChatCanvasDraft,
    private val execute: (JSONObject, Boolean, (ChatGptWebCanvasDocuments) -> Unit, (String) -> Unit) -> Unit,
    private val state: (String, Boolean, Boolean) -> Unit,
    private val dispatched: () -> Unit,
    private val recover: () -> Unit,
    private val stopped: (ChatGptWebCanvasDocuments) -> Unit,
) {
    private var epoch = 0
    private var dialog: AlertDialog? = null

    fun confirm(id: String) {
        val comment = draft.base.comments.firstOrNull { it.id == id } ?: return
        cancel()
        if (draft.changed) {
            dialog = AlertDialog.Builder(activity).setTitle("草稿尚未保存")
                .setMessage("请先保存正文和评论位置，再采纳评论。当前草稿会保留。")
                .setPositiveButton("返回编辑", null).show()
            return
        }
        val run = epoch
        dialog = AlertDialog.Builder(activity).setTitle("采纳评论并改写？")
            .setMessage(comment.content)
            .setPositiveButton("采纳并改写") { _, _ ->
                if (active(run)) {
                    state("正在核对评论", true, false)
                    execute(JSONObject().put("operation", "list").put("path", draft.path).put("force", true), false, { value ->
                        if (active(run)) {
                            val request = draft.acceptCommentRequest(value, id)
                            if (request == null) state("官网版本已变化，请核对，草稿已保留", false, false)
                            else send(request, run)
                        }
                    }, { reason -> if (active(run)) state(reason, false, false) })
                }
            }.setNegativeButton("取消", null).show()
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-canvas-comment-accept-confirm"
        dialog?.getButton(AlertDialog.BUTTON_NEGATIVE)?.contentDescription = "web-chat-canvas-comment-accept-cancel"
    }

    fun resolve(value: ChatGptWebCanvasDocuments) {
        cancel()
        if (!value.unconfirmedWrite || value.scope != draft.scope || value.path != draft.path ||
            value.documents.none { it.id == draft.base.id }) return
        val run = epoch
        state("评论已移除，改写尚未发送", false, false)
        dialog = AlertDialog.Builder(activity).setTitle("继续按评论改写？")
            .setMessage("评论已从官网移除，但 AI 改写尚未发送。可以继续改写，也可以保留当前正文。")
            .setPositiveButton("继续改写") { _, _ ->
                if (active(run) && !draft.changed) send(draft.selection(value, "resume_comment"), run)
            }.setNeutralButton("不再改写") { _, _ ->
                if (active(run)) {
                    state("正在确认保留正文", true, false)
                    execute(draft.selection(value, "verify"), true, { checked ->
                        if (active(run)) stopped(checked)
                    }, { reason -> if (active(run)) state(reason, false, false) })
                }
            }.setNegativeButton("稍后", null).show()
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-canvas-comment-accept-resume"
        dialog?.getButton(AlertDialog.BUTTON_NEUTRAL)?.contentDescription = "web-chat-canvas-comment-accept-stop"
        dialog?.getButton(AlertDialog.BUTTON_NEGATIVE)?.contentDescription = "web-chat-canvas-comment-accept-later"
    }

    private fun send(request: JSONObject, run: Int) {
        state("正在发起改写，原正文已保留", true, false)
        execute(request, true, {
            if (active(run)) {
                state("正在等待改写结果，原正文已保留", false, false)
                dispatched()
            }
        }, { reason ->
            if (active(run)) {
                state(reason, false, false)
                recover()
            }
        })
    }

    private fun active(run: Int) = run == epoch && !activity.isFinishing && !activity.isDestroyed
    fun cancel() { epoch += 1; dialog?.dismiss(); dialog = null }
}
