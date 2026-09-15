package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient

internal class MainAiConversationShareFeature(
    private val activity: AppCompatActivity,
    http: OkHttpClient,
    server: String,
    private val isAiChat: () -> Boolean,
    private val currentMessages: () -> List<ChatMessage>,
    private val streaming: () -> Boolean,
    private val groupId: () -> String?,
    private val onPublished: () -> Unit,
) : DefaultLifecycleObserver {
    private val api = AiConversationShareApi(activity, http, server)
    private val reader = AiConversationShareReader(activity, http, server)
    private val picker = AiConversationShareTargetPicker(activity, http, server)
    private val media = AiConversationShareMediaPreparer(activity)
    private var preview: AiConversationSharePreview? = null
    private var job: Job? = null

    init { activity.lifecycle.addObserver(this) }

    fun forwardOne(message: ChatMessage): Boolean {
        if (!isAiChat() || message.id?.startsWith("chatgpt_web:") != true) return false
        forward(listOf(message.copyForSharing())) {}
        return true
    }

    fun forward(selected: List<ChatMessage>, onComplete: () -> Unit) {
        if (preview?.isShowing() == true || job?.isActive == true) return
        if (!AuthManager.isLoggedIn(activity)) { toast("请先登录一龙账号后分享到群聊"); return }
        val draft = try { AiConversationShareDraftBuilder.build(currentMessages(), selected, streaming()) }
        catch (failure: RuntimeException) { toast(failure.message ?: "请重新选择消息"); return }
        if (draft.provider != "chatgpt") { toast("当前仅支持分享 ChatGPT 精选聊天记录"); return }
        val session = socialSession(activity)
        lateinit var screen: AiConversationSharePreview
        screen = AiConversationSharePreview(activity, draft,
            onPreview = {
                val card = AiConversationShareCard("preview", "preview", draft.title, draft.summary,
                    draft.provider, AuthManager.displayName(activity).orEmpty().ifBlank { "分享者" }, draft.messages.size)
                reader.showSnapshot(AiConversationShareSnapshot(card, draft.messages,
                    ownerId = AuthManager.userId(activity).orEmpty(), gaps = draft.gaps))
            },
            onSelectTarget = picker::show,
            onSubmit = { target, title, summary, cover ->
                if (job?.isActive != true) job = activity.lifecycleScope.launch {
                    screen.progress("正在准备分享") { job?.cancel(); screen.dismiss() }
                    try {
                        check(session == socialSession(activity)) { "账号已变化，请重新打开分享" }
                        val assets = media.prepare(draft.messages) { image ->
                            withContext(Dispatchers.IO) { api.upload(target.id, image, session) }
                        }
                        ensureActive()
                        val document = AiConversationShareCodec.document(draft, title, summary, assets, cover)
                        screen.progress("正在发送到 ${target.name}")
                        withContext(Dispatchers.IO) { api.publish(target.id, document, session) }
                        ensureActive()
                        api.acknowledgePublish(target.id, document, session)
                        screen.dismiss(); onComplete(); onPublished(); toast("已发送到 ${target.name}")
                    } catch (cancelled: CancellationException) { throw cancelled }
                    catch (failure: Exception) {
                        screen.failed(when (failure) {
                            is AiConversationShareMediaException -> failure.userMessage()
                            is AiConversationShareApiException -> failure.message.orEmpty()
                            is IllegalArgumentException, is IllegalStateException -> failure.message ?: "所选内容无法分享"
                            else -> "发送暂未确认，请重试；不会重复发送同一份记录"
                        })
                    }
                }
            },
            onDismiss = { picker.close(); if (preview === screen) preview = null },
        )
        preview = screen
        screen.show()
    }

    fun open(card: AiConversationShareCard) {
        if (card.groupId != groupId()) { toast("请从原群聊打开这份记录"); return }
        reader.show(card)
    }

    fun loadCover(card: AiConversationShareCard, loaded: (String?) -> Unit) {
        if (card.groupId != groupId()) loaded(null) else reader.loadCover(card, loaded)
    }

    fun showCardActions(anchor: android.view.View, message: ChatMessage, card: AiConversationShareCard) {
        if (card.groupId != groupId()) return
        androidx.appcompat.widget.PopupMenu(activity, anchor).apply {
            menu.add("查看聊天记录").setOnMenuItemClickListener { open(card); true }
            if (message.role == "user") menu.add("撤回分享").setOnMenuItemClickListener {
                androidx.appcompat.app.AlertDialog.Builder(activity).setTitle("撤回这份分享？")
                    .setMessage("群成员将无法再打开这份聊天记录。")
                    .setNegativeButton("取消", null).setPositiveButton("撤回") { _, _ ->
                        activity.lifecycleScope.launch {
                            try {
                                withContext(Dispatchers.IO) { api.revoke(card) }
                                reader.invalidate(card); onPublished(); toast("分享已撤回")
                            } catch (cancelled: CancellationException) { throw cancelled }
                            catch (_: Exception) { toast("撤回未确认，请稍后重试") }
                        }
                    }.show()
                true
            }
        }.show()
    }

    override fun onDestroy(owner: LifecycleOwner) {
        job?.cancel(); preview?.dismiss(); picker.close(); reader.dispose()
        activity.lifecycle.removeObserver(this)
    }

    private fun toast(message: String) = Toast.makeText(activity, message, Toast.LENGTH_LONG).show()
}
