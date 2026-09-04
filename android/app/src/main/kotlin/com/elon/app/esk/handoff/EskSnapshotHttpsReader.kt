package com.elon.app.esk.handoff

import okhttp3.Call
import okhttp3.CacheControl
import okhttp3.CookieJar
import okhttp3.HttpUrl
import okhttp3.HttpUrl.Companion.toHttpUrlOrNull
import okhttp3.OkHttpClient
import okhttp3.Request
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.net.Proxy
import java.net.URI
import java.util.concurrent.TimeUnit

/** Only a configured HTTPS origin is accepted, never a caller URL or alternate server. */
internal fun eskSnapshotEndpoint(configuredBase: String): HttpUrl? = runCatching {
    require(configuredBase == configuredBase.trim() && '\\' !in configuredBase)
    val uri = URI(configuredBase)
    require(uri.scheme == "https" && uri.rawUserInfo == null && uri.rawQuery == null && uri.rawFragment == null)
    require(uri.rawPath.isNullOrEmpty() || uri.rawPath == "/")
    val base = configuredBase.toHttpUrlOrNull() ?: return null
    require(base.isHttps && base.username.isEmpty() && base.password.isEmpty())
    base.newBuilder().encodedPath("/api/me/assets/esk").build()
}.getOrNull()

internal fun newEskSnapshotClient(): OkHttpClient = OkHttpClient.Builder()
    .followRedirects(false).followSslRedirects(false).retryOnConnectionFailure(false)
    .cache(null).cookieJar(CookieJar.NO_COOKIES).proxy(Proxy.NO_PROXY)
    .callTimeout(15, TimeUnit.SECONDS).connectTimeout(15, TimeUnit.SECONDS)
    .readTimeout(15, TimeUnit.SECONDS).writeTimeout(15, TimeUnit.SECONDS).build()

/** One foreground request. No application shared client, interceptor, cache or credential fallback. */
internal class EskSnapshotHttpsReader(private val calls: Call.Factory = newEskSnapshotClient()) {
    private val lock = Any()
    private var canceled = false
    private var started = false
    private var activeCall: Call? = null

    fun cancel() = synchronized(lock) {
        canceled = true
        activeCall?.cancel()
    }

    fun fetch(configuredBase: String, tokenProvider: () -> String): Map<String, String> {
        val endpoint = eskSnapshotEndpoint(configuredBase) ?: throw IOException("Secure source unavailable")
        synchronized(lock) {
            check(!canceled && !started)
            started = true
        }
        // Endpoint policy executes before reading the token and before constructing any request.
        val token = tokenProvider()
        require(token.length in 1..8192 && token.all { it.code in 33..126 })
        val request = Request.Builder().url(endpoint).get().header("Accept", "application/json")
            .header("Authorization", "Bearer $token")
            .cacheControl(CacheControl.Builder().noCache().noStore().build()).build()
        val call = synchronized(lock) {
            check(!canceled)
            calls.newCall(request).also { activeCall = it }
        }
        try {
            call.execute().use { response ->
                if (response.code != 200) throw IOException("Source unavailable")
                val body = response.body ?: throw IOException("Missing response")
                val type = body.contentType() ?: throw IOException("Missing content type")
                require(type.type == "application" && type.subtype == "json")
                require(type.charset(Charsets.UTF_8) == Charsets.UTF_8)
                require(body.contentLength() <= EskSnapshotAccountParser.MAX_BYTES)
                val bytes = body.byteStream().use { input ->
                    val out = ByteArrayOutputStream()
                    val buffer = ByteArray(1024)
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        require(out.size() + read <= EskSnapshotAccountParser.MAX_BYTES)
                        out.write(buffer, 0, read)
                    }
                    out.toByteArray()
                }
                synchronized(lock) { check(!canceled) }
                return EskSnapshotAccountParser.parse(bytes)
            }
        } finally {
            synchronized(lock) { activeCall = null }
        }
    }
}
