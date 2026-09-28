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
    private val media = Executors.newFixedThreadPool(2)
    private val main = android.os.Handler(android.os.Looper.getMainLooper())
    private val api = ChatRecordApi(app)
    private val account = AuthManager.userId(app)
    private var local = emptyMap<String, File>()
    private val pending = mutableMapOf<String, MutableList<(Result<File>) -> Unit>>()
    private var group = ""; private var record = ""; private var draftId = ""
    @Volatile private var closed = false
    @Volatile private var generation = 0
    fun start(group: String, record: String, draft: String) {
        this.group = group; this.record = record; draftId = draft
        if (document == null && !loading) refresh()
    }
    fun sameAccount() = account == AuthManager.userId(getApplication()) &&
        (draftId.isNotBlank() || runCatching { api.assertOwner() }.isSuccess)
    private fun post(epoch: Int, block: () -> Unit) { main.post { if (!closed && epoch == generation && sameAccount()) block() } }
    private fun notifyChanged() { changed.value = (changed.value ?: 0) + 1 }
    fun refresh() {
        if (loading || closed) return
        api.cancelPending(); pending.clear()
        val epoch = ++generation
        val started = android.os.SystemClock.elapsedRealtime()
        loading = true; notice = ""; notifyChanged()
        worker.execute {
            val result = runCatching {
                check(!closed && epoch == generation && sameAccount()) { "记录已关闭或账号变化，请重新打开记录" }
                if (draftId.isNotBlank()) {
                    val d = ShareDraftStore(getApplication()).read(draftId) ?: error("导入预览已过期，请重新分享")
                    check(d.owner.isBlank() || d.owner == account) { "此导入草稿属于其他账号" }
                    val doc = requireNotNull(d.record)
                    post(epoch) { document = doc; local = d.files.associate { it.file.name.removePrefix("record_") to it.file } }
                } else {
                    api.peek(group, record)?.let { cached ->
                        val doc = ChatRecordDocument.read(cached.getJSONObject("document"))
                        post(epoch) {
                            document = doc; owner = cached.getString("owner_id"); notifyChanged()
                            android.util.Log.i("ChatRecordReader", "document_ready source=local elapsed_ms=${android.os.SystemClock.elapsedRealtime() - started}")
                        }
                    }
                    val view = api.read(group, record)
                    val doc = ChatRecordDocument.read(view.getJSONObject("document"))
                    post(epoch) {
                        document = doc; owner = view.getString("owner_id")
                        android.util.Log.i("ChatRecordReader", "document_ready source=validated elapsed_ms=${android.os.SystemClock.elapsedRealtime() - started}")
                    }
                }
            }
            val canKeep = result.isFailure && draftId.isBlank() && result.exceptionOrNull() !is ChatRecordCache.AccessDenied && runCatching { api.peek(group, record) != null }.getOrDefault(false)
            post(epoch) {
                result.exceptionOrNull()?.let {
                    if (!canKeep) { document = null; api.cancelPending(); pending.clear() }
                    notice = if (canKeep) "暂时无法更新，已显示近期验证的本地缓存" else it.message ?: "读取失败，请重试"
                }
                loading = false; notifyChanged()
            }
        }
    }
    fun file(row: RecordRow, done: (Result<File>) -> Unit) {
        if (closed) return
        val asset = row.assetId ?: return done(Result.failure(IllegalArgumentException("导出包未包含此附件")))
        pending[asset]?.let { it.add(done); return }
        val callbacks = mutableListOf(done); pending[asset] = callbacks
        val epoch = generation
        media.execute {
            val result = runCatching {
                check(!closed && epoch == generation && sameAccount()) { "记录已关闭或账号变化" }
                if (draftId.isNotBlank()) return@runCatching local[asset] ?: error("导出包未包含此附件")
                api.assetFile(group, record, asset)
            }
            post(epoch) {
                if (pending[asset] !== callbacks) return@post
                pending.remove(asset)
                if (result.exceptionOrNull() is ChatRecordCache.AccessDenied) {
                    generation++; document = null; notice = result.exceptionOrNull()?.message.orEmpty(); loading = false
                    api.cancelPending(); pending.clear(); notifyChanged()
                } else callbacks.forEach { it(result) }
            }
        }
    }
    fun poster(row: RecordRow, done: (Result<ChatRecordVideoPoster.Poster>) -> Unit) {
        file(row) { file -> file.onSuccess { source ->
            val epoch = generation
            if (closed) return@onSuccess
            media.execute {
                val result = runCatching { check(!closed && epoch == generation); ChatRecordVideoPoster.read(source) }
                post(epoch) { done(result) }
            }
        }.onFailure { done(Result.failure(it)) } }
    }
    fun revoke() {
        val epoch = ++generation; api.cancelPending(); pending.clear()
        loading = true; notifyChanged()
        worker.execute {
            val result = runCatching { api.revoke(group, record) }
            post(epoch) { result.onSuccess { document = null; notice = "聊天记录已撤回" }.onFailure { notice = it.message ?: "撤回失败" }; loading = false; notifyChanged() }
        }
    }
    fun cacheUsage(done: (Result<Pair<Long, Long>>) -> Unit) {
        val epoch = generation
        media.execute { val result = runCatching { api.usage(group, record) }; post(epoch) { done(result) } }
    }
    fun clearCache(all: Boolean) {
        val epoch = ++generation; api.cancelPending(); pending.clear(); loading = true; notifyChanged()
        worker.execute {
            val result = runCatching { api.clearCache(group, record, all) }
            post(epoch) { document = null; loading = false; notice = result.exceptionOrNull()?.message ?: "本地缓存已清理，群里的记录没有删除"; notifyChanged() }
        }
    }
    override fun onCleared() {
        closed = true; generation++; pending.clear(); api.close()
        worker.shutdownNow(); media.shutdownNow(); main.removeCallbacksAndMessages(null)
    }
}
