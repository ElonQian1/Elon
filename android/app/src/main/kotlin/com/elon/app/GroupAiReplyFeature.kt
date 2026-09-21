package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

/** Group evidence never borrows the sharer's provider account; continuation uses the existing private-chat flow. */
internal class GroupAiReplyFeature(
    private val activity: AppCompatActivity,
    private val api: AiConversationShareApi,
    private val currentGroup: () -> AppGroup?,
    private val continuation: AiConversationPrivateContinuation,
    private val refresh: () -> Unit,
) : DefaultLifecycleObserver {
    private var job: Job? = null
    private var screen: AiConversationShareReaderView? = null
    private var activeMessage: ChatMessage? = null
    init { activity.lifecycle.addObserver(this) }

    fun open(message: ChatMessage, action: String) {
        val group = currentGroup() ?: return
        val id = message.id ?: return
        if (job?.isActive == true) return
        val session = socialSession(activity)
        fun valid() = !activity.isDestroyed && currentGroup()?.id == group.id && socialSession(activity) == session
        job = activity.lifecycleScope.launch {
            try {
                if (action == "continue") {
                    val snapshot = withContext(Dispatchers.IO) { api.groupReplySnapshot(group.id, id) }
                    if (valid()) { screen?.dialog?.dismiss(); continuation.show(snapshot) }
                } else {
                    val data = withContext(Dispatchers.IO) { api.groupReplySources(group.id, id) }
                    check(data.optString("group_id") == group.id && data.optString("message_id") == id)
                    if (valid()) {
                        if (action == "sharing") sharing(group, message, data, session)
                        else showSources(group, message, data)
                    }
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { if (valid()) {
                screen?.showNotice(R.string.ai_conversation_share_unavailable, canRetry = true)
                toast("记录未开放、已撤回或网络暂不可用，请重试")
            } }
        }
    }

    private fun sharing(group: AppGroup, message: ChatMessage, data: JSONObject, session: String) {
        if (data.optString("requester_id") != AuthManager.userId(activity)) return
        val allowed = data.optBoolean("allow_continue")
        AlertDialog.Builder(activity).setTitle("讨论分享设置")
            .setMessage(if (allowed) "已允许群成员用自己的 ChatGPT 继续讨论。关闭后不能再读取续聊上下文，但不会删除别人已保存的内容。"
                else "允许后，群成员可以读取本次所选记录、问题及回答，并用自己的 ChatGPT 私下继续讨论。不分享其他私人会话，也不创建公开链接。")
            .setNegativeButton("取消", null).setPositiveButton(if (allowed) "关闭分享" else "允许继续讨论") { _, _ ->
                activity.lifecycleScope.launch {
                    try {
                        check(session == socialSession(activity) && currentGroup()?.id == group.id)
                        withContext(Dispatchers.IO) { api.setGroupReplySharing(group.id, message.id!!, !allowed, data.getLong("version")) }
                        if (session == socialSession(activity)) { refresh(); toast("分享设置已更新") }
                    } catch (cancelled: CancellationException) { throw cancelled }
                    catch (_: Exception) { toast("设置未确认，请重新打开后重试") }
                }
            }.show()
    }

    private fun showSources(group: AppGroup, message: ChatMessage, data: JSONObject) {
        val position = screen?.position()
        screen?.dialog?.dismiss()
        val sources = data.getJSONArray("sources")
        val rows = (0 until sources.length()).map { index ->
            val source = sources.getJSONObject(index)
            val sender = source.optString("sender_user_id")
            ChatMessage("friend", source.optString("content"),
                id = source.optString("id"), senderUserId = sender,
                senderLabel = source.optString("sender_name"),
                senderAvatarDataUrl = group.members.firstOrNull { it.id == sender }?.avatarDataUrl,
                createdAtMs = parseChatMessageCreatedAt(source.optString("created_at")) ?: 0,
                recalledAt = source.optString("recalled_at").takeIf { it.isNotBlank() && it != "null" },
                attachments = chatAttachmentsFromJsonArray(source.optJSONArray("attachments")),
                webChatMessage = WebChatProductionMessage("snapshot", source.optString("id"), emptySet(), renderMarkdown = true))
        }.toMutableList()
        val card = AiConversationShareCard("preview", group.id, "${group.name}的聊天记录", "", "chatgpt", "群聊成员", rows.size)
        val snapshot = AiConversationShareSnapshot(card, rows, ownerId = data.optString("requester_id"))
        lateinit var view: AiConversationShareReaderView
        view = AiConversationShareReaderView(activity, card, null, {
            if (screen === view) { screen = null; activeMessage = null }
        }, { open(message, "sources") })
        screen = view; activeMessage = message
        view.show(); view.render(snapshot, rows, ChatAdapter(rows), position)
    }

    override fun onResume(owner: LifecycleOwner) {
        activeMessage?.let { message ->
            screen?.clear()
            screen?.showNotice(R.string.ai_conversation_share_loading, loading = true)
            open(message, "sources")
        }
    }
    override fun onDestroy(owner: LifecycleOwner) {
        job?.cancel(); screen?.dialog?.dismiss(); activity.lifecycle.removeObserver(this)
    }
    private fun toast(text: String) = Toast.makeText(activity, text, Toast.LENGTH_LONG).show()
}
