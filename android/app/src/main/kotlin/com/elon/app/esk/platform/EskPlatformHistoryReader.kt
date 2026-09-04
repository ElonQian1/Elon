package com.elon.app.esk.platform

import okhttp3.Call
import okhttp3.CacheControl
import okhttp3.Request
import java.io.ByteArrayOutputStream
import java.io.IOException

internal enum class EskPlatformHistoryReadFailure {
    SECURE_SOURCE_REQUIRED, SIGN_IN_REQUIRED, NETWORK_FAILED, INVALID_RESPONSE,
    INVALID_REQUEST, CANCELED, ALREADY_USED, HISTORY_CHANGED
}

/** Fixed failure categories only; server content and credentials never survive as a cause. */
internal class EskPlatformHistoryReadException(val failure: EskPlatformHistoryReadFailure) : IOException(failure.name)

internal class EskPlatformHistoryReader(private val calls: Call.Factory = newEskPlatformClient()) {
    private val lock = Any()
    private var canceled = false
    private var started = false
    private var activeCall: Call? = null

    fun cancel() = synchronized(lock) {
        canceled = true
        activeCall?.cancel()
    }

    fun fetch(base: String, cursor: String?, token: () -> String): EskPlatformHistoryPage {
        // Reuse the exact account origin guard before even invoking the credential supplier.
        val origin = eskPlatformEndpoint(base) ?: fail(EskPlatformHistoryReadFailure.SECURE_SOURCE_REQUIRED)
        if (cursor != null && !EskPlatformHistoryParser.validCursor(cursor)) fail(EskPlatformHistoryReadFailure.INVALID_REQUEST)
        val endpoint = origin.newBuilder().encodedPath("/api/me/assets/esk/platform/history")
            .apply { if (cursor != null) addQueryParameter("cursor", cursor) }.build()
        synchronized(lock) {
            if (canceled) fail(EskPlatformHistoryReadFailure.CANCELED)
            if (started) fail(EskPlatformHistoryReadFailure.ALREADY_USED)
            started = true
        }
        val credential = try { token() } catch (_: Exception) { fail(EskPlatformHistoryReadFailure.SIGN_IN_REQUIRED) }
        if (credential.length !in 1..8192 || credential.any { it.code !in 33..126 }) fail(EskPlatformHistoryReadFailure.SIGN_IN_REQUIRED)
        try {
            val request = Request.Builder().url(endpoint).get().header("Accept", "application/json")
                .header("Authorization", "Bearer $credential")
                .cacheControl(CacheControl.Builder().noCache().noStore().build()).build()
            val call = synchronized(lock) {
                if (canceled) fail(EskPlatformHistoryReadFailure.CANCELED)
                calls.newCall(request).also { activeCall = it }
            }
            call.execute().use { response ->
                if (response.code == 401) fail(EskPlatformHistoryReadFailure.SIGN_IN_REQUIRED)
                if (response.code == 409) fail(EskPlatformHistoryReadFailure.HISTORY_CHANGED)
                if (response.code != 200) fail(EskPlatformHistoryReadFailure.NETWORK_FAILED)
                val body = response.body ?: fail(EskPlatformHistoryReadFailure.INVALID_RESPONSE)
                val type = body.contentType() ?: fail(EskPlatformHistoryReadFailure.INVALID_RESPONSE)
                if (type.type != "application" || type.subtype != "json" ||
                    (type.parameter("charset") != null && type.charset() != Charsets.UTF_8) ||
                    body.contentLength() > EskPlatformHistoryParser.MAX_BYTES
                ) fail(EskPlatformHistoryReadFailure.INVALID_RESPONSE)
                val bytes = body.byteStream().use { input ->
                    val output = ByteArrayOutputStream()
                    val buffer = ByteArray(1024)
                    while (true) {
                        val read = input.read(buffer)
                        if (read < 0) break
                        if (output.size() + read > EskPlatformHistoryParser.MAX_BYTES) fail(EskPlatformHistoryReadFailure.INVALID_RESPONSE)
                        output.write(buffer, 0, read)
                    }
                    output.toByteArray()
                }
                val page = try { EskPlatformHistoryParser.parse(bytes, cursor) }
                    catch (_: Exception) { fail(EskPlatformHistoryReadFailure.INVALID_RESPONSE) }
                synchronized(lock) { if (canceled) fail(EskPlatformHistoryReadFailure.CANCELED) }
                return page
            }
        } catch (error: EskPlatformHistoryReadException) {
            throw error
        } catch (_: Exception) {
            synchronized(lock) {
                fail(if (canceled) EskPlatformHistoryReadFailure.CANCELED else EskPlatformHistoryReadFailure.NETWORK_FAILED)
            }
        } finally {
            synchronized(lock) { activeCall = null }
        }
    }

    private fun fail(failure: EskPlatformHistoryReadFailure): Nothing = throw EskPlatformHistoryReadException(failure)
}
