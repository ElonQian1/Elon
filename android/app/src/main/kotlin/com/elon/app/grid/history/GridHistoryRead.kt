package com.elon.app.grid.history

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.grid.chat.BinanceGridReader
import com.elon.app.grid.chat.BinanceGridReadStore
import com.elon.app.grid.create.BinanceCreateSlot
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.host.BinanceReportQuery
import com.elon.app.grid.share.GridShareRead
import kotlinx.coroutines.delay
import kotlinx.coroutines.withTimeout
import org.json.JSONObject
import java.util.UUID

internal class GridHistoryRead(private val activity: AppCompatActivity) {
    data class Page(val rows: List<GridShareRead.Snapshot>, val total: Int, val source: BinanceGridReadStore.Context)
    suspend fun page(days: Int, page: Int, expected: BinanceGridReadStore.Context? = null): Page {
        require(days in setOf(7, 30, 90) && page in 1..1000)
        val host = BinanceHostRuntime.onMain(activity) { it }
        check(host.begin()) { "请先在币安账户页完成登录" }
        host.recoverConnection()
        val source = withTimeout(45_000) {
            var found = BinanceGridReader.context(host)
            while (found == null) { delay(300); found = BinanceGridReader.context(host) }
            found
        }
        check(expected == null || source == expected) { "币安来源已变化，请从第一页重新读取" }
        val slot = Any(); check(BinanceCreateSlot.shared.acquire(slot)) { "币安正在读取其他数据，请稍后重试" }
        try {
            val request = UUID.randomUUID().toString().replace("-", "")
            val q = BinanceReportQuery.parse(JSONObject().put("request", request).put("kind", "history")
                .put("page", page).put("days", days).put("id", "").put("symbol", "").toString())
            host.reports.start(q, source.account, source.kind)
            host.view?.evaluateJavascript("window.__elonBinanceReadV1?.report(${q.json()})") { if (it != "true") host.reports.fail(request) }
                ?: error("币安宿主尚未就绪")
            return withTimeout(36_000) {
                while (true) {
                    check(BinanceGridReader.identity(host) == source) { "币安来源已变化，请重新读取" }
                    val reply = JSONObject(host.reports.reply(request, source.account, source.kind, 2))
                    if (reply.optString("status") == "pending") { delay(300); continue }
                    check(reply.optString("status") == "ready" && reply.optString("coverage") == "page") { "历史记录读取失败，请检查币安网络" }
                    val observed = reply.getLong("observed_at_ms")
                    val rows = reply.getJSONArray("rows")
                    val snapshots = (0 until rows.length()).map { i ->
                        val row = rows.getJSONObject(i)
                        val fields = row.keys().asSequence().associateWith { key -> row.opt(key).takeUnless { it == JSONObject.NULL } }
                        GridShareRead.Snapshot(GridHistoryModel.project(fields, observed), observed, source)
                    }
                    check(snapshots.map { it.fields["id"] }.toSet().size == snapshots.size) { "历史页包含重复策略" }
                    return@withTimeout Page(snapshots, reply.getInt("total"), source)
                }
                @Suppress("UNREACHABLE_CODE") error("history_unavailable")
            }
        } finally { BinanceCreateSlot.shared.release(slot) }
    }
}
