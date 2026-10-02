package com.elon.app.grid.share

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.grid.chat.BinanceGridReadRequest
import com.elon.app.grid.chat.BinanceGridReadStore
import com.elon.app.grid.chat.BinanceGridReader
import com.elon.app.grid.host.BinanceHostRuntime
import kotlinx.coroutines.delay
import kotlinx.coroutines.withTimeout
import java.util.UUID

internal class GridShareRead(private val activity: AppCompatActivity) {
    data class Snapshot(val fields: Map<String, String?>, val observed: Long, val source: BinanceGridReadStore.Context, val positionNotice: String? = null)
    private suspend fun read(request: BinanceGridReadRequest): Map<String, Any?> = kotlinx.coroutines.withTimeoutOrNull(50_000) {
        var next = request
        while (true) {
            val response = BinanceHostRuntime.onMain(activity) { BinanceGridReader.execute(it, next) }
            when (response["status"]) {
                "ready" -> return@withTimeoutOrNull response
                "pending" -> { delay(300); next = next.copy(start = false) }
                else -> error(when (response["error"]) {
                    "login_required", "list_context_unavailable" -> "请在手机币安账户页登录并打开网格列表"
                    "context_changed" -> "币安账号或页面已变化，请重新读取"
                    "host_busy" -> "币安正在读取其他数据，请稍后重试"
                    else -> "币安网格读取失败，请检查官网登录和网络"
                })
            }
        }
        @Suppress("UNREACHABLE_CODE") emptyMap()
    } ?: error("读取超时，请检查币安网络后重试")
    suspend fun list(): Pair<List<Map<String, String?>>, BinanceGridReadStore.Context> {
        val request = BinanceGridReadRequest("list", "share_${UUID.randomUUID()}", start = true, limit = 50)
        val rows = mutableListOf<Map<String, String?>>()
        var response = read(request)
        while (true) {
            @Suppress("UNCHECKED_CAST") rows.addAll(response["rows"] as List<Map<String, String?>>)
            val next = response["next_offset"] as? Int ?: break
            response = read(request.copy(start = false, offset = next))
        }
        return rows to (context() ?: error("币安登录身份尚未就绪"))
    }
    suspend fun detail(id: String, source: BinanceGridReadStore.Context): Snapshot {
        check(context() == source) { "币安来源已变化，请重新读取" }
        val response = read(BinanceGridReadRequest("detail", "share_${UUID.randomUUID()}", true, id))
        check(context() == source) { "币安来源已变化，请重新读取" }
        @Suppress("UNCHECKED_CAST") val fields = response["row"] as Map<String, String?>
        return GridSharePositions.read(activity, Snapshot(fields, response["observed_at_ms"] as Long, source))
    }
    fun context() = BinanceHostRuntime.onMain(activity, BinanceGridReader::context)
    fun historyContext() = BinanceHostRuntime.onMain(activity, BinanceGridReader::identity)
}
