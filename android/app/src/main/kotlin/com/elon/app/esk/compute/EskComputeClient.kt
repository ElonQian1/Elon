package com.elon.app.esk.compute

import com.elon.app.esk.platform.EskPlatformJson
import com.elon.app.esk.platform.eskPlatformEndpoint
import com.elon.app.esk.platform.newEskPlatformClient
import okhttp3.Call
import okhttp3.Request
import java.io.ByteArrayOutputStream
import java.io.IOException

/** Fixed endpoint/errors. Single-use reader; its client never follows redirects. */
internal class EskComputeClient(private val calls: Call.Factory = newEskPlatformClient()) {
    private val lock = Any()
    private var canceled = false
    private var started = false
    private var active: Call? = null
    fun cancel() = synchronized(lock) { canceled = true; active?.cancel() }
    fun fetch(base: String, page: Int, tokenProvider: () -> String): EskComputeSnapshot {
        val endpoint = eskPlatformEndpoint(base)?.newBuilder()?.encodedPath("/api/me/esk-compute-center")
            ?.query(null)?.addQueryParameter("page", page.toString())?.build() ?: unavailable()
        require(page in 1..1000)
        synchronized(lock) { if (canceled || started) unavailable(); started = true }
        val token = tokenProvider().also { if (it.length !in 1..8192 || it.any { c -> c.code !in 33..126 }) unavailable() }
        try {
            val call = calls.newCall(Request.Builder().url(endpoint).get().header("Accept", "application/json")
                .header("Authorization", "Bearer $token").header("Cache-Control", "no-store").build())
            synchronized(lock) { if (canceled) { call.cancel(); unavailable() }; active = call }
            call.execute().use { response ->
                if (response.code != 200) unavailable()
                val body = response.body ?: unavailable()
                val type = body.contentType() ?: unavailable()
                if (type.type != "application" || type.subtype != "json" || body.contentLength() > EskPlatformJson.MAX_BYTES ||
                    (type.parameter("charset") != null && type.charset() != Charsets.UTF_8)) unavailable()
                val bytes = body.byteStream().use { input ->
                    val out = ByteArrayOutputStream()
                    val buffer = ByteArray(1024)
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        if (out.size() + read > EskPlatformJson.MAX_BYTES) unavailable()
                        out.write(buffer, 0, read)
                    }
                    out.toByteArray()
                }
                val result = EskComputeParser.parse(bytes)
                synchronized(lock) { if (canceled) unavailable() }
                return result
            }
        } catch (_: Exception) { unavailable() }
        finally { synchronized(lock) { active = null } }
    }
    private fun unavailable(): Nothing = throw IOException("ESK_COMPUTE_READ_UNAVAILABLE")
}
