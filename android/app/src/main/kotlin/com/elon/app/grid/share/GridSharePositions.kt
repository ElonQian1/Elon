package com.elon.app.grid.share

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.grid.chat.BinanceGridReader
import com.elon.app.grid.create.BinanceCreateSlot
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.host.BinanceReportQuery
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.delay
import kotlinx.coroutines.withTimeout
import org.json.JSONObject
import java.util.UUID

/** Explicit, bounded read through the existing fixed positions adapter; no durable grant. */
internal object GridSharePositions {
    private val fields = mapOf("quantity" to "positionQty", "notional" to "positionNotional", "entry" to "entryPrice",
        "mark" to "markPrice", "liquidation" to "liquidationPrice", "pnl" to "unrealizedPnl")
    fun project(report: JSONObject, symbol: String): Map<String, String> {
        check(report.optString("status") == "ready" && report.optString("coverage") == "strategy_position") { "持仓暂未读取" }
        val rows = report.getJSONArray("rows")
        check(rows.length() == 1) { "持仓缺失或包含多个方向，本次仅分享网格参数" }
        val row = rows.getJSONObject(0); check(row.optString("symbol") == symbol)
        return fields.mapNotNull { (key, target) ->
            val value = row.opt(key) as? String ?: return@mapNotNull null
            if (!value.matches(Regex("-?(0|[1-9][0-9]{0,29})(\\.[0-9]{1,20})?"))) return@mapNotNull null
            if (key in setOf("entry", "mark", "liquidation") && (value.toDoubleOrNull() ?: 0.0) <= 0) return@mapNotNull null
            target to value
        }.toMap()
    }
    suspend fun read(activity: AppCompatActivity, snapshot: GridShareRead.Snapshot): GridShareRead.Snapshot {
        val slot = Any()
        if (!BinanceCreateSlot.shared.acquire(slot)) return snapshot.copy(positionNotice = "币安忙碌，本次未读取持仓")
        val host = BinanceHostRuntime.onMain(activity) { it }
        try {
            check(BinanceGridReader.context(host) == snapshot.source)
            val request = UUID.randomUUID().toString().replace("-", "")
            val query = BinanceReportQuery.parse(JSONObject().put("request", request).put("kind", "positions")
                .put("page", 1).put("days", 7).put("id", snapshot.fields["id"]).put("symbol", snapshot.fields["symbol"]).toString())
            host.reports.start(query, snapshot.source.account, snapshot.source.kind)
            host.view?.evaluateJavascript("window.__elonBinanceReadV1?.report(${query.json()})") { value -> if (value != "true") host.reports.fail(request) }
                ?: error("币安页面尚未就绪")
            return withTimeout(36_000) {
                while (true) {
                    check(BinanceGridReader.context(host) == snapshot.source) { "币安来源变化" }
                    val report = JSONObject(host.reports.reply(request, snapshot.source.account, snapshot.source.kind, 2))
                    if (report.optString("status") != "pending") return@withTimeout snapshot.copy(fields = snapshot.fields + project(report, snapshot.fields.getValue("symbol")!!))
                    delay(300)
                }
                @Suppress("UNREACHABLE_CODE") snapshot
            }
        } catch (_: TimeoutCancellationException) {
            check(BinanceGridReader.context(host) == snapshot.source) { "币安来源变化，请重新读取" }
            return snapshot.copy(positionNotice = "持仓读取超时，本次仅分享网格参数")
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) {
            check(BinanceGridReader.context(host) == snapshot.source) { "币安来源变化，请重新读取" }
            return snapshot.copy(positionNotice = "持仓未读取，本次分享保留已读到的网格参数和收益")
        } finally { BinanceCreateSlot.shared.release(slot) }
    }
}
