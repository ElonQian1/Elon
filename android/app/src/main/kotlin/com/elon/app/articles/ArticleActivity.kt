package com.elon.app.articles

import android.os.Bundle
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.activity.OnBackPressedCallback
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R
import com.elon.app.elonColor
import org.json.JSONArray
import org.json.JSONObject
import kotlin.concurrent.thread

internal class ArticleSession {
    var groupId = ""; var mine = false; var article: JSONObject? = null; var dirty = false
    var mode = "library"; var readId = ""; var revision = 1L; var pickCover = false
}

class ArticleActivity : AppCompatActivity() {
    internal lateinit var ui: ArticleUi
    internal lateinit var api: ArticleApi
    internal lateinit var session: ArticleSession
    internal lateinit var status: TextView
    private lateinit var root: LinearLayout
    private lateinit var toolbar: LinearLayout
    private lateinit var content: LinearLayout
    private lateinit var editorViews: ArticleEditorViews
    private var busy = false
    private val imagePicker = registerForActivityResult(ActivityResultContracts.GetContent()) { uri ->
        if (uri != null && session.article != null) work({ uploadArticleImage(applicationContext, uri, api) }) { media ->
            val a = session.article!!; a.getJSONObject("media").put(media.getString("id"), media.getString("data_url"))
            val d = a.getJSONObject("document")
            if (session.pickCover) d.put("cover", media.getString("id")) else d.getJSONArray("blocks").put(JSONObject().put("type", "image").put("media_id", media.getString("id")).put("caption", ""))
            session.dirty = true; editor()
        }
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        ui = ArticleUi(this); api = ArticleApi(applicationContext)
        session = lastCustomNonConfigurationInstance as? ArticleSession ?: ArticleSession().apply {
            groupId = intent.getStringExtra("group_id").orEmpty()
            readId = intent.getStringExtra("article_id").orEmpty(); revision = intent.getLongExtra("revision", 1)
            if (readId.isNotBlank()) mode = "read"
        }
        root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setBackgroundColor(elonColor(R.color.elon_bg_app)) }
        toolbar = ui.row(); status = ui.text("", 13f, true).apply { setPadding(ui.dp(18), ui.dp(4), ui.dp(18), ui.dp(4)); accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE }
        content = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        root.addView(toolbar); root.addView(status); root.addView(ScrollView(this).apply { isFillViewport = true; addView(content) }, LinearLayout.LayoutParams(-1, 0, 1f))
        androidx.core.view.WindowCompat.setDecorFitsSystemWindows(window, false)
        androidx.core.view.ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets ->
            val padding = insets.getInsets(androidx.core.view.WindowInsetsCompat.Type.systemBars() or androidx.core.view.WindowInsetsCompat.Type.ime())
            view.setPadding(padding.left, padding.top, padding.right, padding.bottom); insets
        }
        setContentView(root); editorViews = ArticleEditorViews(this)
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) { override fun handleOnBackPressed() { back() } })
        when (session.mode) { "editor" -> editor(); "preview" -> preview(); "read" -> read(); else -> library() }
    }
    @Deprecated("Retains only in-memory drafts across configuration changes")
    override fun onRetainCustomNonConfigurationInstance(): Any = session
    internal fun notice(message: String) { status.text = message; Toast.makeText(this, message, Toast.LENGTH_SHORT).show() }
    internal fun work(action: () -> JSONObject, done: (JSONObject) -> Unit) {
        if (busy) return
        busy = true; enable(root, false); status.text = "正在处理…"
        thread(name = "article-operation") {
            val result = runCatching(action)
            runOnUiThread {
                if (isDestroyed || isFinishing) return@runOnUiThread
                busy = false; enable(root, true)
                result.fold({ status.text = ""; done(it) }, { notice(it.message ?: "操作失败，请重试；编辑内容仍保留") })
            }
        }
    }
    private fun enable(view: View, value: Boolean) { view.isEnabled = value; if (view is ViewGroup) for (i in 0 until view.childCount) enable(view.getChildAt(i), value) }
    private fun header(title: String, back: () -> Unit) { toolbar.removeAllViews(); toolbar.addView(ui.button("‹ 返回", back)); toolbar.addView(ui.text(title, 18f), LinearLayout.LayoutParams(0, -2, 1f)) }
    internal fun pickImage(cover: Boolean) { session.pickCover = cover; imagePicker.launch("image/*") }
    internal fun saveDraft(): JSONObject {
        val a = session.article ?: error("草稿已关闭")
        if (!session.dirty) return a
        val saved = api.request("/api/me/articles/${a.getString("id")}/draft", "PUT", JSONObject().put("version", a.getLong("revision")).put("document", a.getJSONObject("document")))
        session.article = saved; session.dirty = false; return saved
    }
    internal fun library() {
        session.mode = "library"; content.removeAllViews(); header("文章") { finish() }
        toolbar.addView(ui.button("写文章") { work({ api.request("/api/me/articles", "POST", JSONObject()) }) { session.article = it; session.dirty = false; editor() } })
        content.addView(ui.row().apply {
            addView(ui.button(if (session.mine) "群文章" else "✓ 群文章") { session.mine = false; library() })
            addView(ui.button(if (session.mine) "✓ 我的文章" else "我的文章") { session.mine = true; library() })
        })
        content.addView(ui.button("币安广场 · 发布记录与账号") { startActivity(android.content.Intent(this, com.elon.app.articles.square.SquareActivity::class.java)) })
        val list = ui.column(); content.addView(list); loadPage(list, 0)
    }
    private fun loadPage(list: LinearLayout, offset: Int) {
        val group = if (session.mine) "" else "&group_id=${java.net.URLEncoder.encode(session.groupId, "UTF-8") }"
        work({ api.request("/api/me/articles?offset=$offset$group") }) { page ->
            val rows = page.optJSONArray("items") ?: JSONArray()
            if (offset == 0 && rows.length() == 0) list.addView(ui.text(if (session.mine) "还没有文章，开始写第一篇吧。" else "群里还没有文章，点击“写文章”开始。", 16f, true))
            for (i in 0 until rows.length()) { val card = rows.getJSONObject(i); list.addView(ui.card(card) {
                if (card.optString("status") == "withdrawn") notice("文章已撤下")
                else if (session.mine) work({ api.request("/api/me/articles/${card.getString("id")}/draft") }) { session.article = it; session.dirty = false; editor() }
                else { session.readId = card.getString("id"); session.revision = card.getLong("revision"); read() }
            }) }
            if (!page.isNull("next_offset")) list.addView(ui.button("加载更多") { val button = list.getChildAt(list.childCount - 1); list.removeView(button); loadPage(list, page.getInt("next_offset")) })
        }
    }
    internal fun editor() {
        session.mode = "editor"; content.removeAllViews(); header("编辑文章", ::back)
        toolbar.addView(ui.button("保存") { work(::saveDraft) { notice("草稿已保存") } }); toolbar.addView(ui.button("预览") { preview() })
        status.text = if (session.dirty) "尚未保存，请保存草稿" else "草稿已保存"
        content.addView(editorViews.render())
    }
    internal fun preview() {
        session.mode = "preview"; content.removeAllViews(); header("预览文章") { editor() }
        toolbar.addView(ui.button("发布到群") { editorViews.chooseGroups() }); content.addView(ui.body(session.article!!))
        content.addView(ui.button("发布到币安广场") { work(::saveDraft) { saved -> startActivity(android.content.Intent(this, com.elon.app.articles.square.SquareActivity::class.java).putExtra("article_id", saved.getString("id"))) } })
    }
    private fun read() {
        session.mode = "read"; content.removeAllViews(); header("文章", ::back)
        val retry = ui.button("重新读取") { read() }; toolbar.addView(retry)
        work({ api.read(session.readId, session.revision) }) { content.addView(ui.body(it)); toolbar.removeView(retry) }
    }
    private fun back() {
        if (busy) { notice("正在保存或读取，请稍候"); return }
        when (session.mode) {
            "preview" -> editor()
            "editor" -> if (session.dirty) AlertDialog.Builder(this).setTitle("还有未保存的修改").setMessage("可以保存后返回，或继续编辑。")
                .setNegativeButton("继续编辑", null).setNeutralButton("放弃修改") { _, _ -> session.article = null; session.dirty = false; library() }
                .setPositiveButton("保存并返回") { _, _ -> work(::saveDraft) { session.article = null; library() } }.show()
                else { session.article = null; library() }
            "read" -> if (session.groupId.isBlank()) finish() else library()
            else -> finish()
        }
    }
}
