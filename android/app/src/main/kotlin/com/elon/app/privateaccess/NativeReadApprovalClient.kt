package com.elon.app.privateaccess

import com.elon.app.esk.platform.eskPlatformEndpoint
import com.elon.app.esk.platform.newEskPlatformClient
import okhttp3.Call
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException

/** Shared bounded native handoff transport; each purpose validates its own exact response. */
internal class NativeReadApprovalClient {
    @Volatile private var canceled = false
    @Volatile private var active: Call? = null
    fun cancel() { canceled = true; active?.cancel() }
    fun authorize(base: String, token: String, body: String, validate: (String) -> Boolean): String {
        try {
            val endpoint = eskPlatformEndpoint(base)?.newBuilder()?.encodedPath("/api/me/asset-access/authorize")
                ?.query(null)?.build() ?: throw IOException()
            require(token.length in 1..8192 && token.all { it.code in 33..126 })
            if (canceled) throw IOException()
            val request = Request.Builder().url(endpoint).header("Authorization", "Bearer $token")
                .header("Accept", "application/json").header("Cache-Control", "no-store")
                .post(body.toRequestBody("application/json; charset=utf-8".toMediaType())).build()
            val call = newEskPlatformClient().newCall(request).also { active = it }
            if (canceled) { call.cancel(); throw IOException() }
            call.execute().use { response ->
                if (response.code != 200) throw IOException()
                if (response.header("Cache-Control")?.split(',')?.none { it.trim().equals("no-store", true) } != false ||
                    response.header("Set-Cookie") != null) throw IOException()
                val payload = response.body ?: throw IOException()
                if (payload.contentType()?.type != "application" || payload.contentType()?.subtype != "json" || payload.contentLength() > 4096) throw IOException()
                val bytes = payload.byteStream().use { stream ->
                    val output = java.io.ByteArrayOutputStream(); val buffer = ByteArray(1024)
                    while (true) {
                        val read = stream.read(buffer); if (read < 0) break
                        if (output.size() + read > 4096) throw IOException()
                        output.write(buffer, 0, read)
                    }; output.toByteArray()
                }
                if (canceled) throw IOException()
                val raw = Charsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(bytes)).toString()
                if (!validate(raw)) throw IOException()
                return raw
            }
        } catch (_: Exception) { throw IOException("只读授权未完成") }
        finally { active = null }
    }
}
