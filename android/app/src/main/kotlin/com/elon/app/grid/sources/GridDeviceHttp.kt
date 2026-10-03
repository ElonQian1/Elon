package com.elon.app.grid.sources

import com.elon.app.BuildConfig
import com.elon.app.esk.platform.EskPlatformSession
import com.elon.app.esk.platform.EskPlatformSessionStore
import com.elon.app.esk.platform.eskPlatformEndpoint
import com.elon.app.esk.platform.newEskPlatformClient
import okhttp3.Call
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.ByteArrayOutputStream

/** Fixed HTTPS endpoint, bounded body, no cookies/redirects and no main token returned to callers. */
internal class GridDeviceHttp {
    @Volatile private var canceled = false
    @Volatile private var active: Call? = null
    fun cancel() { canceled = true; active?.cancel() }
    fun request(session: EskPlatformSession, sessions: EskPlatformSessionStore, body: String? = null): String {
        check(!canceled && session.sameAs(sessions.capture())) { "一龙账号已变化，请重新读取" }
        val endpoint = eskPlatformEndpoint(BuildConfig.ASSET_ACCESS_ORIGIN)?.newBuilder()
            ?.encodedPath("/api/me/grid-device-sources")?.query(null)?.build() ?: error("安全同步服务尚未配置")
        val request = Request.Builder().url(endpoint).header("Authorization", "Bearer " + session.token)
            .header("Accept", "application/json").header("Cache-Control", "no-store")
        if (body != null) {
            require(body.toByteArray(Charsets.UTF_8).size <= 256 * 1024)
            request.post(body.toRequestBody("application/json; charset=utf-8".toMediaType()))
        }
        val call = newEskPlatformClient().newCall(request.build()).also { active = it }
        try {
            if (canceled) { call.cancel(); error("读取已取消") }
            return call.execute().use { response ->
                check(response.code == 200) { if (response.code == 401) "一龙登录已失效，请重新登录" else "跨设备同步暂不可用，请检查来源端和网络" }
                val payload = response.body ?: error("来源响应为空")
                val limit = if (body == null) 1024 * 1024 + 16384 else 4096
                check(payload.contentLength() <= limit && payload.contentType()?.type == "application"
                    && payload.contentType()?.subtype == "json" && response.header("Set-Cookie") == null)
                check(response.header("Cache-Control")?.split(',')?.any { it.trim().equals("no-store", true) } == true)
                val bytes = payload.byteStream().use { input ->
                    val output = ByteArrayOutputStream(); val buffer = ByteArray(4096)
                    while (true) {
                        val size = input.read(buffer); if (size < 0) break
                        check(output.size() + size <= limit); output.write(buffer, 0, size)
                    }
                    output.toByteArray()
                }
                check(!canceled && session.sameAs(sessions.capture())) { "一龙账号已变化，请重新读取" }
                Charsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(bytes)).toString()
            }
        } finally { active = null }
    }
}
