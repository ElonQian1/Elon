package com.elon.app.articles.square

import android.widget.EditText
import android.widget.LinearLayout
import androidx.appcompat.app.AlertDialog
import org.json.JSONObject
import java.text.DateFormat
import java.util.Date

internal val squareLabels = mapOf("queued" to "等待发布", "preparing" to "准备媒体", "submitting" to "提交中", "published" to "已发布", "failed" to "发布失败", "uncertain" to "结果待核实", "cancelled" to "已取消")
internal val squareModes = linkedMapOf("article" to "长文章", "text" to "文字动态", "images" to "图片动态", "video" to "视频动态")
internal fun squareTime(epoch: Long): String = DateFormat.getDateTimeInstance(DateFormat.SHORT, DateFormat.SHORT).format(Date(epoch * 1000))

internal class SquareHistory(private val a: SquareActivity) {
    private lateinit var list: LinearLayout
    fun render() {
        a.content.addView(a.ui.text("关闭页面不影响定时任务。一龙撤下文章不会删除币安帖子。", 14f, true))
        a.content.addView(a.button("在币安管理已有内容 ↗") { a.browse(SquareApi.CREATOR) })
        a.content.addView(a.button("刷新记录") { load() }); list = a.ui.column(); a.content.addView(list); load()
    }
    private fun load(offset: Int = 0) { a.work({ a.api.request("/jobs?offset=$offset") }) { page ->
        if (offset == 0) list.removeAllViews()
        val rows = page.getJSONArray("items")
        if (offset == 0 && rows.length() == 0) list.addView(a.ui.text("还没有发布记录。从文章预览页选择币安广场开始。", 16f, true))
        for (i in 0 until rows.length()) add(rows.getJSONObject(i))
        if (!page.isNull("next_offset")) list.addView(a.button("加载更多") { val last = list.getChildAt(list.childCount - 1); list.removeView(last); load(page.getInt("next_offset")) })
    } }
    private fun add(job: JSONObject) {
        val box = a.ui.column(); val status = job.getString("status"); val id = job.getString("id")
        box.addView(a.ui.text(job.optString("title"), 20f)); box.addView(a.ui.text("${squareModes[job.optString("mode")]} · v${job.optLong("revision")} · ${squareLabels[status] ?: status}", 14f, true))
        box.addView(a.ui.text("计划 ${squareTime(job.optLong("scheduled_at"))} · 尝试${job.optInt("attempts")}次\n${job.optString("message")}", 14f, true))
        job.optString("post_url").takeIf { it.matches(Regex("https://www\\.binance\\.com/en/square/post/[0-9]+")) }?.let { url -> box.addView(a.button("打开币安帖子 ↗") { a.browse(url) }) }
        fun act(verb: String, body: JSONObject = JSONObject()) { a.work({ a.api.request("/jobs/$id/$verb", "POST", body) }) { load() } }
        if (status in listOf("queued", "preparing", "failed")) box.addView(a.button("取消任务") { act("cancel") })
        if (status in listOf("failed", "cancelled")) box.addView(a.button("重试此版本") { a.confirm("将按记录保存的文章版本重新发布，确定继续？") { act("retry") } })
        if (status == "uncertain") {
            box.addView(a.button("记录已发布的帖子") {
                val input = EditText(a).apply { hint = "帖子链接或数字ID"; minHeight = a.ui.dp(48) }
                val dialog = AlertDialog.Builder(a).setTitle("先到币安核实，避免重复发帖").setView(input).setNegativeButton("取消", null).setPositiveButton("记录已发布", null).create()
                dialog.setOnShowListener { dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener { runCatching { SquareApi.postId(input.text.toString()) }.fold({ post -> dialog.dismiss(); act("resolve", JSONObject().put("post_id", post)) }, { input.error = it.message }) } }; dialog.show()
            })
            box.addView(a.button("我已核实未发布") { a.confirm("我已在币安核实，此内容没有发布；确认后可重试。") { act("resolve", JSONObject().put("not_published", true)) } })
        }
        list.addView(box)
    }
}
