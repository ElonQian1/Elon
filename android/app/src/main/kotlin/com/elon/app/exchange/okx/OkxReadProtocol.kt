package com.elon.app.exchange.okx

import com.elon.app.privateaccess.StrictJson
import java.security.MessageDigest
import java.time.Instant
import java.time.format.DateTimeFormatterBuilder
import java.util.Base64
import javax.crypto.Mac
import javax.crypto.spec.SecretKeySpec

/** Global production READ only. Demo credentials never enter this credential domain. */
internal class OkxCredentials(val key: String, val secret: String, val passphrase: String) {
    init {
        for (value in listOf(key, secret, passphrase))
            require(value.length in 1..256 && value.all { it.code in 33..126 })
    }
    override fun toString() = "OkxCredentials(private)"
}

internal class OkxAccount(val reference: String, val kind: String) {
    override fun toString() = "OkxAccount(private)"
}

internal enum class OkxReadFailure {
    AUTHORIZATION_REQUIRED, ACCOUNT_CHANGED, READ_ONLY_KEY_REQUIRED, INVALID_CREDENTIALS,
    CLOCK_SKEW, RATE_LIMITED, NETWORK_UNAVAILABLE, INVALID_RESPONSE, RESPONSE_LIMIT,
    BUSY, HOST_SESSION_REQUIRED, CONNECTION_CHANGED
}
internal class OkxReadException(val reason: OkxReadFailure) : IllegalStateException(reason.name)
internal fun okxFail(reason: OkxReadFailure): Nothing = throw OkxReadException(reason)

internal sealed class OkxReadRequest(val path: String) {
    object Account : OkxReadRequest("/api/v5/account/config")
    class Pending private constructor(val cursor: String?) : OkxReadRequest(
        "/api/v5/tradingBot/grid/orders-algo-pending?algoOrdType=contract_grid&instType=SWAP&limit=100" +
            (cursor?.let { "&after=$it" } ?: "")) {
        companion object {
            fun first() = Pending(null)
            fun after(id: String) = Pending(OkxReadProtocol.id(id))
        }
    }
    class Detail private constructor(val id: String) : OkxReadRequest(
        "/api/v5/tradingBot/grid/orders-algo-details?algoOrdType=contract_grid&algoId=$id") {
        companion object { fun of(id: String) = Detail(OkxReadProtocol.id(id)) }
    }
    class History private constructor(val after: String) : OkxReadRequest(
        "/api/v5/tradingBot/grid/orders-algo-history?algoOrdType=contract_grid&instType=SWAP&limit=50" +
            if (after.isEmpty()) "" else "&after=$after") {
        companion object { fun of(after: String) = History(if (after.isEmpty()) "" else OkxReadProtocol.id(after)) }
    }
    override fun toString() = "OkxReadRequest(fixed GET)"
}

internal object OkxReadProtocol {
    const val SCHEMA = "yilong.okx_host_read.v1"
    const val ORIGIN = "https://www.okx.com"
    const val ENVIRONMENT = "live"
    private val timestampFormat = DateTimeFormatterBuilder().appendInstant(3).toFormatter()
    fun timestamp(now: Long): String = timestampFormat.format(Instant.ofEpochMilli(now))
    fun signature(secret: String, timestamp: String, request: OkxReadRequest): String {
        val mac = Mac.getInstance("HmacSHA256")
        mac.init(SecretKeySpec(secret.toByteArray(Charsets.UTF_8), "HmacSHA256"))
        return Base64.getEncoder().encodeToString(mac.doFinal((timestamp + "GET" + request.path).toByteArray(Charsets.UTF_8)))
    }
    fun digest(value: String): String = MessageDigest.getInstance("SHA-256")
        .digest(value.toByteArray(Charsets.UTF_8)).joinToString("") { "%02x".format(it) }
    fun id(value: String): String = value.also { require(Regex("[1-9][0-9]{0,63}").matches(it)) }
    fun older(a: String, b: String): Boolean = a.length < b.length || (a.length == b.length && a < b)

    @Suppress("UNCHECKED_CAST")
    fun rows(raw: String, limit: Int): List<Map<String, Any?>> {
        val root = StrictJson.parse(raw, 524_288, 1000)
        val code = root["code"] as? String ?: okxFail(OkxReadFailure.INVALID_RESPONSE)
        if (code != "0") okxFail(when (code) {
            "50011", "50040" -> OkxReadFailure.RATE_LIMITED
            "50102" -> OkxReadFailure.CLOCK_SKEW
            "50103", "50104", "50105", "50106", "50110", "50111", "50113", "50114", "50119" -> OkxReadFailure.INVALID_CREDENTIALS
            else -> OkxReadFailure.INVALID_RESPONSE
        })
        val rows = root["data"] as? List<*> ?: okxFail(OkxReadFailure.INVALID_RESPONSE)
        if (rows.size > limit) okxFail(OkxReadFailure.RESPONSE_LIMIT)
        return rows.map { it as? Map<String, Any?> ?: okxFail(OkxReadFailure.INVALID_RESPONSE) }
    }
    fun account(raw: String): OkxAccount {
        val row = rows(raw, 1).singleOrNull() ?: okxFail(OkxReadFailure.INVALID_RESPONSE)
        val uid = id(row["uid"] as? String ?: okxFail(OkxReadFailure.INVALID_RESPONSE))
        val main = id(row["mainUid"] as? String ?: okxFail(OkxReadFailure.INVALID_RESPONSE))
        val permissions = (row["perm"] as? String)?.split(',')?.map(String::trim)?.toSet()
        if (permissions != setOf("read_only")) okxFail(OkxReadFailure.READ_ONLY_KEY_REQUIRED)
        return OkxAccount(digest("okx:global:live:$uid"), if (uid == main) "primary" else "sub")
    }
}
