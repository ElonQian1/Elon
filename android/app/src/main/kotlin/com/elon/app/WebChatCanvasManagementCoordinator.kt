package com.elon.app

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.widget.LinearLayout
import android.widget.EditText
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebCanvasDocument
import com.elon.app.chatgptweb.ChatGptWebCanvasDocumentProtocol
import com.elon.app.chatgptweb.ChatGptWebCanvasDocuments
import com.elon.app.chatgptweb.ChatGptWebCanvasExportFormats
import com.elon.app.chatgptweb.ChatGptWebCanvasHistory
import org.json.JSONObject

internal class WebChatCanvasManagementCoordinator(
    private val activity: AppCompatActivity,
    private val draft: WebChatCanvasDraft,
    private val execute: (JSONObject, Boolean, (ChatGptWebCanvasDocuments) -> Unit, (String) -> Unit) -> Unit,
    private val state: (String, Boolean) -> Unit,
    private val restored: (ChatGptWebCanvasDocuments) -> Unit,
    private val renamed: (ChatGptWebCanvasDocuments) -> Unit,
    private val commentDismissed: (ChatGptWebCanvasDocuments) -> Unit,
    private val exported: (ChatGptWebCanvasDocuments) -> Unit,
) {
    private var epoch = 0
    private var dialog: AlertDialog? = null

    fun showHistory() = prepare { value -> history(value, value.documents.first { it.id == draft.base.id }.documentVersion) }
    fun showShare() = prepare { value -> share(value, "share_lookup", false) }

    fun showExport() {
        cancel()
        val run = epoch
        if (draft.changed) {
            dialog = AlertDialog.Builder(activity).setTitle("草稿尚未保存")
                .setMessage("导出使用官网已保存的版本。请先保存正文修改。")
                .setPositiveButton("返回编辑", null).show()
            return
        }
        val formats = ChatGptWebCanvasExportFormats.options(draft.base.documentType)
        if (formats.isEmpty()) return
        dialog = AlertDialog.Builder(activity).setTitle("导出画布")
            .setItems(formats.map { it.label }.toTypedArray()) { _, position ->
                if (active(run)) prepare { value ->
                    if (draft.changed || !draft.matches(value) || value.unconfirmedWrite) {
                        state("官网版本已变化，请先核对，草稿已保留", false)
                    } else request(draft.selection(value, "prepare_export").put("format", formats[position].key),
                        false, "正在准备导出", exported)
                }
            }.setNegativeButton("取消", null).show()
        dialog?.listView?.contentDescription = "web-chat-canvas-export-formats"
    }

    fun confirmDismissComment(id: String) {
        if (draft.base.comments.none { it.id == id }) return
        cancel()
        val run = epoch
        dialog = AlertDialog.Builder(activity).setTitle("忽略这条评论？")
            .setMessage("这条评论将从官网画布中移除，不会让 AI 改写正文。未保存的正文草稿会保留。")
            .setPositiveButton("忽略评论") { _, _ ->
                if (active(run)) prepare { value ->
                    val command = draft.dismissCommentRequest(value, id)
                    if (command == null) state("官网版本已变化，请先核对，草稿已保留", false)
                    else request(command, true, "正在忽略评论", commentDismissed)
                }
            }.setNegativeButton("取消", null).show()
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-canvas-comment-dismiss-confirm"
        dialog?.getButton(AlertDialog.BUTTON_NEGATIVE)?.contentDescription = "web-chat-canvas-comment-dismiss-cancel"
    }

    fun showRename() {
        cancel()
        val run = epoch
        val input = EditText(activity).apply {
            setSingleLine(true)
            setText(draft.base.title)
            selectAll()
            filters = arrayOf(android.text.InputFilter.LengthFilter(512))
            contentDescription = "web-chat-canvas-rename-title"
        }
        val padding = (16 * activity.resources.displayMetrics.density).toInt()
        val frame = LinearLayout(activity).apply {
            setPadding(padding, 0, padding, 0)
            addView(input, LinearLayout.LayoutParams(-1, -2))
        }
        dialog = AlertDialog.Builder(activity).setTitle("重命名画布").setView(frame)
            .setPositiveButton("保存名称", null).setNegativeButton("取消", null).show()
        dialog?.getButton(AlertDialog.BUTTON_NEGATIVE)?.contentDescription = "web-chat-canvas-rename-cancel"
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.apply {
            contentDescription = "web-chat-canvas-rename-confirm"
            setOnClickListener {
                if (!active(run)) return@setOnClickListener
                val title = input.text.toString().trim()
                if (!ChatGptWebCanvasDocumentProtocol.validTitle(title)) {
                    input.error = "请输入有效名称（最多 512 字符）"
                    return@setOnClickListener
                }
                if (title == draft.base.title) { dismissDialog(); return@setOnClickListener }
                isEnabled = false
                prepare { value ->
                    val command = draft.renameRequest(value, title)
                    if (command == null) state("官网版本已变化，请先核对，草稿已保留", false)
                    else request(command, true, "正在保存名称", renamed)
                }
            }
        }
    }

    private fun prepare(done: (ChatGptWebCanvasDocuments) -> Unit) {
        cancel()
        request(JSONObject().put("operation", "list").put("path", draft.path).put("force", true), false, "正在读取画布") { value ->
            if (value.scope != draft.scope || value.documents.none { it.id == draft.base.id }) {
                state("画布或登录身份已变化，草稿已保留", false)
            } else done(value)
        }
    }

    private fun history(value: ChatGptWebCanvasDocuments, before: Long) {
        request(draft.selection(value, "history").put("beforeVersion", before), false, "正在读取历史版本") { result ->
            val page = result.history
            if (page == null || page.documentId != draft.base.id) state("历史版本尚未确认", false)
            else showHistoryPage(result, page)
        }
    }

    private fun showHistoryPage(value: ChatGptWebCanvasDocuments, page: ChatGptWebCanvasHistory) {
        val run = epoch
        dismissDialog()
        val builder = AlertDialog.Builder(activity).setTitle("画布历史版本")
        if (page.versions.isEmpty()) builder.setMessage("没有更早的版本")
        else builder.setItems(page.versions.map { "版本 ${it.documentVersion} · ${it.title}" }.toTypedArray()) { _, position ->
            if (active(run)) preview(value, page, page.versions[position])
        }
        page.nextBeforeVersion?.let { before -> builder.setPositiveButton("更早版本") { _, _ ->
            if (active(run)) history(value, before)
        } }
        if (page.beforeVersion != value.documents.first { it.id == draft.base.id }.documentVersion) {
            builder.setNeutralButton("最近版本") { _, _ -> if (active(run)) showHistory() }
        }
        dialog = builder.setNegativeButton("返回编辑", null).show()
        dialog?.listView?.contentDescription = "web-chat-canvas-history-list"
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-canvas-history-older"
    }

    private fun preview(value: ChatGptWebCanvasDocuments, page: ChatGptWebCanvasHistory, version: ChatGptWebCanvasDocument) {
        val run = epoch
        dismissDialog()
        val padding = (16 * activity.resources.displayMetrics.density).toInt()
        val column = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(padding, padding, padding, padding)
            addView(TextView(activity).apply {
                text = version.content
                textSize = 16f
                setTextIsSelectable(true)
                contentDescription = "web-chat-canvas-history-content"
            })
            if (version.comments.isNotEmpty()) addView(TextView(activity).apply {
                text = version.comments.joinToString("\n\n", "\n评论\n") { it.content }
                setTextIsSelectable(true)
            })
        }
        val scroll = ScrollView(activity).apply { addView(column) }
        val frame = LinearLayout(activity).apply {
            addView(scroll, LinearLayout.LayoutParams(-1, (activity.resources.displayMetrics.heightPixels * 0.5).toInt()))
        }
        dialog = AlertDialog.Builder(activity).setTitle("版本 ${version.documentVersion}").setView(frame)
            .setPositiveButton("恢复此版本", null)
            .setNegativeButton("返回历史") { _, _ -> if (active(run)) showHistoryPage(value, page) }.show()
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.apply {
            contentDescription = "web-chat-canvas-history-restore"
            setOnClickListener {
                if (!active(run)) return@setOnClickListener
                if (draft.changed || !draft.matches(value) || value.unconfirmedWrite) {
                    notice("请先保存草稿或核对当前版本，再恢复历史")
                } else confirmRestore(value, page, version)
            }
        }
    }

    private fun confirmRestore(value: ChatGptWebCanvasDocuments, page: ChatGptWebCanvasHistory, version: ChatGptWebCanvasDocument) {
        dismissDialog()
        val run = epoch
        dialog = AlertDialog.Builder(activity).setTitle("恢复到版本 ${version.documentVersion}？")
            .setMessage("官网当前正文和评论将恢复为刚才预览的历史版本，并生成一个新版本。")
            .setPositiveButton("确认恢复") { _, _ ->
                if (active(run) && !draft.changed && draft.matches(value) && !value.unconfirmedWrite) {
                    request(draft.selection(value, "restore").put("historyTicket", page.ticket)
                        .put("restoreVersion", version.documentVersion), true, "正在恢复历史版本") { result ->
                        restored(result)
                    }
                }
            }.setNegativeButton("取消") { _, _ -> if (active(run)) preview(value, page, version) }.show()
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-canvas-history-restore-confirm"
    }

    private fun share(value: ChatGptWebCanvasDocuments, operation: String, confirmed: Boolean) {
        request(draft.selection(value, operation), confirmed, if (confirmed) "正在处理分享" else "正在读取分享状态") { result ->
            showShareResult(result)
        }
    }

    private fun showShareResult(value: ChatGptWebCanvasDocuments) {
        val share = value.share ?: return state("分享状态尚未确认", false)
        val run = epoch
        dismissDialog()
        val builder = AlertDialog.Builder(activity).setTitle("画布分享")
        when (share.state) {
            "public" -> builder.setMessage("已发布版本 ${share.documentVersion}\n\n${share.url}")
                .setPositiveButton("复制链接") { _, _ ->
                    if (active(run)) {
                        val clipboard = activity.getSystemService(Context.CLIPBOARD_SERVICE) as? ClipboardManager
                        clipboard?.setPrimaryClip(ClipData.newPlainText("画布分享", share.url))
                        if (clipboard != null) notice("链接已复制")
                    }
                }
            "restricted" -> builder.setMessage("当前分享访问受限。不会自动修改官网的访问范围。")
            "missing" -> {
                if (value.unconfirmedWrite) {
                    builder.setMessage("官网暂未查到分享链接，上次画布操作结果仍未确认。可以再次核对分享，或返回编辑核对版本。")
                        .setPositiveButton("再次核对") { _, _ -> if (active(run)) showShare() }
                        .setNeutralButton("核对创建结果") { _, _ ->
                            if (active(run)) confirmShareAcknowledgement(value)
                        }
                } else {
                    val canPublish = !draft.changed && draft.matches(value)
                    builder.setMessage(if (canPublish) "此画布尚未创建分享链接。" else "此画布尚未创建分享链接。请先保存草稿或核对官网版本。")
                    if (canPublish) builder.setPositiveButton("创建分享链接") { _, _ ->
                        if (active(run)) confirmShare(value)
                    }
                }
            }
        }
        dialog = builder.setNegativeButton("返回编辑", null).show()
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-canvas-original-share-primary"
    }

    private fun confirmShare(value: ChatGptWebCanvasDocuments) {
        dismissDialog()
        val run = epoch
        dialog = AlertDialog.Builder(activity).setTitle("创建画布分享链接？")
            .setMessage("当前已保存的画布内容将提交给官网创建分享链接。获得链接的人可能查看这些内容。")
            .setPositiveButton("确认创建") { _, _ ->
                if (active(run) && !draft.changed && draft.matches(value) && !value.unconfirmedWrite) share(value, "share_create", true)
            }.setNegativeButton("取消") { _, _ -> if (active(run)) showShareResult(value) }.show()
        dialog?.getButton(AlertDialog.BUTTON_POSITIVE)?.contentDescription = "web-chat-canvas-original-share-confirm"
    }

    private fun confirmShareAcknowledgement(value: ChatGptWebCanvasDocuments) {
        dismissDialog()
        val run = epoch
        dialog = AlertDialog.Builder(activity).setTitle("核对上次创建结果")
            .setMessage("将再次读取官网状态并确认本次结果，不会重新发送创建请求。")
            .setPositiveButton("核对并确认") { _, _ -> if (active(run)) share(value, "share_ack", true) }
            .setNegativeButton("取消", null).show()
    }

    private fun request(request: JSONObject, confirmed: Boolean, message: String, done: (ChatGptWebCanvasDocuments) -> Unit) {
        val run = epoch
        state(message, true)
        execute(request, confirmed, { value ->
            if (active(run)) { state("读取完成", false); done(value) }
        }, { reason ->
            if (active(run)) {
                state("$reason，原草稿已保留", false)
                dismissDialog()
                dialog = AlertDialog.Builder(activity).setTitle("操作尚未确认").setMessage(reason)
                    .setPositiveButton("返回编辑", null).show()
            }
        })
    }

    private fun active(run: Int) = run == epoch && !activity.isFinishing && !activity.isDestroyed
    private fun notice(message: String) = Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
    private fun dismissDialog() { val previous = dialog; dialog = null; previous?.dismiss() }
    fun cancel() { epoch += 1; dismissDialog() }
}
