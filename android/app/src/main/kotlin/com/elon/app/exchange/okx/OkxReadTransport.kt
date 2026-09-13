package com.elon.app.exchange.okx

import okhttp3.CookieJar
import okhttp3.OkHttpClient
import okhttp3.Request
import java.util.concurrent.TimeUnit

internal fun interface OkxReadGateway { fun get(credentials: OkxCredentials, request: OkxReadRequest): String }

/** No generic URL, cookies, redirects, request body, or write endpoint. */
internal class OkxReadTransport : OkxReadGateway {
    private val client = OkHttpClient.Builder().cookieJar(CookieJar.NO_COOKIES)
        .followRedirects(false).followSslRedirects(false).retryOnConnectionFailure(false)
        .connectTimeout(10, TimeUnit.SECONDS).readTimeout(10, TimeUnit.SECONDS)
        .callTimeout(20, TimeUnit.SECONDS).build()
    override fun get(credentials: OkxCredentials, request: OkxReadRequest): String {
        try {
            val timestamp = OkxReadProtocol.timestamp(System.currentTimeMillis())
            val call = Request.Builder().url(OkxReadProtocol.ORIGIN + request.path).get()
                .header("OK-ACCESS-KEY", credentials.key)
                .header("OK-ACCESS-SIGN", OkxReadProtocol.signature(credentials.secret, timestamp, request))
                .header("OK-ACCESS-TIMESTAMP", timestamp).header("OK-ACCESS-PASSPHRASE", credentials.passphrase)
                .header("Accept", "application/json").build()
            return client.newCall(call).execute().use { response ->
                if (response.code == 429) okxFail(OkxReadFailure.RATE_LIMITED)
                if (!response.isSuccessful) okxFail(if (response.code in setOf(401, 403)) OkxReadFailure.INVALID_CREDENTIALS else OkxReadFailure.NETWORK_UNAVAILABLE)
                val body = response.body ?: okxFail(OkxReadFailure.INVALID_RESPONSE)
                if (body.contentLength() > 524_288) okxFail(OkxReadFailure.RESPONSE_LIMIT)
                body.byteStream().use { it.readBytesBounded(524_288).toString(Charsets.UTF_8) }
            }
        } catch (error: OkxReadException) { throw error }
        catch (_: java.io.IOException) { okxFail(OkxReadFailure.NETWORK_UNAVAILABLE) }
    }
}

/** Nonempty short pages must continue; only a successful empty page proves completeness. */
internal class OkxPendingReader(private val gateway: OkxReadGateway, private val clock: () -> Long = android.os.SystemClock::elapsedRealtime) {
    fun read(credentials: OkxCredentials): List<Map<String, Any?>> {
        val deadline = clock() + 60_000
        val result = linkedMapOf<String, Map<String, Any?>>()
        var request = OkxReadRequest.Pending.first()
        repeat(32) {
            if (clock() >= deadline || Thread.currentThread().isInterrupted) okxFail(OkxReadFailure.NETWORK_UNAVAILABLE)
            val rows = OkxReadProtocol.rows(gateway.get(credentials, request), 100)
            if (clock() >= deadline) okxFail(OkxReadFailure.NETWORK_UNAVAILABLE)
            if (rows.isEmpty()) return result.values.toList()
            var oldest: String? = null
            for (row in rows) {
                val id = OkxReadProtocol.id(row["algoId"] as? String ?: okxFail(OkxReadFailure.INVALID_RESPONSE))
                if (request.cursor?.let { !OkxReadProtocol.older(id, it) } == true || result.put(id, row) != null)
                    okxFail(OkxReadFailure.INVALID_RESPONSE)
                if (oldest == null || OkxReadProtocol.older(id, oldest!!)) oldest = id
                if (result.size > 1000) okxFail(OkxReadFailure.RESPONSE_LIMIT)
            }
            request = OkxReadRequest.Pending.after(oldest!!)
        }
        okxFail(OkxReadFailure.RESPONSE_LIMIT)
    }
}
