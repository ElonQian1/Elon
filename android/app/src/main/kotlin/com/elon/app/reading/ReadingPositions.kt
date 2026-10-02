package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import okhttp3.MediaType.Companion.toMediaType
import org.json.JSONArray
import org.json.JSONObject
import java.net.URLEncoder
import java.security.MessageDigest
import java.util.UUID
import java.util.concurrent.TimeUnit

/** Private metadata only. Each durable operation keeps its ID until an exact server acknowledgement. */
internal class ReadingPositions(private val context: Context, private val http: OkHttpClient, private val server: String,
    val scope: String, private val changed: () -> Unit) {
    private val session = socialSession(context)
    private val owner = AuthManager.userId(context).orEmpty()
    private val key = MessageDigest.getInstance("SHA-256").digest("$server|$owner|$scope".toByteArray()).joinToString("") { "%02x".format(it) }
    private val prefs = context.getSharedPreferences("reading_positions_v1", Context.MODE_PRIVATE)
    private var state = runCatching { JSONObject(prefs.getString(key, "{}")!!) }.getOrDefault(JSONObject())
    private val main = Handler(Looper.getMainLooper())
    private var alive = true
    private var busy = false
    private var sending = ""
    private val device = UUID.randomUUID().toString()
    private var sequence = 0L
    private var activeRevision = state.optJSONObject("conversation_progress")?.optLong("revision") ?: 0L
    private var firstLoad = true
    private var positionChanged = false
    var active: String? = ""; private set
    var supported = state.optBoolean("supported"); private set
    var denied = false; private set
    var error = ""; private set
    val bookmarks: List<JSONObject> get() = array(state.optJSONArray("bookmarks"))
    val pending: Int get() = queue().size
    val blocked: List<JSONObject> get() = queue().filter { it.has("blocked") }
    private val flushTask = Runnable { flush() }
    private fun valid() = alive && session == socialSession(context)
    private fun queue() = array(state.optJSONArray("queue"))
    private fun item(id: String) = bookmarks.find { it.optString("id") == id }
    fun progress(id: String) = if (id.isEmpty()) state.optJSONObject("conversation_progress") else item(id)?.optJSONObject("progress")
    fun candidates(id: String) = array(if (id.isEmpty()) state.optJSONArray("conversation_candidates") else item(id)?.optJSONArray("candidates"))
    fun position(id: String, resume: Boolean = true): JSONObject? = if (resume) queue().lastOrNull { it.optString("bookmark_id") == id && it.optString("action") in listOf("progress", "resolve") }?.optJSONObject("position")
        ?: progress(id)?.optJSONObject("position") ?: item(id)?.optJSONObject("anchor") else item(id)?.optJSONObject("anchor")
    fun activate(id: String?) { active = id; activeRevision = id?.let { progress(it)?.optLong("revision") } ?: 0L; changed() }
    private fun save(edit: () -> Unit): Boolean {
        val before = state.toString()
        return try {
            edit(); val text = state.toString()
            check(text.length <= 1_000_000 && queue().size <= 1000) { "本机书签空间已满，请同步后重试" }
            check(prefs.edit().putString(key, text).commit()) { "本机保存失败" }
            error = ""; changed(); true
        } catch (e: Exception) { state = JSONObject(before); error = e.message ?: "保存失败"; changed(); false }
    }
    private fun operation(action: String, id: String) = JSONObject().put("kind", scope.substringBefore(':')).put("id", scope.substringAfter(':'))
        .put("project", "").put("action", action).put("bookmark_id", id).put("operation_id", UUID.randomUUID().toString())
        .put("device_id", device).put("device_seq", ++sequence)
    private fun enqueue(op: JSONObject) { state.put("queue", JSONArray(queue() + op)) }
    private fun schedule() { main.removeCallbacks(flushTask); main.postDelayed(flushTask, 3000) }
    fun add(message: ChatMessage, title: String, note: String, duplicate: Boolean = false): String? {
        val messageId = message.id ?: return null
        if (!supported || denied) return null
        if (!duplicate) bookmarks.find { it.optJSONObject("anchor")?.optString("message_id") == messageId }?.let { return it.getString("id") }
        val id = UUID.randomUUID().toString(); val label = title.trim().ifBlank { "书签 ${java.time.Instant.now()}" }.take(80)
        val position = JSONObject().put("message_id", messageId).put("fraction", 0.0)
        val ok = save {
            state.put("bookmarks", JSONArray(bookmarks + JSONObject().put("id", id).put("title", label).put("note", note).put("anchor", position).put("revision", 1).put("local", true)))
            enqueue(operation("create", id).put("title", label).put("note", note).put("position", position))
        }
        if (ok) schedule()
        return if (ok) id else null
    }
    fun rename(id: String, title: String, note: String): Boolean {
        val item = item(id) ?: return false
        val ok = save {
            val create = queue().find { it.optString("bookmark_id") == id && it.optString("action") == "create" && !it.optBoolean("attempted") && it.optString("operation_id") != sending }
            if (create != null) create.put("title", title).put("note", note)
            else enqueue(operation("update", id).put("base_revision", item.optLong("revision")).put("title", title).put("note", note))
            item.put("title", title).put("note", note)
        }
        if (ok) schedule(); return ok
    }
    fun remove(id: String): Boolean {
        val item = item(id) ?: return false
        val ok = save {
            val unsent = queue().any { it.optString("bookmark_id") == id && it.optString("action") == "create" && !it.optBoolean("attempted") && it.optString("operation_id") != sending }
            state.put("queue", JSONArray(queue().filter { it.optString("bookmark_id") != id || it.optString("operation_id") == sending || (it.optString("action") == "create" && it.optBoolean("attempted")) }))
            if (!unsent) enqueue(operation("delete", id).put("base_revision", item.optLong("revision")))
            state.put("bookmarks", JSONArray(bookmarks.filter { it.optString("id") != id }))
        }
        if (ok) { if (active == id) activate(null); schedule() }; return ok
    }
    fun putPosition(position: JSONObject, explicitId: String? = active): Boolean {
        val id = explicitId ?: return false
        if (denied) return false
        val ok = save {
            state.put("queue", JSONArray(queue().filterNot { it.optString("action") == "progress" && it.optString("bookmark_id") == id && it.optString("operation_id") != sending }))
            enqueue(operation("progress", id).put("base_revision", if (active == id) activeRevision else progress(id)?.optLong("revision") ?: 0L).put("position", position))
        }
        if (ok) { positionChanged = true; schedule() }; return ok
    }
    fun restore(item: JSONObject): String? {
        val id = add(ChatMessage("user", "", id = item.getJSONObject("anchor").getString("message_id")), item.optString("title"), item.optString("note"), true)
        if (id != null) item.optJSONObject("progress")?.optJSONObject("position")?.let { putPosition(it, id) }
        return id
    }
    fun resolve(id: String, position: JSONObject) {
        if (save { state.put("queue", JSONArray(queue().filter { it.optString("bookmark_id") != id || it.optString("operation_id") == sending })); enqueue(operation("resolve", id).put("position", position).put("base_revision", progress(id)?.optLong("revision") ?: 0L)) }) flush()
    }
    fun query(id: String, resume: Boolean): Map<String, String> {
        val pending = queue().any { it.optString("bookmark_id") == id && it.optString("action") in listOf("create", "progress", "resolve") }
        val p = position(id, resume)
        return if (pending && p != null) mapOf("around" to p.getString("message_id")) else mapOf("bookmark" to id, "resume" to resume.toString())
    }
    private fun request(method: String, path: String, body: JSONObject? = null): JSONObject {
        val builder = AuthManager.applyAuth(context, Request.Builder().url(server + path).header("Cache-Control", "no-cache"))
        if (method == "POST") builder.post(body.toString().toRequestBody("application/json".toMediaType())) else builder.get()
        val call = http.newCall(builder.build()); call.timeout().timeout(12, TimeUnit.SECONDS)
        return call.execute().use { response ->
            val result = JSONObject(response.body?.string().orEmpty().ifEmpty { "{}" })
            if (!response.isSuccessful) throw SocialChatReadError(response.code, result.optString("error", "同步失败"))
            result
        }
    }
    fun load(done: () -> Unit = {}) {
        if (!valid() || denied) return
        SocialChatReadChannel.io.execute {
            val result = runCatching {
                check(request("GET", "/api/me/reading-capabilities").optBoolean("reading_bookmarks")) { "服务器暂未支持书签" }
                val all = mutableListOf<JSONObject>(); var cursor = ""; var page: JSONObject
                do { page = request("GET", "/api/me/reading-bookmarks?kind=${encode(scope.substringBefore(':'))}&id=${encode(scope.substringAfter(':'))}&after=${encode(cursor)}")
                    all.addAll(array(page.optJSONArray("bookmarks"))); cursor = page.optString("next").takeUnless { it == "null" }.orEmpty()
                } while (cursor.isNotEmpty() && all.size < 100)
                page.put("bookmarks", JSONArray(all))
            }
            main.post {
                if (!valid()) return@post
                result.onSuccess { page ->
                    supported = true
                    val saved = save {
                        state.put("supported", true)
                        val remote = array(page.optJSONArray("bookmarks")); val removed = queue().filter { it.optString("action") == "delete" }.map { it.optString("bookmark_id") }
                        val local = bookmarks.filter { row -> queue().any { it.optString("action") == "create" && it.optString("bookmark_id") == row.optString("id") } && remote.none { it.optString("id") == row.optString("id") } }
                        state.put("bookmarks", JSONArray(remote.filter { it.optString("id") !in removed } + local))
                        queue().filter { it.optString("action") == "update" }.forEach { edit -> item(edit.optString("bookmark_id"))?.put("title", edit.optString("title"))?.put("note", edit.optString("note")) }
                        state.put("conversation_progress", page.opt("conversation_progress")).put("conversation_candidates", page.opt("conversation_candidates"))
                    }
                    if (!saved) return@onSuccess
                    if (firstLoad && active == "" && !positionChanged && queue().none { it.optString("bookmark_id").isEmpty() && it.optString("action") in listOf("progress", "resolve") }) {
                        activeRevision = progress("")?.optLong("revision") ?: 0L
                    }
                    firstLoad = false
                    done(); flush()
                }.onFailure(::failed); changed()
            }
        }
    }
    fun flush() {
        main.removeCallbacks(flushTask)
        if (!valid() || busy || !supported || denied) return
        val blockedIds = blocked.map { it.optString("bookmark_id") }.toSet()
        val first = queue().firstOrNull { it.optString("bookmark_id") !in blockedIds } ?: return
        if (!first.optBoolean("attempted") && !save { first.put("attempted", true) }) return
        val op = JSONObject(first.toString()); sending = op.getString("operation_id"); busy = true
        SocialChatReadChannel.io.execute {
            val result = if (session == socialSession(context)) runCatching { request("POST", "/api/me/reading-bookmarks", op) } else Result.failure(IllegalStateException("账号已变化"))
            main.post {
                if (!valid()) return@post
                var again = false
                result.onSuccess { response ->
                    again = save {
                        state.put("queue", JSONArray(queue().filter { it.optString("operation_id") != op.getString("operation_id") }))
                        val id = op.optString("bookmark_id"); val item = item(id)
                        if (response.has("revision")) item?.put("revision", response.getLong("revision"))
                        if (response.has("revision")) { var revision = response.getLong("revision"); queue().filter { it.optString("bookmark_id") == id && it.optString("action") in listOf("update", "delete") }.forEach { it.put("base_revision", revision++) } }
                        if (op.optString("action") == "create") item?.put("local", false)
                        response.optJSONObject("progress")?.let { progress ->
                            if (id.isEmpty()) state.put("conversation_progress", progress).put("conversation_candidates", response.optJSONArray("candidates") ?: JSONArray())
                            else item?.put("progress", progress)?.put("candidates", response.optJSONArray("candidates") ?: JSONArray())
                            if (!response.optBoolean("conflict") && active == id) activeRevision = progress.optLong("revision")
                        }
                    }
                    if (response.optBoolean("conflict")) { error = "阅读位置有冲突，请打开书签选择"; again = false }
                }.onFailure { failure ->
                    if (failure is SocialChatReadError && failure.status in listOf(400, 409, 429)) save { queue().find { it.optString("operation_id") == sending }?.put("blocked", failure.status) }
                    if (failure is SocialChatReadError && failure.status == 404 && op.optString("action") != "create") {
                        val id = op.optString("bookmark_id")
                        save { state.put("queue", JSONArray(queue().filter { it.optString("bookmark_id") != id })); state.put("bookmarks", JSONArray(bookmarks.filter { it.optString("id") != id })) }
                        if (active == id) active = null
                    }
                    failed(failure)
                }
                sending = ""; busy = false; changed(); if (again) flush()
                else if (!denied && pending > 0 && blocked.isEmpty()) main.postDelayed(flushTask, 15000)
            }
        }
    }
    fun retryOperation(id: String, discard: Boolean) {
        val op = queue().find { it.optString("operation_id") == id } ?: return
        if (discard) {
            save {
                state.put("queue", JSONArray(queue().filter { if (op.optString("action") == "create") it.optString("bookmark_id") != op.optString("bookmark_id") else it.optString("operation_id") != id }))
                if (op.optString("action") == "create") state.put("bookmarks", JSONArray(bookmarks.filter { it.optString("id") != op.optString("bookmark_id") }))
            }; load(); return
        }
        load { save {
            op.put("operation_id", UUID.randomUUID().toString()); op.remove("blocked")
            if (op.optString("action") in listOf("update", "delete")) item(op.optString("bookmark_id"))?.let { op.put("base_revision", it.optLong("revision")) }
        } }
    }
    private fun failed(failure: Throwable) {
        if (failure is SocialChatReadError && failure.status in listOf(401, 403)) { denied = true; active = null }
        error = if (denied) "无法访问此会话的书签" else if (failure is SocialChatReadError && failure.status == 409) "书签已变化，请刷新后处理本机修改" else "已保留本机书签，待联网同步"
    }
    fun close() { main.removeCallbacks(flushTask); alive = false; active = null }
    companion object {
        fun array(value: JSONArray?): List<JSONObject> = (0 until (value?.length() ?: 0)).mapNotNull { value?.optJSONObject(it) }
        private fun encode(value: String) = URLEncoder.encode(value, "UTF-8")
    }
}
