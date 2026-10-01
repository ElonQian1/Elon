package com.elon.app.grid.share

import android.content.Intent
import android.widget.ScrollView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.elon.app.AiConversationShareTarget
import com.elon.app.AiConversationShareTargetPicker
import com.elon.app.AuthManager
import com.elon.app.ChatMessage
import com.elon.app.socialSession
import com.elon.app.grid.host.BinanceAccountActivity
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import okhttp3.OkHttpClient
import org.json.JSONObject

internal class GridShareFeature(private val activity: AppCompatActivity, http: OkHttpClient, private val server: String,
    private val group: () -> String?, private val published: () -> Unit,
    private val quote: (ChatMessage) -> Unit, private val analyze: (ChatMessage) -> Unit) {
    private val api = GridShareApi(activity, http, server)
    private val read = GridShareRead(activity)
    private val picker = AiConversationShareTargetPicker(activity, http, server)
    private val bindings = activity.getSharedPreferences("grid_share_bindings", 0)
    private var dialog: AlertDialog? = null
    private var preview: GridSharePreview? = null
    private var job: Job? = null
    private var epoch = 0
    private var attempted = false
    init { current = this }
    fun available() = group() != null && AuthManager.isLoggedIn(activity)
    private fun bindingKey(target: String, id: String) = "$server|${AuthManager.userId(activity)}|$target|$id"
    private fun active(run: Int, session: String) = epoch == run && socialSession(activity) == session && !activity.isFinishing && !activity.isDestroyed
    fun open() { if (available()) picker.show { target -> load(target, null) } }
    private fun load(target: AiConversationShareTarget, previous: String?) {
        close(); attempted = false; val run = epoch; val session = socialSession(activity)
        dialog = AlertDialog.Builder(activity).setTitle("读取当前币安网格").setMessage("正在读取手机当前账户…")
            .setNegativeButton("取消") { _, _ -> close() }.create().also { it.setOnCancelListener { close() }; it.show() }
        job = activity.lifecycleScope.launch {
            try {
                val (rows, source) = read.list()
                if (!active(run, session)) return@launch
                dialog?.dismiss()
                if (rows.isEmpty()) {
                    dialog = AlertDialog.Builder(activity).setTitle("当前账户没有运行中的网格")
                        .setMessage("可以保留当前账户，也可以在币安账户页核对。")
                        .setPositiveButton("打开币安账户") { _, _ -> activity.startActivity(Intent(activity, BinanceAccountActivity::class.java)) }
                        .setNegativeButton("返回", null).show(); return@launch
                }
                if (previous != null) {
                    val binding = bindings.getString(bindingKey(target.id, previous), null)?.let(::JSONObject)
                        ?: error("请在原分享设备更新；此设备没有原网格账户绑定")
                    check(binding.optString("account") == source.account && binding.optString("kind") == source.kind) { "请使用原分享的币安账户更新" }
                    val id = binding.getString("strategy")
                    check(rows.any { it["id"] == id }) { "原网格已不在运行列表，无法更新" }
                    detail(target, previous, id, source, run, session)
                } else {
                    dialog = GridSharePicker.show(activity, rows) { row ->
                        job = activity.lifecycleScope.launch { detail(target, null, row["id"]!!, source, run, session) }
                    }
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if (active(run, session)) { dialog?.dismiss(); toast(failure.message ?: "读取失败，请重试") } }
        }
    }
    private suspend fun detail(target: AiConversationShareTarget, previous: String?, id: String,
        source: com.elon.app.grid.chat.BinanceGridReadStore.Context, run: Int, session: String) {
        if (!active(run, session)) return
        dialog?.dismiss()
        dialog = AlertDialog.Builder(activity).setTitle("读取网格详情")
            .setMessage("正在读取参数与持仓，请稍候…")
            .setNegativeButton("取消") { _, _ -> close() }.create().also { it.setOnCancelListener { close() }; it.show() }
        try {
            val snapshot = read.detail(id, source)
            if (!active(run, session)) return
            dialog?.dismiss(); dialog = null
            preview = GridSharePreview(activity, snapshot, target.name, previous, submit = { grid ->
                if (active(run, session)) publish(target, snapshot, grid, run, session)
            }, close = { preview = null })
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (failure: Exception) { if (active(run, session)) { dialog?.dismiss(); dialog = null; toast(failure.message ?: "详情读取失败") } }
    }
    private fun publish(target: AiConversationShareTarget, snapshot: GridShareRead.Snapshot, grid: JSONObject, run: Int, session: String) {
        if (read.context() != snapshot.source || !attempted && System.currentTimeMillis() - snapshot.observed > 300_000) {
            preview?.failed("币安来源已变化或快照超过五分钟，请取消并重新读取"); return
        }
        preview?.progress()
        attempted = true
        job = activity.lifecycleScope.launch {
            try {
                val result = withContext(Dispatchers.IO) { api.publish(target.id, grid, session) }
                if (!active(run, session)) return@launch
                val id = result.getString("snapshot_id")
                val card = GridShareModel.card(result.getJSONObject("message").getString("content"))
                    ?: error("发送回执不匹配，请重试确认")
                check(card.optString("snapshot_id") == id && card.optString("group_id") == target.id) { "发送回执不匹配，请重试确认" }
                bindings.edit().putString(bindingKey(target.id, id), JSONObject().put("strategy", snapshot.fields["id"])
                    .put("account", snapshot.source.account).put("kind", snapshot.source.kind).toString()).apply()
                preview?.close(); published(); toast("已发送到 ${target.name}")
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if (active(run, session)) preview?.failed(failure.message ?: "发送未确认，请重试") }
        }
    }
    fun openCard(card: JSONObject, message: ChatMessage) {
        if (card.optString("group_id") != group()) { toast("请从原群聊打开此分享"); return }
        show(card.getString("group_id"), card.getString("snapshot_id"), message, card.getString("snapshot_id"))
    }
    private fun show(target: String, id: String, message: ChatMessage, original: String) {
        close(); val run = epoch; val session = socialSession(activity)
        val ui = GridShareViews(activity); val body = ui.column().apply { addView(ui.text("正在读取…")) }
        dialog = AlertDialog.Builder(activity).setCustomTitle(ui.dialogTitle("网格快照详情"))
            .setView(ScrollView(activity).apply { addView(body) }).setNegativeButton("关闭", null).create()
        dialog!!.setOnDismissListener { if (run == epoch) { epoch++; job?.cancel() } }; dialog!!.show(); ui.styleDialog(dialog!!)
        job = activity.lifecycleScope.launch {
            try {
                val view = withContext(Dispatchers.IO) { api.request(target, id, session = session) }
                if (!active(run, session) || group() != target) return@launch
                check(view.getString("snapshot_id") == id && view.getString("group_id") == target)
                val grid = view.getJSONObject("document").getJSONObject("grid")
                check(grid.getString("schema") == GridShareModel.SCHEMA)
                body.removeAllViews(); body.addView(ui.text("${view.optString("owner_name")} 分享 · 历史快照", quiet = true))
                body.addView(ui.summary(grid)); body.addView(ui.details(grid))
                if (grid.optString("note").isNotBlank()) body.addView(ui.text(grid.optString("note")))
                body.addView(ui.text("未读取不代表零。网格利润与策略总盈亏不是同一口径。", quiet = true))
                val latest = view.optString("latest_snapshot_id").takeIf { it.startsWith("ai_snapshot_") }
                if (latest != null) body.addView(ui.button("已有更新 · 查看最新快照") { show(target, latest, message, original) })
                if (id == original) {
                    body.addView(ui.button("引用讨论") { if (active(run, session)) { close(); quote(message) } })
                    body.addView(ui.button("询问群 AI") { if (active(run, session)) { close(); analyze(message) } })
                } else body.addView(ui.text("引用最新版本请从群里的新卡片发起。", quiet = true))
                if (view.optString("owner_id") == AuthManager.userId(activity)) {
                    if (latest == null) body.addView(ui.button("手动更新快照") { if (active(run, session)) load(AiConversationShareTarget(target, "当前群聊"), id) })
                    body.addView(ui.button("撤回分享") { if (active(run, session)) revoke(target, id, session) })
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if (active(run, session)) { body.removeAllViews(); body.addView(ui.text(failure.message ?: "读取失败")); body.addView(ui.button("重试") { show(target, id, message, original) }) } }
        }
    }
    private fun revoke(target: String, id: String, session: String) {
        job = activity.lifecycleScope.launch {
            try { withContext(Dispatchers.IO) { api.request(target, id, revoke = true, session = session) }; close(); published(); toast("分享已撤回") }
            catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { toast(failure.message ?: "撤回未确认，请重试") }
        }
    }
    private fun close() { epoch++; job?.cancel(); dialog?.dismiss(); dialog = null; preview?.close(); preview = null }
    fun dispose() { close(); picker.close(); if (current === this) current = null }
    private fun toast(text: String) = Toast.makeText(activity, text, Toast.LENGTH_LONG).show()
    companion object { var current: GridShareFeature? = null; private set }
}
