package com.elon.app.esk.platform

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

internal enum class EskPlatformReadFailure {
    SECURE_SOURCE_REQUIRED, SIGN_IN_REQUIRED, NETWORK_FAILED, INVALID_RESPONSE, CANCELED, ALREADY_USED
}

/** Fixed categories only: no response body, URL, header, token, or exception cause is retained. */
internal class EskPlatformReadException(val failure: EskPlatformReadFailure) : IOException(failure.name)

internal fun eskPlatformEndpoint(configuredBase: String): HttpUrl? = runCatching {
    require(configuredBase == configuredBase.trim() && '\\' !in configuredBase)
    val uri = URI(configuredBase)
    require(uri.scheme == "https" && uri.rawUserInfo == null && uri.rawQuery == null && uri.rawFragment == null)
    require(uri.rawPath.isNullOrEmpty() || uri.rawPath == "/")
    val base = configuredBase.toHttpUrlOrNull() ?: return null
    require(base.isHttps && base.username.isEmpty() && base.password.isEmpty())
    base.newBuilder().encodedPath("/api/me/assets/esk/platform").addQueryParameter("limit", "20").build()
}.getOrNull()

internal fun newEskPlatformClient(): OkHttpClient = OkHttpClient.Builder()
    .followRedirects(false).followSslRedirects(false).retryOnConnectionFailure(false)
    .cache(null).cookieJar(CookieJar.NO_COOKIES).proxy(Proxy.NO_PROXY)
    .callTimeout(15, TimeUnit.SECONDS).connectTimeout(15, TimeUnit.SECONDS)
    .readTimeout(15, TimeUnit.SECONDS).writeTimeout(15, TimeUnit.SECONDS).build()

internal class EskPlatformAccountReader(private val calls: Call.Factory = newEskPlatformClient()) {
    private val lock = Any()
    private var canceled = false
    private var started = false
    private var activeCall: Call? = null

    fun cancel() = synchronized(lock) {
        canceled = true
        activeCall?.cancel()
    }

    fun fetch(configuredBase: String, tokenProvider: () -> String): EskPlatformAccount {
        val endpoint = eskPlatformEndpoint(configuredBase) ?: fail(EskPlatformReadFailure.SECURE_SOURCE_REQUIRED)
        synchronized(lock) {
            if (canceled) fail(EskPlatformReadFailure.CANCELED)
            if (started) fail(EskPlatformReadFailure.ALREADY_USED)
            started = true
        }
        val token = try { tokenProvider() } catch (_: Exception) { fail(EskPlatformReadFailure.SIGN_IN_REQUIRED) }
        if (token.length !in 1..8192 || token.any { it.code !in 33..126 }) fail(EskPlatformReadFailure.SIGN_IN_REQUIRED)
        try {
            val request = Request.Builder().url(endpoint).get().header("Accept", "application/json")
                .header("Authorization", "Bearer $token")
                .cacheControl(CacheControl.Builder().noCache().noStore().build()).build()
            val call = synchronized(lock) {
                if (canceled) fail(EskPlatformReadFailure.CANCELED)
                calls.newCall(request).also { activeCall = it }
            }
            call.execute().use { response ->
                if (response.code == 401) fail(EskPlatformReadFailure.SIGN_IN_REQUIRED)
                if (response.code != 200) fail(EskPlatformReadFailure.NETWORK_FAILED)
                val body = response.body ?: fail(EskPlatformReadFailure.INVALID_RESPONSE)
                val type = body.contentType() ?: fail(EskPlatformReadFailure.INVALID_RESPONSE)
                if (type.type != "application" || type.subtype != "json" ||
                    (type.parameter("charset") != null && type.charset() != Charsets.UTF_8) ||
                    body.contentLength() > EskPlatformAccountParser.MAX_BYTES
                ) fail(EskPlatformReadFailure.INVALID_RESPONSE)
                val bytes = body.byteStream().use { input ->
                    val out = ByteArrayOutputStream()
                    val buffer = ByteArray(1024)
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        if (out.size() + read > EskPlatformAccountParser.MAX_BYTES) fail(EskPlatformReadFailure.INVALID_RESPONSE)
                        out.write(buffer, 0, read)
                    }
                    out.toByteArray()
                }
                val account = try { EskPlatformAccountParser.parse(bytes) }
                    catch (_: Exception) { fail(EskPlatformReadFailure.INVALID_RESPONSE) }
                synchronized(lock) { if (canceled) fail(EskPlatformReadFailure.CANCELED) }
                return account
            }
        } catch (error: EskPlatformReadException) {
            throw error
        } catch (_: Exception) {
            synchronized(lock) {
                fail(if (canceled) EskPlatformReadFailure.CANCELED else EskPlatformReadFailure.NETWORK_FAILED)
            }
        } finally {
            synchronized(lock) { activeCall = null }
        }
    }

    private fun fail(failure: EskPlatformReadFailure): Nothing = throw EskPlatformReadException(failure)
}
