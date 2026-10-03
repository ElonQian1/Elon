package com.elon.app.grid.sources

import android.content.Context
import android.os.Handler
import android.os.Looper
import com.elon.app.esk.platform.EskPlatformSessionStore
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.host.BinanceHostState
import com.elon.app.privateaccess.StrictJson
import com.elon.app.privateaccess.number
import java.util.UUID
import java.util.concurrent.Executors

/** Explicit opt-in producer. Reuses host observations; never starts a trade or exports the web session. */
internal class GridDeviceSync private constructor(context: Context) {
    private val prefs = context.getSharedPreferences("grid_device_sources_v1", Context.MODE_PRIVATE)
    private val handler = Handler(Looper.getMainLooper())
    private val worker = Executors.newSingleThreadExecutor()
    @Volatile private var epoch = 0L
    private var lastAccepted: String? = null
    private var inFlight: String? = null
    private var pending: Pair<BinanceHostRuntime, Boolean>? = null
    @Volatile private var transport: GridDeviceHttp? = null
    private val sessions = EskPlatformSessionStore(context) {
        transport?.cancel()
        handler.post { epoch++; lastAccepted = null; pending = null; status = "一龙账号已变化，已停止旧会话同步" }
    }
    var status = "未开启本人设备同步"; private set
    private fun owner() = sessions.capture()?.userId?.let(BinanceHostState::digest)
    fun enabled() = owner()?.let { prefs.getBoolean("enabled_$it", false) } == true
    fun setEnabled(enabled: Boolean, host: BinanceHostRuntime) {
        val owner = owner() ?: error("请先登录一龙")
        check(prefs.edit().putBoolean("enabled_$owner", enabled).commit())
        epoch++; transport?.cancel(); lastAccepted = null
        publish(host, unavailable = !enabled)
    }
    fun changed(host: BinanceHostRuntime) { if (enabled()) publish(host) }
    fun retry(host: BinanceHostRuntime) { lastAccepted = null; publish(host, unavailable = !enabled()) }
    fun deviceId(): String = prefs.getString("device", null) ?: UUID.randomUUID().toString().also {
        check(prefs.edit().putString("device", it).commit())
    }
    private fun publish(host: BinanceHostRuntime, unavailable: Boolean = false) {
        check(Looper.myLooper() == Looper.getMainLooper())
        val session = sessions.capture() ?: return
        val fresh = !unavailable && host.live() && host.owner() == BinanceHostState.digest(session.userId) && host.state.fresh()
        val marker = listOf(session.userId, host.state.account, host.state.generation, fresh, unavailable).joinToString(":")
        if (marker == lastAccepted || marker == inFlight) return
        if (inFlight != null) { pending = host to unavailable; return }
        val device = deviceId()
        val sequence = Math.addExact(prefs.getLong("sequence", 0), 1)
        require(sequence <= 9_007_199_254_740_991L)
        check(prefs.edit().putLong("sequence", sequence).commit())
        val now = System.currentTimeMillis()
        val observed = if (fresh) host.state.observed else now
        val body = StrictJson.encode(mapOf("schema" to GridDeviceContract.SCHEMA, "platform" to "android",
            "device_id" to device, "sequence" to sequence,
            "account" to if (fresh) host.state.account else null, "account_kind" to if (fresh) host.state.accountKind else "unknown",
            "observed_at_ms" to observed, "fresh_until_ms" to if (fresh) observed + 300_000 else observed,
            "status" to if (fresh) "fresh" else "unavailable", "rows" to if (fresh) host.state.snapshotRows() else emptyList<Any>()))
        inFlight = marker; status = "正在同步给本人设备…"
        val ticket = epoch
        worker.execute {
            val client = GridDeviceHttp().also { transport = it }
            val result = runCatching {
                check(ticket == epoch)
                val reply = StrictJson.parse(client.request(session, sessions, body), 4096)
                check(reply.keys == setOf("schema", "sequence", "status")
                    && reply["schema"] == "yilong.grid_device_sources.ack.v1"
                    && reply.number("sequence") == sequence && reply["status"] in setOf("accepted", "unchanged"))
            }
            handler.post {
                if (inFlight == marker) inFlight = null
                if (ticket == epoch && session.sameAs(sessions.capture()) && result.isSuccess) {
                    lastAccepted = marker
                    status = if (fresh) "已同步给本人设备" else "已清除远端旧网格"
                } else if (ticket == epoch) status = "同步未确认，可点击重试；本机读取不受影响"
                val next = pending; pending = null
                if (next != null) publish(next.first, unavailable = next.second)
            }
        }
    }
    companion object {
        @Volatile private var instance: GridDeviceSync? = null
        fun get(context: Context) = instance ?: synchronized(this) {
            instance ?: GridDeviceSync(context.applicationContext).also { instance = it }
        }
    }
}
