package com.elon.app.esk.platform.access

import com.elon.app.esk.platform.eskPlatformEndpoint
import com.elon.app.esk.platform.newEskPlatformClient
import okhttp3.Call
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.IOException

/** Never logs server responses, private credentials, or credential-bearing exception causes. */
internal class AssetAccessApprovalClient {
    @Volatile private var canceled = false
    @Volatile private var active: Call? = null
    fun cancel() { canceled = true; active?.cancel() }

    fun authorize(base: String, token: String, input: AssetAccessRequest): String {
        try {
            val endpoint = eskPlatformEndpoint(base)?.newBuilder()?.encodedPath("/api/me/asset-access/authorize")
                ?.query(null)?.build() ?: throw IOException()
            require(token.length in 1..8192 && token.all { it.code in 33..126 })
            if (canceled) throw IOException()
            val request = Request.Builder().url(endpoint).header("Authorization", "Bearer $token")
                .header("Accept", "application/json").header("Cache-Control", "no-store")
                .post(input.approvalBody().toRequestBody("application/json; charset=utf-8".toMediaType())).build()
            val call = newEskPlatformClient().newCall(request).also { active = it }
            if (canceled) { call.cancel(); throw IOException() }
            call.execute().use { response ->
                if (response.code != 200) throw IOException()
                val body = response.body ?: throw IOException()
                if (body.contentType()?.type != "application" || body.contentType()?.subtype != "json" ||
                    body.contentLength() > 4096) throw IOException()
                val bytes = body.byteStream().use { stream ->
                    val output = java.io.ByteArrayOutputStream()
                    val buffer = ByteArray(1024)
                    while (true) {
                        val read = stream.read(buffer)
                        if (read < 0) break
                        if (output.size() + read > 4096) throw IOException()
                        output.write(buffer, 0, read)
                    }
                    output.toByteArray()
                }
                if (bytes.size > 4096 || canceled) throw IOException()
                val raw = Charsets.UTF_8.newDecoder().decode(java.nio.ByteBuffer.wrap(bytes)).toString()
                if (!input.validateResult(raw, System.currentTimeMillis())) throw IOException()
                return raw
            }
        } catch (_: Exception) { throw IOException("资产授权未完成") }
        finally { active = null }
    }
}
