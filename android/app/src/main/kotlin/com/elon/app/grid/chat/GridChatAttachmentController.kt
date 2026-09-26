package com.elon.app.grid.chat

import android.content.Intent
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.widget.EditText
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.host.BinanceAccountActivity
import org.json.JSONObject
import java.util.UUID

/** UI for selecting one snapshot, using the same service as authenticated MCP. */
internal class GridChatAttachmentController(private val activity: AppCompatActivity,
    private val input: EditText, private val scope: () -> String?) {
    private val handler = Handler(Looper.getMainLooper())
    private val draft = GridChatDraft(SystemClock::elapsedRealtime)
    private var dialog: AlertDialog? = null
    private var epoch = 0L
    private var phase = "idle"
    private var rowCount = 0
    fun status() = mapOf("schema" to "yilong.binance_grid_attachment.v1", "active" to (current === this),
        "phase" to phase, "row_count" to rowCount, "draft_present" to input.text.contains(GridChatDraft.BEGIN), "auto_refresh" to false)
    fun remove() {
        epoch++; handler.removeCallbacksAndMessages(null); dialog?.dismiss(); dialog = null
        write(draft.clear(input.text.toString())); phase = "idle"; rowCount = 0
    }
    fun activate() { current = this }
    fun deactivate() {
        if (current === this) current = null
        remove()
    }
    fun reconcile() {
        if (draft.present && !draft.scopeCurrent(scope())) write(draft.clear(input.text.toString()))
    }
    fun validateSend(text: String): Boolean {
        if (!text.contains(GridChatDraft.BEGIN)) return true
        val source = BinanceHostRuntime.onMain(activity, BinanceGridReader::context)
        val error = draft.validate(text, scope(), source) ?: return true
        showError(error); return false
    }
    fun guardMcp(port: com.elon.app.WebChatSocialMcpPort): com.elon.app.WebChatSocialMcpPort =
        object : com.elon.app.WebChatSocialMcpPort {
            override fun uiState() = port.uiState()
            override fun control(args: JSONObject): JSONObject {
                if (args.optString("action") in setOf("send_input", "chatgpt_send_page_input") && !validateSend(input.text.toString())) {
                    return JSONObject().put("control_ok", false).put("error", "grid_snapshot_invalid")
                }
                return port.control(args)
            }
        }
    fun open() {
        if (input.text.contains(GridChatDraft.BEGIN)) {
            dialog = AlertDialog.Builder(activity).setTitle("已附带网格")
                .setMessage("快照已显示在输入框中。发送前会核对来源和有效期；发送按钮不会刷新币安数据。")
                .setPositiveButton("重新读取") { _, _ -> write(draft.clear(input.text.toString())); readList() }
                .setNeutralButton("移除快照") { _, _ -> write(draft.clear(input.text.toString())) }
                .setNegativeButton("返回", null).show()
        } else readList()
    }
    private fun readList() {
        val target = scope() ?: return showError("chat_not_ready")
        phase = "reading_list"; rowCount = 0
        val run = ++epoch
        val request = BinanceGridReadRequest("list", "chat_${UUID.randomUUID()}", start = true, limit = 50)
        dialog = AlertDialog.Builder(activity).setTitle("读取币安网格").setMessage("正在读取手机当前币安账户…")
            .setNegativeButton("取消") { _, _ -> epoch++ }.create().also { it.setOnCancelListener { epoch++ }; it.show() }
        read(request, target, run) { reply ->
            val rows = mutableListOf<Map<String, String?>>()
            fun collect(page: Map<String, Any?>) {
                @Suppress("UNCHECKED_CAST") rows.addAll(page["rows"] as List<Map<String, String?>>)
                val next = page["next_offset"] as? Int
                if (next != null) read(request.copy(start = false, offset = next), target, run, ::collect)
                else select(rows, target, run)
            }
            collect(reply)
        }
    }
    private fun select(rows: List<Map<String, String?>>, target: String, run: Long) {
        rowCount = rows.size; phase = if (rows.isEmpty()) "empty" else "selecting"
        dialog?.dismiss()
        if (rows.isEmpty()) {
            dialog = AlertDialog.Builder(activity).setTitle("当前账户没有运行中的网格")
                .setMessage("手机和 Win 使用各自的币安登录会话。可在手机币安账户页核对，也可以保留当前账户。")
                .setPositiveButton("核对币安账户") { _, _ -> activity.startActivity(Intent(activity, BinanceAccountActivity::class.java)) }
                .setNegativeButton("返回聊天", null).show(); return
        }
        val labels = rows.map { "${it["symbol"]} · ${it["direction"] ?: "?"} ${it["leverage"] ?: "?"}× · ${it["count"] ?: "?"}格 · #${it["id"]}" }.toTypedArray()
        dialog = AlertDialog.Builder(activity).setTitle("选择要附带的网格")
            .setItems(labels) { _, index ->
                phase = "reading_detail"
                val request = BinanceGridReadRequest("detail", "chat_${UUID.randomUUID()}", true, rows[index]["id"])
                read(request, target, run) { preview(it, target) }
            }.setNegativeButton("取消") { _, _ -> epoch++ }.show()
    }
    private fun read(request: BinanceGridReadRequest, target: String, run: Long, done: (Map<String, Any?>) -> Unit) {
        if (run != epoch || activity.isFinishing || activity.isDestroyed || scope() != target) return
        val reply = runCatching { BinanceHostRuntime.onMain(activity) { BinanceGridReader.execute(it, request) } }
            .getOrElse { showError("read_failed"); return }
        when (reply["status"]) {
            "pending" -> handler.postDelayed({ read(request.copy(start = false), target, run, done) }, 300)
            "ready" -> done(reply)
            else -> { dialog?.dismiss(); showError(reply["error"] as? String ?: "read_failed") }
        }
    }
    private fun preview(reply: Map<String, Any?>, target: String) {
        if (scope() != target) return
        val source = BinanceHostRuntime.onMain(activity, BinanceGridReader::context) ?: return showError("context_changed")
        @Suppress("UNCHECKED_CAST") val fields = reply["row"] as Map<String, String?>
        val block = draft.stage(target, source, reply["observed_at_ms"] as Long, reply["valid_for_ms"] as Long, fields)
        phase = "preview"
        dialog = AlertDialog.Builder(activity).setTitle("网格快照预览")
            .setMessage(block).setPositiveButton("加入输入框") { _, _ ->
                if (scope() != target) { showError("context_changed"); return@setPositiveButton }
                write(GridChatDraft.strip(input.text.toString()).trimEnd() + "\n\n" + block)
                phase = "attached"
                Toast.makeText(activity, "已加入快照，输入问题后点击发送。可从附带网格入口移除。", Toast.LENGTH_LONG).show()
            }.setNegativeButton("取消", null).show()
    }
    private fun write(value: String) {
        if (value != input.text.toString()) { input.setText(value); input.setSelection(value.length) }
    }
    private fun showError(code: String) {
        phase = "failed"
        val message = when (code) {
            "login_required", "list_context_unavailable" -> "币安会话尚未就绪，请在手机币安账户页完成登录或打开网格列表后重试。"
            "host_busy" -> "币安正在处理其他操作，请稍后重新点击附带网格。"
            "chat_not_ready" -> "请先等待当前 ChatGPT 会话就绪。"
            "question_required" -> "请在快照之外输入要问的问题。"
            "snapshot_expired" -> "网格快照已过期，请重新点击附带网格。"
            "context_changed", "snapshot_missing", "snapshot_modified" -> "网格来源、聊天目标或快照内容已变化，请移除旧快照后重新读取。"
            else -> "本次网格读取未完成，请稍后重试；不会使用旧数据。"
        }
        Toast.makeText(activity, message, Toast.LENGTH_LONG).show()
    }
    companion object {
        var current: GridChatAttachmentController? = null; private set
        fun chatScope(state: JSONObject, url: String?): String? {
            if (!state.optBoolean("adapter_current") || !state.optBoolean("authenticated") || state.optBoolean("login_required")) return null
            if (url.isNullOrBlank() || !url.startsWith("https://chatgpt.com/")) return null
            return "${state.optLong("page_generation", -1)}:$url"
        }
    }
}
