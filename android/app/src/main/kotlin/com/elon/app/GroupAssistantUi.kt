package com.elon.app

import android.app.Dialog
import android.graphics.drawable.ColorDrawable
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import org.json.JSONArray
import org.json.JSONObject

internal class GroupAssistantUi(
    private val activity: AppCompatActivity, private val api: GroupAssistantApi,
    private val sync: GroupAssistantSync, private val read: suspend (JSONObject) -> JSONObject,
) {
    private var dialog: Dialog? = null
    private var reader: AiConversationShareReaderView? = null
    private var content: LinearLayout? = null
    private var status: TextView? = null
    private var group: AppGroup? = null
    private var session = ""
    private var job: Job? = null
    private var syncJob: Job? = null
    private var page = "list"
    private var rows = emptyList<JSONObject>()
    private var generation = 0
    private var foreground = true
    val open get() = dialog?.isShowing == true
    private fun dp(n: Int) = (n * activity.resources.displayMetrics.density).toInt()
    private fun valid(epoch: Int) = foreground && open && generation == epoch && session == socialSession(activity)
    private fun text(value: String, size: Float = 15f) = TextView(activity).apply {
        text = value; textSize = size; setTextColor(ContextCompat.getColor(activity, R.color.elon_text_primary))
        setPadding(dp(8), dp(10), dp(8), dp(10))
    }
    private fun button(label: String, id: String, action: () -> Unit) = Button(activity).apply {
        text = label; isAllCaps = false; contentDescription = id; minHeight = dp(48)
        setOnClickListener { if (job?.isActive != true) action() }
    }
    fun show(value: AppGroup) {
        close(); group = value; session = socialSession(activity); foreground = true
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL; setPadding(dp(12), dp(12), dp(12), dp(12))
            setBackgroundColor(ContextCompat.getColor(activity, R.color.elon_bg_app))
        }
        val header = LinearLayout(activity).apply { gravity = Gravity.CENTER_VERTICAL }
        header.addView(button("返回", "group-assistant-back", ::close))
        header.addView(text("群 AI 助手", 20f), LinearLayout.LayoutParams(0, -2, 1f))
        root.addView(header)
        root.addView(text(value.name, 13f))
        val controls = LinearLayout(activity)
        controls.addView(button("分享关注事项", "group-assistant-share") { catalog() }, LinearLayout.LayoutParams(0, -2, 1f))
        controls.addView(button("刷新", "group-assistant-refresh") { load(true) })
        root.addView(controls)
        status = text("正在读取关注事项", 13f).also { root.addView(it); it.accessibilityLiveRegion = 1 }
        content = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL }
        root.addView(ScrollView(activity).apply { addView(content) }, LinearLayout.LayoutParams(-1, 0, 1f))
        dialog = Dialog(activity).apply {
            setContentView(root); setOnDismissListener { generation++; job?.cancel(); syncJob?.cancel(); reader?.dialog?.dismiss(); dialog = null }
            show(); window?.apply { setBackgroundDrawable(ColorDrawable(android.graphics.Color.TRANSPARENT)); setLayout(-1, -1) }
        }
        load(true)
    }
    private fun launch(work: suspend (Int) -> Unit) {
        if (job?.isActive == true) return
        val epoch = generation
        job = activity.lifecycleScope.launch {
            try { work(epoch) }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { if (valid(epoch)) status?.text = "读取或同步失败，已显示内容保留。请刷新重试；登录失效时先打开 ChatGPT 完成登录。" }
        }
    }
    private fun load(refresh: Boolean): Unit = launch { epoch ->
        val g = group ?: return@launch
        val listed = api.call(g.id)
        if (!valid(epoch)) return@launch
        rows = objects(listed.getJSONArray("items")); render()
        if (refresh && syncJob?.isActive != true) syncJob = activity.lifecycleScope.launch {
            try {
                for (row in rows.filter { it.optBoolean("owned") }.take(10)) {
                    if (!valid(epoch)) return@launch
                    sync.one(g.id, row)
                }
                val fresh = api.call(g.id)
                if (valid(epoch)) { rows = objects(fresh.getJSONArray("items")); if (page == "list") render() }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) { if (valid(epoch) && page == "list") status?.text = "同步暂不可用，已显示内容保留" }
        }
    }
    private fun render() {
        page = "list"
        content?.removeAllViews()
        status?.text = if (rows.isEmpty()) "暂无关注事项" else "${rows.size} 个关注事项 · 分享者在线时同步"
        rows.forEach { row ->
            content?.addView(button(row.getString("title"), "group-assistant-item-${row.getString("id")}") { updates(row) })
            content?.addView(text("${row.optString("owner_name")} · ChatGPT · ${stateLabel(row.optString("state"))}\n最近同步：${date(row.optString("synced_at"))}", 13f))
            if (row.optBoolean("owned")) content?.addView(button("取消分享", "group-assistant-revoke-${row.getString("id")}") { revoke(row) })
        }
    }
    private fun catalog(filter: String = "scheduled", cursor: String? = null, previous: List<JSONObject> = emptyList(), expected: String? = null): Unit = launch { epoch ->
        page = "catalog"
        status?.text = "正在读取我的关注事项"
        val result = read(JSONObject().put("operation", "list").put("filter", filter).put("force", false)
            .apply { cursor?.let { put("cursor", it) }; expected?.let { put("expectedScope", it) } })
        if (!valid(epoch)) return@launch
        val scope = result.getString("scope")
        val data = result.getJSONObject("data")
        val items = (previous + objects(data.getJSONArray("items"))).distinctBy { it.getString("id") }
        content?.removeAllViews(); status?.text = "我的关注事项 · 选择后预览"
        content?.addView(button("返回群关注事项", "group-assistant-catalog-back", ::render))
        val filters = LinearLayout(activity)
        listOf("scheduled" to "进行中", "paused" to "已暂停", "finished" to "已完成").forEach { (key,label) ->
            filters.addView(button(label, "group-assistant-filter-$key") { catalog(key) }, LinearLayout.LayoutParams(0, -2, 1f))
        }
        content?.addView(filters)
        if (items.isEmpty()) content?.addView(text("这个分类暂无关注事项"))
        items.forEach { item -> content?.addView(button(item.getString("title"), "group-assistant-task-${item.getString("id")}") { preview(item, scope) }) }
        if (!data.isNull("nextCursor")) content?.addView(button("加载更多", "group-assistant-catalog-more") {
            catalog(filter, data.getString("nextCursor"), items, scope)
        })
    }
    private fun preview(task: JSONObject, scope: String): Unit = launch { epoch ->
        val data = sync.latest(task.getString("id"), scope)
        if (!valid(epoch)) return@launch
        val body = data.optJSONObject("update")?.optString("contentText").orEmpty()
        val consent = "分享范围：该关注事项的标题、最新结果正文和今后的更新。保留正文表格、代码和链接；不含交互图表及附件原件。不分享任务指令、其他会话或登录信息。\n分享者在线且登录有效时同步；离线保留已同步结果。取消分享后，群内结果将撤下，但不能收回别人已保存的副本。"
        AlertDialog.Builder(activity).setTitle(task.getString("title"))
            .setMessage("$consent\n\n${if (body.isBlank()) "当前暂无更新" else body.take(1600)}${if (body.length > 1600) "\n（摘要预览，完整结果可先阅读）" else ""}")
            .setNegativeButton("取消", null)
            .setNeutralButton("阅读完整结果") { _, _ -> if (valid(epoch) && body.isNotBlank()) showText(task.getString("title"), body) }
            .setPositiveButton("分享至本群") { _, _ -> if (valid(epoch)) publish(task, scope) }.show()
        status?.text = if (body.isBlank()) "当前没有更新，仍可订阅今后结果" else "已读取最新结果，请确认分享范围"
    }
    private fun publish(task: JSONObject, scope: String): Unit = launch { epoch ->
        val g = group ?: return@launch
        // Recheck the chosen account before creating a persistent authorization.
        sync.latest(task.getString("id"), scope)
        if (!valid(epoch)) return@launch
        val created = api.call(g.id, body = JSONObject().put("task_id",task.getString("id"))
            .put("account_scope",scope).put("title",task.getString("title")).put("consent",true))
        if (!valid(epoch)) return@launch
        sync.one(g.id, JSONObject().put("id",created.getString("id")).put("task_id",task.getString("id")).put("account_scope",scope))
        val fresh = api.call(g.id)
        if (valid(epoch)) { rows = objects(fresh.getJSONArray("items")); render(); status?.text = "已分享，后续更新将自动同步" }
    }
    private fun revoke(row: JSONObject) {
        AlertDialog.Builder(activity).setTitle("取消分享？").setMessage("停止同步并撤下群内已同步结果。不会删除 ChatGPT 中的任务；别人已保存的内容无法收回。")
            .setNegativeButton("返回",null).setPositiveButton("取消分享") { _, _ -> launch { epoch ->
                val g = group ?: return@launch
                api.call(g.id,row.getString("id"),delete=true)
                if (valid(epoch)) { rows = rows.filterNot { it.getString("id") == row.getString("id") }; render() }
            } }.show()
    }
    private fun updates(row: JSONObject, before: Long = 0, previous: List<JSONObject> = emptyList()): Unit = launch { epoch ->
        page = "updates"
        val g = group ?: return@launch
        val page = api.call(g.id,row.getString("id"),before=before)
        if (!valid(epoch)) return@launch
        val items = previous + objects(page.getJSONArray("items"))
        content?.removeAllViews(); status?.text = row.getString("title") + " · 更新动态"
        content?.addView(button("返回关注事项", "group-assistant-updates-back", ::render))
        if (items.isEmpty()) content?.addView(text("尚无已同步更新"))
        items.forEach { update -> content?.addView(button("${date(update.getString("created_at"))}\n${update.getString("content").take(90)}", "group-assistant-update-${update.getString("id")}") {
            showText(row.getString("title"),update.getString("content"))
        }) }
        if (!page.isNull("next_cursor")) content?.addView(button("较早更新", "group-assistant-updates-more") { updates(row,page.getLong("next_cursor"),items) })
    }
    private fun showText(title: String, body: String) {
        val g = group ?: return
        reader?.dialog?.dismiss()
        val rows = mutableListOf(ChatMessage("assistant", body, id="assistant-result",
            webChatMessage=WebChatProductionMessage("snapshot","assistant-result",emptySet(),renderMarkdown=true)))
        val card = AiConversationShareCard("preview",g.id,title,"","chatgpt","群 AI 助手",1)
        reader = AiConversationShareReaderView(activity,card,null,{ reader = null },null).also {
            it.show(); it.render(AiConversationShareSnapshot(card,rows),rows,ChatAdapter(rows),null)
        }
    }
    fun pause() { foreground = false; generation++; job?.cancel(); syncJob?.cancel(); reader?.dialog?.dismiss() }
    fun resume() { foreground = true; if (open) { if (session != socialSession(activity)) close() else load(false) } }
    fun close() { generation++; job?.cancel(); syncJob?.cancel(); reader?.dialog?.dismiss(); dialog?.dismiss(); dialog = null; rows = emptyList() }
    private fun objects(value: JSONArray) = (0 until value.length()).map { value.getJSONObject(it) }
    private fun date(value: String): String = runCatching {
        java.time.Instant.parse(value).atZone(java.time.ZoneId.systemDefault()).format(java.time.format.DateTimeFormatter.ofPattern("MM-dd HH:mm"))
    }.getOrDefault("尚未同步")
    private fun stateLabel(value: String) = when(value) {
        "ready" -> "已同步"; "no_update" -> "暂无新结果"; "missing" -> "原任务已删除或不可访问"
        "auth_required" -> "分享者需登录原账号"; "requires_action" -> "需分享者处理"; else -> "同步暂不可用"
    }
}
