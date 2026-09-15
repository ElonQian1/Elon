package com.elon.app.sharing

import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.text.Editable
import android.text.TextWatcher
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.ViewModelProvider
import com.elon.app.AuthManager
import com.elon.app.LoginActivity
import com.elon.app.MainActivity
import com.elon.app.articles.ArticleUi

class ExternalShareActivity : AppCompatActivity() {
    private lateinit var model: ExternalShareModel
    private lateinit var ui: ArticleUi
    private lateinit var content: LinearLayout
    private var directoryRequested = false
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        model = ViewModelProvider(this)[ExternalShareModel::class.java]; ui = ArticleUi(this)
        content = ui.column(); installShareWindow(ScrollView(this).apply { addView(content) })
        model.changed.observe(this) { render() }
        model.start(intent, savedInstanceState?.getString("draft_id") ?: intent.getStringExtra("draft_id"))
    }
    override fun onResume() { super.onResume(); if (::model.isInitialized) render() }
    override fun onSaveInstanceState(outState: Bundle) { model.draft?.let { outState.putString("draft_id", it.id); model.store.save(it) }; super.onSaveInstanceState(outState) }
    override fun onStop() { model.draft?.let { model.store.save(it) }; super.onStop() }
    private fun render() {
        content.removeAllViews(); content.addView(ui.text("发送到一龙", 22f))
        content.addView(ui.button("取消") { if (!model.busy) { model.draft?.let(model.store::remove); finish() } })
        val d = model.draft
        if (model.notice.isNotBlank()) content.addView(ui.text(model.notice))
        if (model.busy) { content.addView(ui.text(if (d == null) "正在读取分享内容…" else "正在处理，请稍候…")); return }
        if (d == null) return
        if (d.state in setOf("sent", "uncertain", "sending")) {
            content.addView(ui.text(if (d.state == "sent") "已发送到 ${d.target}" else "发送结果未确认，请打开聊天核对。此页面不会自动重发。"))
            content.addView(ui.button("打开一龙聊天") { startActivity(Intent(this, MainActivity::class.java)); finish() }); return
        }
        val input = EditText(this).apply { hint = "分享文字或文章链接"; setText(d.text); minLines = 2; maxLines = 6; contentDescription = "分享内容" }
        input.addTextChangedListener(watcher { d.text = input.text.toString() }); content.addView(input)
        d.files.forEachIndexed { index, attachment ->
            content.addView(ui.text(attachment.displayName, 15f))
            if (attachment.mimeType.startsWith("image/")) {
                val image = android.widget.ImageView(this).apply { adjustViewBounds = true; maxHeight = ui.dp(180); contentDescription = "待发送图片 ${index + 1}" }
                content.addView(image, LinearLayout.LayoutParams(-1, ui.dp(160)))
                com.elon.app.ChatImagePreviewLoader.load(this, attachment.file.path) { bitmap -> image.post { image.setImageBitmap(bitmap) } }
                val links = model.candidates[index].orEmpty()
                if (links.isNotEmpty()) {
                    content.addView(ui.button(attachment.sourceLink?.let { "${it.label} · 已附加（点击更改）" } ?: "选择图片中的链接（${links.size} 个）") {
                        AlertDialog.Builder(this).setTitle("附加到图片的链接").setItems((links.map { it.url } + "不附加链接").toTypedArray()) { _, selected ->
                            attachment.sourceLink = links.getOrNull(selected); model.store.save(d); render()
                        }.show()
                    })
                }
            }
        }
        SourceLink.fromText(d.text)?.let { link -> content.addView(ui.button("预览 · ${link.label}") { ArticleLinkActivity.open(this, link.url) }) }
        if (!AuthManager.isLoggedIn(this)) {
            content.addView(ui.text("登录后继续选择好友或群聊，分享内容已暂存。"))
            content.addView(ui.button("登录后继续") { model.store.save(d); startActivity(Intent(this, LoginActivity::class.java).putExtra("external_share_draft", d.id)) }); return
        }
        if (d.owner.isNotBlank() && d.owner != AuthManager.userId(this)) { content.addView(ui.text("账号已变化，请重新分享，防止发到错误账号。")); return }
        content.addView(ui.text("选择好友或群聊", 18f))
        val search = EditText(this).apply { hint = "搜索好友、群聊"; setSingleLine(); contentDescription = "搜索分享接收方" }; content.addView(search)
        val list = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }; content.addView(list)
        fun results() {
            list.removeAllViews(); val query = search.text.toString().trim()
            val matches = model.targets.filter { it.name.contains(query, true) }
            matches.forEach { target -> list.addView(ui.button("${if (target.kind == "group") "群聊" else "好友"} · ${target.name}") {
                val text = d.text.trim()
                if (text.codePointCount(0, text.length) > 4000 || (text.isBlank() && d.files.isEmpty())) { model.notice = "请填写不超过 4000 字的内容"; render(); return@button }
                AlertDialog.Builder(this).setTitle("发送给 ${target.name}？").setMessage("${d.files.size} 个附件" + if (text.isNotBlank()) "\n${text.take(160)}" else "")
                    .setNegativeButton("取消", null).setPositiveButton("发送") { _, _ -> model.send(target, text) }.show()
            }) }
            if (matches.isEmpty()) list.addView(ui.text("没有匹配的会话"))
        }
        search.addTextChangedListener(watcher(::results)); results()
        content.addView(ui.button("刷新会话") { model.loadTargets() })
        if (!directoryRequested) { directoryRequested = true; model.loadTargets() }
    }
    private fun watcher(action: () -> Unit) = object : TextWatcher {
        override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
        override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) { action() }
        override fun afterTextChanged(s: Editable?) {}
    }
    companion object {
        fun shareText(context: Context, text: String) { context.startActivity(Intent(context, ExternalShareActivity::class.java).setAction(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT, text)) }
    }
}
