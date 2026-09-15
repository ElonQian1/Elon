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
                candidates = d.files.mapIndexedNotNull { i, a -> if (a.mimeType.startsWith("image/")) i to ImageQrLinks.decode(a.file) else null }.toMap()
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
        work { targets = ShareTransport(getApplication()).targets() }
    }
    fun send(target: ShareTarget, text: String) {
        val d = draft ?: return
        if (busy || d.state != "ready") return
        if (d.owner.isNotBlank() && d.owner != AuthManager.userId(getApplication())) { notice = "账号已变化，请重新分享"; changed.value = (changed.value ?: 0) + 1; return }
        d.text = text; d.target = target.name; d.owner = AuthManager.userId(getApplication()).orEmpty()
        store.save(d)
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
    private fun work(action: () -> Unit) {
        busy = true; notice = ""; changed.value = (changed.value ?: 0) + 1
        worker.execute {
            runCatching(action).onFailure { notice = it.message ?: "操作失败，请重试" }
            busy = false; changed.postValue((changed.value ?: 0) + 1)
        }
    }
    override fun onCleared() { worker.shutdown() }
}
