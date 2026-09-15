package com.elon.app.articles.square

import android.app.DatePickerDialog
import android.app.TimePickerDialog
import android.widget.CheckBox
import androidx.appcompat.app.AlertDialog
import org.json.JSONArray
import org.json.JSONObject
import java.util.Calendar
import java.util.UUID

internal class SquareComposer(private val a: SquareActivity) {
    private var mode = "article"
    private var cover: String? = null
    private val images = mutableListOf<String>()
    private var media = JSONObject()
    internal var video: JSONObject? = null
    private var preview: JSONObject? = null
    private var requestKey = UUID.randomUUID().toString()
    private var schedule: Long? = null
    private var converted = false
    private var publicOk = false
    fun init() { media = a.article?.optJSONObject("media") ?: JSONObject(); cover = a.article?.optJSONObject("document")?.optString("cover")?.takeIf { it.isNotBlank() && it != "null" } }
    fun invalidate() { preview = null; converted = false; publicOk = false; requestKey = UUID.randomUUID().toString() }
    fun addImage(value: JSONObject) {
        val id = value.getString("id"); media.put(id, value.getString("data_url"))
        if (mode == "article") cover = id else if (id !in images && images.size < 4) images.add(id)
        invalidate(); a.show("publish")
    }
    private fun choose(title: String, values: List<String>, labels: List<String>, done: (String) -> Unit) {
        AlertDialog.Builder(a).setTitle(title).setItems(labels.toTypedArray()) { _, i -> done(values[i]); invalidate(); a.show("publish") }.setNegativeButton("取消", null).show()
    }
    private fun selection(): JSONObject = JSONObject().put("article_id", a.article!!.getString("id")).put("version", a.article!!.getLong("revision")).put("mode", mode)
        .put("media_ids", JSONArray(if (mode == "images") images else emptyList<String>()))
        .put("cover_id", if (mode == "article") cover ?: JSONObject.NULL else JSONObject.NULL)
        .put("video_id", if (mode == "video") video?.getString("id") ?: JSONObject.NULL else JSONObject.NULL)
    fun render() {
        if (a.account?.optBoolean("bound") != true) { a.content.addView(a.ui.text("请先绑定币安广场账号")); a.content.addView(a.button("绑定账号") { a.show("account") }); return }
        val article = a.article ?: run { a.content.addView(a.ui.text("请从自己文章的预览页进入")); return }
        a.content.addView(a.ui.text(article.getJSONObject("document").optString("title"), 22f))
        a.content.addView(a.ui.text("使用已保存文章 v${article.getLong("revision")}。群聊中的文章权限保持原样。", 14f, true))
        a.content.addView(a.button("发布形式：${squareModes[mode]} ▾") { choose("发布形式", squareModes.keys.toList(), squareModes.values.toList()) { mode = it } })
        val ids = media.keys().asSequence().toList()
        when (mode) {
            "article" -> {
                a.content.addView(a.button("选择文章封面 ▾") { choose("文章封面", listOf("") + ids, listOf("不使用封面") + ids.indices.map { "图片 ${it + 1}" }) { cover = it.ifBlank { null } } })
                cover?.let { if (media.has(it)) a.content.addView(a.ui.image(media.getString(it), true)) }
            }
            "images" -> {
                a.content.addView(a.ui.text("已选择 ${images.size}/4 张，按选择顺序展示", 14f, true))
                ids.forEachIndexed { i, id ->
                    a.content.addView(a.ui.image(media.getString(id), true))
                    a.content.addView(CheckBox(a).apply {
                        text = "图片 ${i + 1}" + if (id in images) " · 第${images.indexOf(id) + 1}张" else ""; minHeight = a.ui.dp(48); isChecked = id in images; isEnabled = isChecked || images.size < 4
                        setOnCheckedChangeListener { _, checked -> if (checked) images.add(id) else images.remove(id); invalidate(); a.show("publish") }
                    })
                }
            }
            "video" -> {
                a.content.addView(a.ui.text("MP4/WebM，最多32MB、10分钟，自动使用首帧封面", 14f, true))
                a.content.addView(a.button("选择视频") { a.pick(true) }); video?.optString("cover")?.takeIf { it.startsWith("data:image/") }?.let { a.content.addView(a.ui.image(it, true)) }
            }
        }
        if (mode in listOf("article", "images")) a.content.addView(a.button("添加图片") { a.pick(false) })
        a.content.addView(a.button("生成币安版本预览") { val body = selection(); a.work({ a.api.request("/preview", "POST", body) }) { preview = it; publicOk = false; converted = false; a.show("publish") } })
        preview?.let { preview(it) }
    }
    private fun preview(value: JSONObject) {
        a.content.addView(a.ui.text("发布到：${value.getString("account_label")}", 22f))
        if (mode == "article") a.content.addView(a.ui.text(value.getString("title"), 22f))
        val images = value.getJSONObject("media"); val selection = value.getJSONObject("selection")
        val order = if (selection.getString("mode") == "images") selection.getJSONArray("media_ids").let { ids -> (0 until ids.length()).map { ids.getString(it) } } else images.keys().asSequence().toList()
        order.forEach { a.content.addView(a.ui.image(images.getString(it), true)) }
        a.content.addView(a.ui.text(value.getString("text"), 17f).apply { setTextIsSelectable(true) })
        val warnings = value.getJSONArray("warnings"); for (i in 0 until warnings.length()) a.content.addView(a.ui.text(warnings.getString(i), 14f, true))
        a.content.addView(a.button(schedule?.let { "发送时间：${squareTime(it)}" } ?: "发送时间：立即发送") { chooseTime() })
        if (schedule != null) a.content.addView(a.button("改为立即发送") { schedule = null; a.show("publish") })
        val send = a.button(if (schedule == null) "确认公开发布" else "确认定时发布") {
            val at = schedule
            if (at != null && at <= System.currentTimeMillis() / 1000) { a.notice("请选择未来的发送时间"); return@button }
            val body = JSONObject().put("selection", value.getJSONObject("selection")).put("preview_hash", value.getString("preview_hash")).put("request_key", requestKey)
                .put("public_confirmed", publicOk).put("conversion_confirmed", converted).put("scheduled_at", at ?: JSONObject.NULL)
            a.work({ a.api.request("/jobs", "POST", body) }) { a.show("history"); a.notice("任务已记录 · ${squareLabels[it.optString("status")] ?: it.optString("status")}") }
        }.apply { isEnabled = converted && publicOk }
        a.content.addView(CheckBox(a).apply { text = "已核对文字转换和选定媒体"; minHeight = a.ui.dp(48); isChecked = converted; setOnCheckedChangeListener { _, checked -> converted = checked; send.isEnabled = converted && publicOk } })
        a.content.addView(CheckBox(a).apply { text = "将此内容公开发布到上述币安账号"; minHeight = a.ui.dp(48); isChecked = publicOk; setOnCheckedChangeListener { _, checked -> publicOk = checked; send.isEnabled = converted && publicOk } })
        a.content.addView(a.ui.text("一龙撤下文章不会删除币安帖子。结果不确定时，先到发布记录核实。", 14f, true)); a.content.addView(send)
    }
    private fun chooseTime() {
        val c = Calendar.getInstance().apply { timeInMillis = (schedule ?: (System.currentTimeMillis() / 1000 + 3600)) * 1000 }
        DatePickerDialog(a, { _, year, month, day ->
            TimePickerDialog(a, { _, hour, minute ->
                c.set(year, month, day, hour, minute, 0); c.set(Calendar.MILLISECOND, 0); schedule = c.timeInMillis / 1000; a.show("publish")
            }, c.get(Calendar.HOUR_OF_DAY), c.get(Calendar.MINUTE), true).show()
        }, c.get(Calendar.YEAR), c.get(Calendar.MONTH), c.get(Calendar.DAY_OF_MONTH)).apply { datePicker.minDate = System.currentTimeMillis(); datePicker.maxDate = System.currentTimeMillis() + 30L * 86400 * 1000 }.show()
    }
}
