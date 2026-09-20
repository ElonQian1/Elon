package com.elon.app

import android.os.Handler
import android.os.Looper
import android.widget.EditText
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner

/** Hands an explicit draft to the existing personal composer; never calls the group send API. */
internal class AiConversationPrivateContinuation(
    private val activity: AppCompatActivity,
    private val nativeChat: () -> MainSocialAiChatFeature,
    private val input: EditText,
    private val sourceGroup: () -> AppGroup?,
    private val openGroup: (AppGroup) -> Unit,
) : DefaultLifecycleObserver {
    private val handler = Handler(Looper.getMainLooper())
    private var generation = 0
    private var returnTarget: AppGroup? = null
    private var owner: String? = null
    private var busy = false
    private var returnConfirmation = false

    init { activity.lifecycle.addObserver(this) }

    fun show(snapshot: AiConversationShareSnapshot) {
        if (busy) return
        val group = sourceGroup()?.takeIf { it.id == snapshot.card.groupId } ?: return
        val draft = runCatching { AiConversationContinuationPrompt.build(snapshot.messages) }
            .getOrElse { toast(it.message ?: "这份记录暂不能继续讨论"); return }
        AlertDialog.Builder(activity).setTitle("用自己的 ChatGPT 继续讨论")
            .setMessage("将在你的空白会话中准备这份选段的文字，确认后由你按发送。图片仍可在分享记录中查看，不会自动上传。后续对话不会发送到群里。")
            .setNegativeButton("取消", null)
            .setPositiveButton("进入私人会话") { _, _ -> start(group, draft) }.show()
    }

    private fun start(group: AppGroup, draft: String) {
        if (busy || sourceGroup()?.id != group.id) return
        if (!input.text.isNullOrBlank()) { toast("请先保留或处理当前输入框中的草稿"); return }
        busy = true
        val ticket = ++generation
        val session = socialSession(activity)
        owner = session
        returnTarget = group
        nativeChat().openChatGptWeb()
        awaitReady(ticket, session, draft, null, 0)
    }

    private fun awaitReady(ticket: Int, session: String, draft: String, request: String?, attempt: Int) {
        if (ticket != generation || activity.isDestroyed) return
        if (session != socialSession(activity)) { cancel(); toast("账号已变化，请重新打开分享"); return }
        val chat = nativeChat()
        if (!chat.isChatModeActive() || chat.providerId() != WebChatProviderId.CHATGPT_WEB) {
            if (attempt > 3) { cancel(); return }
        }
        val port = chat.chatGptConsumerPort()
        val state = port?.state()
        if (attempt >= 60) { busy = false; toast("私人会话尚未就绪，未填入或发送任何分享内容"); return }
        if (request == null && state?.adapterCurrent == true && chat.webChatComposerReady()) {
            if (state.streaming || state.draftPresent || !input.text.isNullOrBlank()) {
                busy = false; toast("原会话正在处理或有草稿，未覆盖。请先处理后重试"); return
            }
            val result = port.executeSessionCommand("chatgpt_new_conversation")
            if (!result.accepted || result.requestId.isNullOrBlank()) {
                busy = false; toast("未确认创建私人会话，未发送分享内容"); return
            }
            handler.postDelayed({ awaitReady(ticket, session, draft, result.requestId, attempt + 1) }, 250)
            return
        }
        if (request != null && state != null) {
            val receipt = state.commandRequests.firstOrNull { it.id == request }
            if (receipt?.status in listOf(WebChatConsumerCommandStatus.FAILED, WebChatConsumerCommandStatus.TIMED_OUT)) {
                busy = false; toast("新会话未确认，分享内容没有发送"); return
            }
            val empty = chat.currentMessages().none { it.role in listOf("user", "friend", "assistant", "ai") && it.content.isNotBlank() }
            if (receipt?.status == WebChatConsumerCommandStatus.SUCCEEDED &&
                AiConversationContinuationPrompt.freshRoute(state.pageUrl) && empty && !state.streaming && !state.draftPresent) {
                if (!input.text.isNullOrBlank()) { busy = false; toast("保留你刚输入的草稿，未覆盖"); return }
                input.setText(draft)
                input.setSelection(input.text.length)
                input.requestFocus()
                busy = false
                toast("已准备为私人会话草稿，可补充问题后发送；返回键回到原群聊")
                return
            }
        }
        handler.postDelayed({ awaitReady(ticket, session, draft, request, attempt + 1) }, 250)
    }

    fun returnToGroup(): Boolean {
        val group = returnTarget ?: return false
        if (owner != socialSession(activity)) { cancel(); return false }
        if (!nativeChat().isChatModeActive()) { cancel(); return false }
        if (!input.text.isNullOrBlank()) {
            if (!returnConfirmation) {
                returnConfirmation = true
                AlertDialog.Builder(activity).setTitle("放弃未发送的私人草稿？")
                    .setMessage("这份草稿不会带入群聊。")
                    .setNegativeButton("继续编辑", null)
                    .setPositiveButton("放弃并返回") { _, _ ->
                        if (owner == socialSession(activity)) { input.text?.clear(); cancel(); openGroup(group) }
                    }.setOnDismissListener { returnConfirmation = false }.show()
            }
        } else { cancel(); openGroup(group) }
        return true
    }

    private fun cancel() { generation++; busy = false; returnTarget = null; owner = null; handler.removeCallbacksAndMessages(null) }
    override fun onDestroy(owner: LifecycleOwner) = cancel()
    private fun toast(text: String) = Toast.makeText(activity, text, Toast.LENGTH_LONG).show()
}
