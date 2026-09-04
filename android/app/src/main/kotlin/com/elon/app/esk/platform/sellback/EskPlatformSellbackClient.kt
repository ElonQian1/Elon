package com.elon.app.esk.platform.sellback

import com.elon.app.esk.platform.eskPlatformEndpoint
import com.elon.app.esk.platform.newEskPlatformClient
import okhttp3.Call
import okhttp3.CacheControl
import okhttp3.HttpUrl
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import java.io.ByteArrayOutputStream
import java.io.IOException
import com.google.gson.Gson

internal enum class SellbackNetworkFailure {
    SECURE_SOURCE_REQUIRED, SIGN_IN_REQUIRED, INVALID_REQUEST, INVALID_RESPONSE,
    UNAVAILABLE, CONFLICT, NOT_FOUND, NETWORK_FAILED, CANCELED, ALREADY_USED,
}
internal class SellbackNetworkException(val failure: SellbackNetworkFailure) : IOException(failure.name)

/** A single request only, on the existing guarded origin; never retries a write automatically. */
internal class EskPlatformSellbackClient(private val calls: Call.Factory = newEskPlatformClient()) {
    private val lock = Any()
    private var started = false
    private var canceled = false
    private var active: Call? = null
    fun cancel() = synchronized(lock) { canceled = true; active?.cancel() }

    fun page(base: String, cursor: String?, token: () -> String): SellbackPage {
        val endpoint = endpoint(base)
        if (cursor != null && !EskPlatformSellbackParser.validCursor(cursor)) fail(SellbackNetworkFailure.INVALID_REQUEST)
        val url = endpoint.newBuilder().addQueryParameter("limit", "20")
            .apply { if (cursor != null) addQueryParameter("cursor", cursor) }.build()
        return parse { EskPlatformSellbackParser.page(exchange(url, null, token), cursor) }
    }
    fun lookup(base: String, id: String, token: () -> String): SellbackResult {
        val endpoint = endpoint(base)
        if (!EskPlatformSellbackParser.validId(id)) fail(SellbackNetworkFailure.INVALID_REQUEST)
        val result = parse { EskPlatformSellbackParser.result(exchange(endpoint.newBuilder().addPathSegment(id).build(), null, token)) }
        if (result.request.id != id) fail(SellbackNetworkFailure.INVALID_RESPONSE)
        return result
    }
    fun lookupKey(base: String, key: String, token: () -> String): SellbackResult {
        val endpoint = endpoint(base)
        if (key.length !in 1..96 || !Regex("[A-Za-z0-9._:-]+").matches(key)) fail(SellbackNetworkFailure.INVALID_REQUEST)
        val body = Gson().toJson(sortedMapOf("schema" to "yilong.esk.platform_sellback_lookup.v1", "idempotency_key" to key))
        val result = parse { EskPlatformSellbackParser.result(exchange(endpoint.newBuilder().addPathSegment("lookup").build(), body, token)) }
        if (result.request.key != key || !result.replayed) fail(SellbackNetworkFailure.INVALID_RESPONSE)
        return result
    }
    fun execute(base: String, action: SellbackAction, token: () -> String): SellbackResult {
        val endpoint = endpoint(base)
        val url = if (action.isSubmit) endpoint else endpoint.newBuilder()
            .addPathSegment(requireNotNull(action.requestId)).addPathSegment("cancel").build()
        val result = parse { EskPlatformSellbackParser.result(exchange(url, action.body, token)) }
        if (!action.matches(result.request) || (action.isSubmit && result.request.status == "canceled" && !result.replayed))
            fail(SellbackNetworkFailure.INVALID_RESPONSE)
        return result
    }
    private fun endpoint(base: String): HttpUrl = eskPlatformEndpoint(base)?.newBuilder()
        ?.encodedPath("/api/me/assets/esk/platform/sellback-requests")?.query(null)?.build()
        ?: fail(SellbackNetworkFailure.SECURE_SOURCE_REQUIRED)

    private fun exchange(url: HttpUrl, body: String?, token: () -> String): ByteArray {
        synchronized(lock) {
            if (canceled) fail(SellbackNetworkFailure.CANCELED)
            if (started) fail(SellbackNetworkFailure.ALREADY_USED)
            started = true
        }
        val credential = try { token() } catch (_: Exception) { fail(SellbackNetworkFailure.SIGN_IN_REQUIRED) }
        if (credential.length !in 1..8192 || credential.any { it.code !in 33..126 }) fail(SellbackNetworkFailure.SIGN_IN_REQUIRED)
        try {
            val request = Request.Builder().url(url).header("Accept", "application/json")
                .header("Authorization", "Bearer $credential")
                .cacheControl(CacheControl.Builder().noCache().noStore().build())
                .apply { if (body == null) get() else post(body.toRequestBody("application/json".toMediaType())) }.build()
            val call = synchronized(lock) {
                if (canceled) fail(SellbackNetworkFailure.CANCELED)
                calls.newCall(request).also { active = it }
            }
            call.execute().use { response ->
                if (response.code != 200) fail(when (response.code) {
                    401 -> SellbackNetworkFailure.SIGN_IN_REQUIRED
                    400 -> SellbackNetworkFailure.INVALID_REQUEST
                    403, 503 -> SellbackNetworkFailure.UNAVAILABLE
                    404 -> SellbackNetworkFailure.NOT_FOUND
                    409 -> SellbackNetworkFailure.CONFLICT
                    else -> SellbackNetworkFailure.NETWORK_FAILED
                })
                val data = response.body ?: fail(SellbackNetworkFailure.INVALID_RESPONSE)
                val type = data.contentType() ?: fail(SellbackNetworkFailure.INVALID_RESPONSE)
                if (type.type != "application" || type.subtype != "json" ||
                    (type.parameter("charset") != null && type.charset() != Charsets.UTF_8) ||
                    data.contentLength() > EskPlatformSellbackParser.MAX_BYTES) fail(SellbackNetworkFailure.INVALID_RESPONSE)
                val bytes = data.byteStream().use { input ->
                    val output = ByteArrayOutputStream()
                    val buffer = ByteArray(1024)
                    while (true) {
                        val count = input.read(buffer)
                        if (count < 0) break
                        if (output.size() + count > EskPlatformSellbackParser.MAX_BYTES) fail(SellbackNetworkFailure.INVALID_RESPONSE)
                        output.write(buffer, 0, count)
                    }
                    output.toByteArray()
                }
                synchronized(lock) { if (canceled) fail(SellbackNetworkFailure.CANCELED) }
                return bytes
            }
        } catch (error: SellbackNetworkException) { throw error }
        catch (_: Exception) { synchronized(lock) {
            fail(if (canceled) SellbackNetworkFailure.CANCELED else SellbackNetworkFailure.NETWORK_FAILED)
        } } finally { synchronized(lock) { active = null } }
    }
    private fun <T> parse(block: () -> T): T = try { block() }
        catch (error: SellbackNetworkException) { throw error }
        catch (_: Exception) { fail(SellbackNetworkFailure.INVALID_RESPONSE) }
    private fun fail(failure: SellbackNetworkFailure): Nothing = throw SellbackNetworkException(failure)
}
