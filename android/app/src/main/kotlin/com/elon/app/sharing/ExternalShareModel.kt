package com.elon.app.sharing

import android.app.Application
import android.content.Intent
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.MutableLiveData
import com.elon.app.AuthManager
import java.util.concurrent.Executors

internal class ExternalShareModel(app: Application) : AndroidViewModel(app) {
    val changed = MutableLiveData(0)
    val store = ShareDraftStore(app)
    var draft: ShareDraft? = null
    var targets = emptyList<ShareTarget>()
    var candidates = emptyMap<Int, List<SourceLink>>()
    var busy = false
    var notice = ""
    private val worker = Executors.newSingleThreadExecutor()
    fun start(intent: Intent, restore: String?) {
        if (draft != null || busy) return
        work {
            draft = restore?.let(store::read) ?: store.import(intent)
            draft?.let { d ->
                if (d.state == "sending") { d.state = "uncertain"; store.save(d) }
                candidates = if (d.record != null) emptyMap() else d.files.mapIndexedNotNull { i, a -> if (a.mimeType.startsWith("image/")) i to ImageQrLinks.decode(a.file) else null }.toMap()
                candidates.forEach { (i, links) -> if (d.files[i].sourceLink == null) d.files[i].sourceLink = links.singleOrNull() ?: if (links.isEmpty()) SourceLink.fromText(d.text) else null }
                store.save(d)
            }
        }
    }
    fun loadTargets() {
        if (busy || !AuthManager.isLoggedIn(getApplication())) return
        val owner = AuthManager.userId(getApplication()).orEmpty()
        val d = draft ?: return
        if (d.owner.isNotBlank() && d.owner != owner) return
        d.owner = owner; store.save(d)
        work { targets = ShareTransport(getApplication()).targets().filter { d.record == null || it.kind == "group" } }
    }
    fun send(target: ShareTarget, text: String) {
        val d = draft ?: return
        if (busy || (d.state != "ready" && !(d.record != null && d.state == "uncertain" && target.id == d.recordTargetId))) return
        if (d.owner.isNotBlank() && d.owner != AuthManager.userId(getApplication())) { notice = "账号已变化，请重新分享"; changed.value = (changed.value ?: 0) + 1; return }
        d.text = text; d.target = target.name; d.owner = AuthManager.userId(getApplication()).orEmpty()
        store.save(d)
        if (d.record != null) { work { sendRecord(d, target) }; return }
        work {
            val api = ShareTransport(getApplication())
            check(d.owner == AuthManager.userId(getApplication())) { "账号已变化，请重新分享" }
            val refs = api.upload(d)
            d.state = "sending"; store.save(d)
            try {
                val result = api.send(target, text, refs)
                check(!result.optJSONObject("message")?.optString("id").isNullOrBlank())
                d.state = "sent"; store.save(d); notice = "已发送到 ${target.name}"
            } catch (error: Exception) {
                d.state = "uncertain"; store.save(d)
                error("发送结果未确认，请到 ${target.name} 核对，避免重复发送")
            }
        }
    }
    private fun sendRecord(d: ShareDraft, target: ShareTarget) {
        require(target.kind == "group")
        val api = com.elon.app.chatrecords.ChatRecordApi(getApplication())
        require(d.recordTargetId.isBlank() || d.recordTargetId == target.id) { "请先确认上一群聊的发送结果" }
        d.recordTargetId = target.id; store.save(d)
        if (d.publishedRecord == null) {
            val local = requireNotNull(d.record)
            require(local.title.isNotBlank() && local.title.length <= 120) { "请填写不超过 120 字的聊天记录标题" }
            val uploaded = local.messages.mapNotNull { it.assetId }.distinct().associateWith { asset ->
                val file = d.files.single { it.file.name == "record_$asset" }.file
                api.upload(target.id, file)
            }
            d.publishedRecord = local.copy(messages = local.messages.map { it.copy(assetId = it.assetId?.let(uploaded::getValue)) })
            store.save(d)
        }
        d.state = "sending"; store.save(d)
        try {
            val result = api.publish(target.id, d.id, requireNotNull(d.publishedRecord))
            check(!result.optJSONObject("message")?.optString("id").isNullOrBlank())
            d.state = "sent"; store.save(d); notice = "已发送到 ${target.name}"
        } catch (e: Exception) { d.state = "uncertain"; store.save(d); error("发送结果未确认，可重试同一记录，不会重复发送。${e.message.orEmpty()}") }
    }
    private fun work(action: () -> Unit) {
        busy = true; notice = ""; changed.value = (changed.value ?: 0) + 1
        worker.execute {
            runCatching(action).onFailure { notice = it.message ?: "操作失败，请重试" }
            busy = false; changed.postValue((changed.value ?: 0) + 1)
        }
    }
    override fun onCleared() { worker.shutdown() }
}
