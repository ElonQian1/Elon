package com.elon.app.articles

import android.text.InputFilter
import android.text.InputType
import android.view.View
import android.widget.EditText
import android.widget.LinearLayout
import androidx.appcompat.app.AlertDialog
import androidx.core.widget.doAfterTextChanged
import org.json.JSONArray
import org.json.JSONObject

internal class ArticleEditorViews(private val host: ArticleActivity) {
    private val ui get() = host.ui
    private val state get() = host.session
    private val article get() = state.article!!
    private val document get() = article.getJSONObject("document")
    private val blocks get() = document.getJSONArray("blocks")
    private fun changed() { state.dirty = true; host.status.text = "尚未保存，请保存草稿" }
    private fun field(label: String, value: String, lines: Int, max: Int, change: (String) -> Unit): View = ui.column().apply {
        setPadding(0, ui.dp(6), 0, ui.dp(6)); addView(ui.text(label, 14f, true))
        addView(EditText(host).apply {
            setText(value); textSize = 17f; minLines = lines; gravity = android.view.Gravity.TOP
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES or if (lines > 1) InputType.TYPE_TEXT_FLAG_MULTI_LINE else 0
            filters = arrayOf(InputFilter.LengthFilter(max)); setSelectAllOnFocus(false)
            doAfterTextChanged { change(it.toString()); changed() }
        })
    }
    fun render(): View = ui.column().apply {
        addView(field("标题", document.optString("title"), 1, 120) { document.put("title", it) })
        addView(field("摘要（选填，显示在群聊卡片）", document.optString("summary"), 2, 300) { document.put("summary", it) })
        addView(ui.button("选择封面") { host.pickImage(true) })
        article.optJSONObject("media")?.optString(document.optString("cover"))?.takeIf { it.isNotBlank() }?.let { addView(ui.image(it, true)); addView(ui.button("移除封面") { document.put("cover", JSONObject.NULL); changed(); host.editor() }) }
        addView(ui.text("图文正文", 20f))
        for (i in 0 until blocks.length()) addView(renderBlock(i))
        addView(ui.button("＋ 添加段落") {
            if (blocks.length() >= 120) { host.notice("正文最多120个段落"); return@button }
            blocks.put(JSONObject().put("type", "paragraph").put("text", "")); changed(); host.editor()
        })
        addView(ui.button("＋ 插入图片") { if (blocks.length() < 120) host.pickImage(false) else host.notice("正文最多120个段落") })
        addView(ui.text("修改后需要保存；再次发布将发送新版本，旧卡片保留原文。", 13f, true))
        addView(ui.button("撤下文章") {
            AlertDialog.Builder(host).setTitle("撤下文章？").setMessage("所有群中的文章将无法打开，撤下后不能再次发布。")
                .setNegativeButton("取消", null).setPositiveButton("撤下") { _, _ -> host.work({
                    val saved = host.saveDraft(); host.api.request("/api/me/articles/${saved.getString("id")}/withdraw", "POST", JSONObject().put("version", saved.getLong("revision")))
                }) { state.article = null; state.dirty = false; host.library() } }.show()
        })
    }
    private fun renderBlock(i: Int): View = ui.column().apply {
        val b = blocks.getJSONObject(i)
        addView(ui.row().apply {
            addView(ui.text("第${i + 1}段", 13f, true), LinearLayout.LayoutParams(0, -2, 1f))
            if (i > 0) addView(ui.button("上移") { val previous = blocks.getJSONObject(i - 1); blocks.put(i - 1, b); blocks.put(i, previous); changed(); host.editor() })
            addView(ui.button("删除") { blocks.remove(i); changed(); host.editor() })
        })
        if (b.optString("type") == "image") {
            addView(ui.image(article.getJSONObject("media").optString(b.optString("media_id"))))
            addView(field("图片说明", b.optString("caption"), 1, 300) { b.put("caption", it) })
        } else {
            val keys = arrayOf("paragraph", "heading", "quote"); val names = arrayOf("正文", "小标题", "引用")
            addView(ui.button(names[keys.indexOf(b.optString("type")).coerceAtLeast(0)]) {
                AlertDialog.Builder(host).setTitle("段落类型").setItems(names) { _, index -> b.put("type", keys[index]); changed(); host.editor() }.show()
            })
            addView(field("内容", b.optString("text"), 4, 50000) { b.put("text", it) })
        }
    }
    fun chooseGroups() {
        host.work({ host.api.request("/api/me/groups") }) { result ->
            val groups = result.optJSONArray("groups") ?: JSONArray()
            if (groups.length() == 0) { host.notice("你还没有可发布的群聊"); return@work }
            val selected = BooleanArray(groups.length()) { groups.getJSONObject(it).optString("id") == state.groupId }
            val names = Array(groups.length()) { groups.getJSONObject(it).optString("name") }
            val dialog = AlertDialog.Builder(host).setTitle("发布到群 · 仅群成员可读").setMultiChoiceItems(names, selected) { _, index, checked -> selected[index] = checked }
                .setNegativeButton("取消", null).setPositiveButton("确认发布", null).create()
            dialog.setOnShowListener { dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val ids = JSONArray(); selected.forEachIndexed { i, v -> if (v) ids.put(groups.getJSONObject(i).getString("id")) }
                if (ids.length() !in 1..10) { host.notice("请选择1至10个群聊"); return@setOnClickListener }
                dialog.dismiss(); host.work({ val saved = host.saveDraft(); host.api.request("/api/me/articles/${saved.getString("id")}/publish", "POST", JSONObject().put("version", saved.getLong("revision")).put("group_ids", ids)) }) {
                    state.article?.put("status", "published"); host.notice(if (it.optJSONArray("messages")?.length() == 0) "这些群已收到此版本，没有重复发送" else "文章已发布到群聊"); host.preview()
                }
            } }; dialog.show()
        }
    }
}
