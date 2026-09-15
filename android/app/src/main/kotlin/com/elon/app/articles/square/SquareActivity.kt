package com.elon.app.articles.square

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.text.InputType
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.activity.OnBackPressedCallback
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.R
import com.elon.app.elonColor
import com.elon.app.articles.ArticleApi
import com.elon.app.articles.ArticleUi
import org.json.JSONObject
import kotlin.concurrent.thread

class SquareActivity : AppCompatActivity() {
    internal lateinit var ui: ArticleUi
    internal lateinit var api: SquareApi
    internal lateinit var content: LinearLayout
    internal var account: JSONObject? = null
    internal var article: JSONObject? = null
    internal var busy = false
    internal lateinit var composer: SquareComposer
    private lateinit var status: TextView
    private lateinit var root: LinearLayout
    private var tab = "history"
    private val imagePicker = registerForActivityResult(ActivityResultContracts.GetContent()) { uri -> if (uri != null) work({ squareImage(applicationContext, uri, api) }) { composer.addImage(it) } }
    private val videoPicker = registerForActivityResult(ActivityResultContracts.GetContent()) { uri -> if (uri != null) work({ squareVideo(applicationContext, uri, api) }) { composer.video = it; composer.invalidate(); show("publish") } }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        ui = ArticleUi(this); api = SquareApi(applicationContext); composer = SquareComposer(this)
        root = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setBackgroundColor(elonColor(R.color.elon_bg_app)) }
        root.addView(ui.row().apply { addView(button("‹ 返回") { if (!busy) finish() }); addView(ui.text("币安广场", 20f)) })
        status = ui.text("", 14f, true).apply { accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE; setPadding(ui.dp(18), 0, ui.dp(18), 0) }; root.addView(status)
        root.addView(ui.row().apply {
            if (!intent.getStringExtra("article_id").isNullOrBlank()) addView(button("发布内容") { show("publish") })
            addView(button("发布记录") { show("history") }); addView(button("绑定账号") { show("account") })
        })
        content = ui.column(); root.addView(ScrollView(this).apply { addView(content) }, LinearLayout.LayoutParams(-1, 0, 1f))
        androidx.core.view.WindowCompat.setDecorFitsSystemWindows(window, false)
        androidx.core.view.ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets -> val p = insets.getInsets(androidx.core.view.WindowInsetsCompat.Type.systemBars() or androidx.core.view.WindowInsetsCompat.Type.ime()); view.setPadding(p.left, p.top, p.right, p.bottom); insets }
        setContentView(root)
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) { override fun handleOnBackPressed() { if (!busy) finish() else status.text = "正在处理，请稍候；发布结果可在记录里核实" } })
        load()
    }
    private fun load() {
        content.removeAllViews(); content.addView(button("重试连接") { load() })
        work({
            val result = JSONObject().put("account", api.request("/account"))
            intent.getStringExtra("article_id")?.takeIf { it.matches(Regex("article_[\\w-]+")) }?.let { result.put("article", ArticleApi(applicationContext).request("/api/me/articles/$it/draft")) }
            result
        }) { account = it.getJSONObject("account"); article = it.optJSONObject("article"); composer.init(); show(if (!account!!.optBoolean("bound")) "account" else if (article != null) "publish" else "history") }
    }
    internal fun show(value: String) {
        if (busy) return
        tab = value; content.removeAllViews()
        if (account == null) { content.addView(button("重试连接") { load() }); return }
        when (tab) { "account" -> SquareAccount(this).render(); "publish" -> composer.render(); else -> SquareHistory(this).render() }
    }
    internal fun button(label: String, action: () -> Unit) = ui.button(label) { if (!busy) action() }.apply { minHeight = ui.dp(48) }
    internal fun field(label: String, value: String = "", secret: Boolean = false): EditText {
        content.addView(ui.text(label, 14f, true))
        return EditText(this).apply {
            setText(value); textSize = 16f; minHeight = ui.dp(48); setSingleLine(true)
            inputType = InputType.TYPE_CLASS_TEXT or if (secret) InputType.TYPE_TEXT_VARIATION_PASSWORD else InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO; setTextColor(elonColor(R.color.elon_text_primary))
            content.addView(this)
        }
    }
    internal fun confirm(message: String, action: () -> Unit) { AlertDialog.Builder(this).setTitle("请确认").setMessage(message).setNegativeButton("取消", null).setPositiveButton("确认") { _, _ -> action() }.show() }
    internal fun browse(url: String) { runCatching { startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url))) }.onFailure { notice("没有可打开网页的应用") } }
    internal fun notice(message: String) { status.text = message }
    internal fun pick(video: Boolean) { if (video) videoPicker.launch("video/*") else imagePicker.launch("image/*") }
    internal fun work(action: () -> JSONObject, done: (JSONObject) -> Unit) {
        if (busy) return
        busy = true; status.text = "正在处理…"; val controls = mutableListOf<Pair<View, Boolean>>()
        fun disable(view: View) { controls.add(view to view.isEnabled); view.isEnabled = false; if (view is ViewGroup) for (i in 0 until view.childCount) disable(view.getChildAt(i)) }
        disable(root)
        thread(name = "square-operation") { val result = runCatching(action); runOnUiThread {
            if (isFinishing || isDestroyed) return@runOnUiThread
            busy = false; controls.forEach { (view, enabled) -> view.isEnabled = enabled }
            result.fold({ status.text = ""; done(it) }, { notice(it.message ?: "安全连接失败，请刷新发布记录核实") })
        } }
    }
}
