package com.elon.app

import android.os.SystemClock
import android.view.View
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebCanvasDocumentProtocol
import com.elon.app.chatgptweb.ChatGptWebCanvasDocuments
import com.elon.app.chatgptweb.ChatGptWebConversation
import com.elon.app.chatgptweb.ChatGptWebConversationPath
import org.json.JSONObject

internal class WebChatCanvasDocumentsCoordinator(
    private val activity: AppCompatActivity,
    private val host: View,
    private val provider: () -> WebChatProviderId?,
    private val port: () -> WebChatConsumerPort?,
    private val currentPath: () -> String?,
    private val openConversation: (String) -> WebChatConsumerCommandResult,
) {
    private var epoch = 0
    private var poll: Runnable? = null
    private var sheet: WebChatActionSheetHandle? = null
    private var sheetGeneration = 0
    private var editor: WebChatCanvasEditorView? = null
    private var management: WebChatCanvasManagementCoordinator? = null
    private var generation: WebChatCanvasGenerationCoordinator? = null
    private var generationWatch: Runnable? = null
    private var draft: WebChatCanvasDraft? = null
    private var index: ChatGptWebCanvasDocuments? = null
    private val downloads = WebChatFileDownloadDialog(activity, host, port)

    fun show(conversation: ChatGptWebConversation) {
        cancel()
        val owner = port() ?: return
        if (provider() != WebChatProviderId.CHATGPT_WEB || !ChatGptWebCanvasDocumentProtocol.validPath(conversation.path)) return
        val run = epoch
        fun load() {
            val path = ChatGptWebConversationPath.fromUrl(owner.state().pageUrl)
                ?.takeIf { ChatGptWebCanvasDocumentProtocol.validPath(it) && samePath(it) } ?: conversation.path
            execute(owner, path, JSONObject().put("operation", "list").put("path", path).put("force", false), false,
                { value, _ -> index = value; list(owner, path, value) },
                { reason -> status(reason, retry = { show(conversation) }) })
        }
        status("正在读取画布", loading = true)
        if (samePath(conversation.path)) return load()
        if (WebChatConversationDraftNavigation.blocks(conversation.path, currentPath(), owner.state().draftPresent)) {
            cancel()
            WebChatConversationDraftNavigation.dialog(activity).show()
            return
        }
        if (!openConversation(conversation.path).accepted) return status("暂时无法打开会话", retry = { show(conversation) })
        val started = SystemClock.elapsedRealtime()
        val wait = object : Runnable {
            override fun run() {
                if (!alive(run, owner)) { if (run == epoch) cancel(); return }
                if (samePath(conversation.path) && owner.state().adapterCurrent) { poll = null; load(); return }
                if (SystemClock.elapsedRealtime() - started >= 12_000) {
                    poll = null
                    status("会话尚未加载完成", retry = { show(conversation) })
                } else host.postDelayed(this, 250)
            }
        }
        poll = wait
        host.post(wait)
    }

    private fun list(owner: WebChatConsumerPort, path: String, value: ChatGptWebCanvasDocuments) {
        dismissSheet()
        var selected: String? = null
        val run = epoch
        val generation = sheetGeneration
        sheet = WebChatActionSheet.showUpdatable(activity, "画布",
            if (value.documents.isEmpty()) listOf(WebChatActionSheetItem("empty", "此会话暂无画布", enabled = false,
                contentDescription = "web-chat-canvas-documents-empty"))
            else value.documents.mapIndexed { position, document -> WebChatActionSheetItem(document.id, document.title,
                subtitle = "版本 ${document.documentVersion}", contentDescription = "web-chat-canvas-document-$position") },
            footerActions = listOf(WebChatActionSheetFooterAction("刷新", "web-chat-canvas-documents-refresh", dismissOnClick = false) {
                execute(owner, path, JSONObject().put("operation", "list").put("path", path).put("force", true), false,
                    { refreshed, _ -> index = refreshed; list(owner, path, refreshed) }, { reason -> status(reason) })
            }),
            onCancelled = { if (generation == sheetGeneration) cancel() },
            onDismissed = {
                if (generation == sheetGeneration) {
                    sheet = null
                    stopPolling()
                    selected?.let { id -> host.post { if (alive(run, owner)) openEditor(owner, value, id) } }
                }
            },
        ) { selected = it.id }
    }

    private fun openEditor(owner: WebChatConsumerPort, value: ChatGptWebCanvasDocuments, id: String) {
        val document = value.documents.firstOrNull { it.id == id } ?: return
        val retained = draft
        if (retained != null && retained.changed && (retained.base.id != id || retained.path != value.path || retained.scope != value.scope)) {
            val run = epoch
            androidx.appcompat.app.AlertDialog.Builder(activity).setTitle("保留了另一份画布草稿")
                .setMessage("打开其他画布前，是否放弃尚未保存的草稿？")
                .setPositiveButton("放弃并打开") { _, _ ->
                    if (alive(run, owner)) { draft = null; openEditor(owner, value, id) }
                }.setNegativeButton("保留", null).show()
            return
        }
        val editing = retained?.takeIf { it.changed && it.base.id == id && it.path == value.path && it.scope == value.scope }
            ?: WebChatCanvasDraft(value.path, value.scope, document)
        draft = editing
        index = value
        val run = epoch
        management?.cancel()
        management = WebChatCanvasManagementCoordinator(activity, editing,
            execute = { request, confirmed, done, failed ->
                execute(owner, editing.path, request, confirmed, { result, _ ->
                    index = result
                    if (result.scope != editing.scope || result.documents.none { it.id == editing.base.id }) {
                        failed("画布或登录身份已变化")
                    } else done(result)
                }, failed)
            }, state = { message, working -> editor?.render(message, working) },
            restored = { result ->
                val restored = result.documents.firstOrNull { it.id == editing.base.id }
                if (restored == null || result.scope != editing.scope || result.unconfirmedWrite || editing.changed) {
                    editor?.render("恢复结果待核对，草稿已保留", allowed = false)
                } else {
                    editing.adopt(restored)
                    editor?.render("已恢复 · 版本 ${restored.documentVersion}", allowed = true, reset = true)
                }
            }, renamed = { result ->
                val renamed = result.documents.firstOrNull { it.id == editing.base.id }
                if (result.scope == editing.scope && !result.unconfirmedWrite && renamed != null && editing.acceptRename(renamed)) {
                    editor?.render(if (editing.changed) "名称已更新，正文草稿未保存" else "名称已更新", allowed = true)
                } else editor?.render("名称结果待核对，草稿已保留", allowed = false)
            }, commentDismissed = { result ->
                val changed = result.documents.firstOrNull { it.id == editing.base.id }
                if (result.scope == editing.scope && !result.unconfirmedWrite && changed != null && editing.acceptCommentDismissal(changed)) {
                    editor?.render(if (editing.changed) "评论已忽略，正文草稿未保存" else "评论已忽略", allowed = true)
                } else editor?.render("评论结果待核对，草稿已保留", allowed = false)
            }, exported = { result ->
                val export = result.exportFile
                if (alive(run, owner) && !editing.changed && editing.matches(result) && !result.unconfirmedWrite &&
                    export != null && export.documentId == editing.base.id) {
                    val command = owner.downloadConversationFile(editing.path, export.file.id, export.file.downloadHandle)
                    val requestId = command.requestId
                    if (command.accepted && requestId != null) {
                        editor?.render("版本 ${editing.base.documentVersion}")
                        downloads.show(owner, requestId)
                    } else editor?.render("暂时无法导出，请重试")
                } else editor?.render("版本尚未确认，草稿已保留", allowed = false)
            })
        generation?.cancel()
        generation = WebChatCanvasGenerationCoordinator(activity, editing,
            execute = { request, confirmed, done, failed -> execute(owner, editing.path, request, confirmed,
                { result, _ -> index = result; done(result) }, failed) },
            state = { message, working, allowed -> editor?.render(message, working, allowed) },
            dispatched = { watchGeneration(owner, editing) })
        editor = WebChatCanvasEditorView(activity, editing,
            save = { save(owner, editing) }, check = { refresh(owner, editing) },
            history = { management?.showHistory() }, share = { management?.showShare() },
            rename = { management?.showRename() },
            export = { management?.showExport() },
            generate = { start, end -> generation?.show(start, end) },
            dismissComment = { management?.confirmDismissComment(it) },
            closed = { if (run == epoch) cancel() })
        editor?.show()
        if (value.unconfirmedWrite || !editing.matches(value)) editor?.render("官网版本需要核对，草稿已保留", allowed = false)
    }

    private fun watchGeneration(owner: WebChatConsumerPort, editing: WebChatCanvasDraft) {
        generationWatch?.let(host::removeCallbacks)
        val run = epoch
        val started = SystemClock.elapsedRealtime()
        var nextRead = started + 2_000
        var reads = 0
        val watcher = object : Runnable {
            override fun run() {
                if (!alive(run, owner) || !samePath(editing.path) || index?.unconfirmedWrite == false) {
                    generationWatch = null
                    return
                }
                val now = SystemClock.elapsedRealtime()
                if (now - started >= 90_000 || reads >= 6) {
                    generationWatch = null
                    editor?.render("改写结果待核对，原正文已保留", allowed = false)
                    return
                }
                // Read cached native stream state. Only idle transitions trigger bounded canonical reads.
                if (!owner.state().streaming && poll == null && now >= nextRead) {
                    reads += 1
                    nextRead = now + 10_000
                    refresh(owner, editing)
                }
                host.postDelayed(this, 1_000)
            }
        }
        generationWatch = watcher
        host.postDelayed(watcher, 1_000)
    }

    private fun save(owner: WebChatConsumerPort, editing: WebChatCanvasDraft) {
        val current = index ?: return
        val request = editing.saveRequest(current)
        if (request == null) { editor?.render("请先核对官网版本和评论位置"); return }
        editor?.render("正在保存", working = true)
        execute(owner, editing.path, request, true, { value, _ ->
            index = value
            val saved = value.documents.firstOrNull { it.id == editing.base.id }
            if (value.scope != editing.scope || saved == null || value.unconfirmedWrite) {
                editor?.render("保存结果待核对，草稿已保留", allowed = false)
            } else {
                editing.adopt(saved)
                editor?.render("已保存 · 版本 ${saved.documentVersion}", allowed = true, reset = true)
            }
        }, { reason -> editor?.render("$reason，草稿已保留", allowed = false) })
    }

    private fun refresh(owner: WebChatConsumerPort, editing: WebChatCanvasDraft) {
        editor?.render("正在核对官网版本", working = true)
        execute(owner, editing.path,
            JSONObject().put("operation", "list").put("path", editing.path).put("force", true), false,
            { value, _ ->
                index = value
                if (value.scope != editing.scope) editor?.render("登录或页面身份已变化，原草稿仅供复制", allowed = false)
                else if (value.unconfirmedWrite) verify(owner, editing, value, false) { checked -> review(owner, editing, checked) }
                else review(owner, editing, value)
            }, { reason -> editor?.render("$reason，草稿已保留", allowed = false) })
    }

    private fun review(owner: WebChatConsumerPort, editing: WebChatCanvasDraft, value: ChatGptWebCanvasDocuments) {
        index = value
        val server = value.documents.firstOrNull { it.id == editing.base.id }
        if (server == null || value.scope != editing.scope) {
            editor?.render("原画布或身份已变化，草稿仅供复制", allowed = false)
            return
        }
        if (!value.unconfirmedWrite && !editing.changed && server.documentVersion > editing.base.documentVersion) {
            editing.adopt(server)
            editor?.render("已更新 · 版本 ${server.documentVersion}", allowed = true, reset = true)
            return
        }
        if (!value.unconfirmedWrite && editing.acceptRename(server)) {
            editor?.render(if (editing.changed) "名称已更新，正文草稿未保存" else "名称已更新", allowed = true)
            return
        }
        if (!value.unconfirmedWrite && editing.acceptCommentDismissal(server)) {
            editor?.render(if (editing.changed) "评论已忽略，正文草稿未保存" else "评论已忽略", allowed = true)
            return
        }
        if (!value.unconfirmedWrite && server.content == editing.content && server.comments == editing.comments) {
            editing.adopt(server)
            editor?.render("已与官网一致", allowed = true, reset = true)
            return
        }
        if (!value.unconfirmedWrite && editing.matches(value)) {
            editor?.render("官网版本未变化，可继续编辑", allowed = true)
            return
        }
        editor?.render("请核对版本，草稿已保留", allowed = false)
        val run = epoch
        fun resolve(keepDraft: Boolean) {
            if (!alive(run, owner) || !samePath(editing.path)) return
            fun apply(checked: ChatGptWebCanvasDocuments) {
                val latest = checked.documents.firstOrNull { it.id == server.id }
                if (checked.scope != editing.scope || checked.unconfirmedWrite || latest != server) {
                    editor?.render("官网版本再次变化，请重新核对", allowed = false)
                    return
                }
                index = checked
                if (keepDraft) editing.rebase(server) else editing.adopt(server)
                editor?.render(if (keepDraft) "草稿已保留，保存前请再次确认" else "已采用官网版本", allowed = true, reset = true)
            }
            if (value.unconfirmedWrite) verify(owner, editing, value, true, ::apply) else apply(value)
        }
        editor?.compare(server, adopt = { resolve(false) }, rebase = { resolve(true) })
    }

    private fun verify(owner: WebChatConsumerPort, editing: WebChatCanvasDraft, value: ChatGptWebCanvasDocuments,
        confirmed: Boolean, done: (ChatGptWebCanvasDocuments) -> Unit) {
        editor?.render("正在核对上次保存结果", working = true)
        execute(owner, editing.path, editing.selection(value, "verify"), confirmed,
            { checked, detail ->
                index = checked
                if (detail == "canvas_generation_pending") editor?.render("正在等待改写结果，原正文已保留", allowed = false)
                else done(checked)
            }, { reason -> editor?.render("$reason，草稿已保留", allowed = false) })
    }

    private fun execute(owner: WebChatConsumerPort, path: String, request: JSONObject, confirmed: Boolean,
        done: (ChatGptWebCanvasDocuments, String?) -> Unit, failed: (String) -> Unit) {
        if (poll != null) { failed("已有画布操作正在处理，请稍后重试"); return }
        if (port() !== owner || !samePath(path) || provider() != WebChatProviderId.CHATGPT_WEB) {
            failed("当前会话已变化"); return
        }
        val command = owner.canvasDocument(request, confirmed)
        if (!command.accepted || command.requestId == null) { failed(failure(command.error)); return }
        val run = epoch
        val started = SystemClock.elapsedRealtime()
        val task = object : Runnable {
            override fun run() {
                if (!alive(run, owner)) { if (run == epoch) cancel(); return }
                if (!samePath(path)) { poll = null; failed("当前会话已变化"); return }
                val state = owner.state().commandRequests.firstOrNull { it.id == command.requestId }
                val value = owner.canvasDocuments()?.takeIf { it.requestId == command.requestId && it.path == path }
                if (state?.status == WebChatConsumerCommandStatus.SUCCEEDED && value != null) {
                    poll = null; done(value, state.detail); return
                }
                if (state?.status in setOf(WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT) ||
                    SystemClock.elapsedRealtime() - started >= 22_000) {
                    poll = null; failed(failure(state?.detail)); return
                }
                host.postDelayed(this, 200)
            }
        }
        poll = task
        host.post(task)
    }

    private fun status(message: String, loading: Boolean = false, retry: (() -> Unit)? = null) {
        dismissSheet()
        val generation = sheetGeneration
        sheet = WebChatActionSheet.showUpdatable(activity, "画布",
            listOf(WebChatActionSheetItem("status", message, enabled = false, contentDescription = "web-chat-canvas-documents-status")),
            footerActions = if (!loading && retry != null) listOf(WebChatActionSheetFooterAction("重试", "web-chat-canvas-documents-retry") { retry() }) else emptyList(),
            onCancelled = { if (generation == sheetGeneration) cancel() },
        ) {}
    }

    private fun samePath(path: String) = ChatGptWebConversationPath.identity(path) != null &&
        ChatGptWebConversationPath.identity(path) == ChatGptWebConversationPath.identity(currentPath())
    private fun alive(run: Int, owner: WebChatConsumerPort) = run == epoch && port() === owner &&
        provider() == WebChatProviderId.CHATGPT_WEB && !activity.isFinishing && !activity.isDestroyed

    fun cancel() {
        epoch += 1
        stopPolling()
        dismissSheet()
        management?.cancel(); management = null
        generation?.cancel(); generation = null
        generationWatch?.let(host::removeCallbacks); generationWatch = null
        downloads.dismiss()
        val view = editor; editor = null; view?.dismiss()
    }

    private fun stopPolling() {
        poll?.let(host::removeCallbacks)
        poll = null
    }

    private fun dismissSheet() {
        sheetGeneration += 1
        val previous = sheet
        sheet = null
        previous?.dismiss()
    }

    private fun failure(detail: String?): String = when (detail) {
        "canvas_version_conflict" -> "官网版本已变化"
        "canvas_title_invalid" -> "画布名称无效"
        "canvas_comment_invalid" -> "这条评论已变化，请核对官网版本"
        "canvas_write_unconfirmed" -> "保存结果尚未确认，请核对版本"
        "canvas_generation_unconfirmed" -> "改写请求结果尚未确认，请核对版本，不要重复提交"
        "canvas_generation_unavailable", "canvas_generation_owner_unavailable" -> "当前官网改写链路尚未就绪"
        "canvas_generation_blocked" -> "官网当前不允许发起改写"
        "canvas_share_write_unconfirmed", "canvas_share_verification_required" -> "分享结果尚未确认，请打开画布分享核对"
        "canvas_share_unconfirmed" -> "官网分享状态尚未确认"
        "canvas_history_unconfirmed", "canvas_history_changed" -> "历史版本尚未确认，请重新读取"
        "canvas_web_edit_pending", "canvas_conversation_busy", "canvas_busy" -> "官网画布正在编辑，请稍后重试"
        "canvas_auth_unavailable", "canvas_http_401", "canvas_http_403" -> "请确认官网登录状态"
        "canvas_timeout" -> "读取超时，请重试"
        "canvas_selection_expired", "canvas_context_changed", "canvas_history_selection_expired" -> "会话状态已变化，请核对版本"
        else -> "暂时未能完成，请重试"
    }
}
