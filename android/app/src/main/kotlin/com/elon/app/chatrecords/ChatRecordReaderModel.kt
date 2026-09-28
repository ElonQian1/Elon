package com.elon.app.chatrecords

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.MutableLiveData
import com.elon.app.AuthManager
import com.elon.app.sharing.ShareDraftStore
import java.io.File
import java.util.concurrent.Executors

internal class ChatRecordReaderModel(app: Application) : AndroidViewModel(app) {
    val changed = MutableLiveData(0)
    var document: ChatRecordDocument? = null; private set
    var owner = ""; private set
    var notice = ""; private set
    var loading = false; private set
    var parent: String? = null
    val offsets = mutableMapOf<String, Int>()
    private val worker = Executors.newSingleThreadExecutor()
    private val api = ChatRecordApi(app)
    private val account = AuthManager.userId(app)
    private var local = emptyMap<String, File>()
    private val cache = linkedMapOf<String, File>()
    private var group = ""; private var record = ""; private var draftId = ""
    private var closed = false
    fun start(group: String, record: String, draft: String) {
        this.group = group; this.record = record; draftId = draft
        if (document == null && !loading) refresh()
    }
    fun sameAccount() = account == AuthManager.userId(getApplication())
    fun refresh() {
        if (loading) return
        loading = true; notice = ""; changed.value = (changed.value ?: 0) + 1
        worker.execute {
            runCatching {
                check(sameAccount()) { "账号已变化，请重新打开记录" }
                if (draftId.isNotBlank()) {
                    val d = ShareDraftStore(getApplication()).read(draftId) ?: error("导入预览已过期，请重新分享")
                    check(d.owner.isBlank() || d.owner == account) { "此导入草稿属于其他账号" }
                    document = requireNotNull(d.record)
                    local = d.files.associate { it.file.name.removePrefix("record_") to it.file }
                } else {
                    val view = api.read(group, record)
                    document = ChatRecordDocument.read(view.getJSONObject("document")); owner = view.getString("owner_id")
                }
            }.onFailure { document = null; notice = it.message ?: "读取失败，请重试" }
            loading = false; changed.postValue((changed.value ?: 0) + 1)
        }
    }
    fun file(row: RecordRow, done: (Result<File>) -> Unit) {
        worker.execute {
            val result = runCatching {
                check(!closed && sameAccount()) { "记录已关闭或账号变化" }
                val asset = requireNotNull(row.assetId)
                if (draftId.isNotBlank()) return@runCatching local[asset] ?: error("导出包未包含此附件")
                api.assertOwner()
                cache[asset]?.takeIf { it.isFile } ?: api.assetFile(group, record, asset).also { cache[asset] = it }
            }
            if (!closed) android.os.Handler(android.os.Looper.getMainLooper()).post { if (!closed && sameAccount()) done(result) }
        }
    }
    fun revoke() {
        loading = true; changed.value = (changed.value ?: 0) + 1
        worker.execute {
            runCatching { api.revoke(group, record); document = null; notice = "聊天记录已撤回" }
                .onFailure { notice = it.message ?: "撤回失败" }
            loading = false; changed.postValue((changed.value ?: 0) + 1)
        }
    }
    override fun onCleared() {
        closed = true
        worker.shutdown()
    }
}
