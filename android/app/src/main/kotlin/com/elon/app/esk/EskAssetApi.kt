package com.elon.app.esk

import android.content.Context
import com.elon.app.AuthManager
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.json.JSONObject

internal data class EskAssetSnapshot(
    val mode: String,
    val enabled: Boolean,
    val total: String,
    val available: String,
    val reservedForSellback: String,
    val reservedForQuant: String,
    val reservedTotal: String,
    val balanceRevision: Long,
    val updatedAt: String?,
    val sellbackRequestEnabled: Boolean,
    val chainStatus: String,
    val statusMessage: String,
)

internal data class EskSellbackRequest(
    val requestId: String,
    val amount: String,
    val status: String,
    val submittedAt: String,
)

internal class EskAssetApi(
    private val context: Context,
    private val http: OkHttpClient,
    private val serverUrl: String,
) {
    fun account(): EskAssetSnapshot {
        val root = execute(authenticated(Request.Builder().url(url("/api/me/assets/esk")).get()))
        val asset = root.optJSONObject("asset") ?: error("ESK 资产身份缺失")
        val balance = root.optJSONObject("balance") ?: error("ESK 余额缺失")
        val sellback = root.optJSONObject("sellback") ?: error("ESK 卖回策略缺失")
        require(root.optString("schema") == "yilong.esk.asset_account.v2") {
            "ESK 资产协议版本不匹配"
        }
        require(asset.optString("symbol") == "ESK") { "ESK 资产标识不匹配" }
        require(asset.optString("chain_status") == "not_deployed") { "ESK 上链状态不匹配" }
        require(root.optBoolean("simulated") && !root.optBoolean("funds_moved", true)) {
            "ESK Paper 安全标识不匹配"
        }
        return EskAssetSnapshot(
            mode = root.optString("mode", "invalid"),
            enabled = root.optBoolean("enabled"),
            total = exactAmount(balance, "total"),
            available = exactAmount(balance, "available"),
            reservedForSellback = exactAmount(balance, "reserved_for_sellback"),
            reservedForQuant = exactAmount(balance, "reserved_for_quant"),
            reservedTotal = exactAmount(balance, "reserved_total"),
            balanceRevision = balance.optLong("revision").takeIf { balance.has("revision") && it >= 0 }
                ?: error("ESK 余额修订无效"),
            updatedAt = balance.optString("updated_at").ifBlank { null },
            sellbackRequestEnabled = sellback.optBoolean("request_enabled"),
            chainStatus = asset.optString("chain_status", "unknown"),
            statusMessage = root.optString("status_message", "ESK 资产状态未知"),
        )
    }

    fun requests(): List<EskSellbackRequest> {
        val root = execute(authenticated(Request.Builder().url(url("/api/me/assets/esk/sellback-requests?limit=20")).get()))
        require(root.optBoolean("simulated") && !root.optBoolean("funds_moved", true)) {
            "ESK 卖回申请安全标识不匹配"
        }
        val values = root.optJSONArray("requests") ?: return emptyList()
        return (0 until values.length()).mapNotNull { index ->
            values.optJSONObject(index)?.let(::parseRequest)
        }
    }

    fun createSellback(amount: String, idempotencyKey: String): EskSellbackRequest {
        val body = JSONObject().put("amount", amount).put("idempotency_key", idempotencyKey)
        return parseRequest(execute(authenticated(
            Request.Builder().url(url("/api/me/assets/esk/sellback-requests"))
                .post(body.toString().toRequestBody(JSON)),
        )))
    }

    fun cancelSellback(requestId: String): EskSellbackRequest {
        val body = JSONObject().put("confirmation", "CANCEL ESK SELLBACK REQUEST")
        val encoded = android.net.Uri.encode(requestId)
        return parseRequest(execute(authenticated(
            Request.Builder().url(url("/api/me/assets/esk/sellback-requests/$encoded/cancel"))
                .post(body.toString().toRequestBody(JSON)),
        )))
    }

    private fun parseRequest(value: JSONObject): EskSellbackRequest {
        require(value.optBoolean("simulated") && !value.optBoolean("funds_moved", true)) {
            "ESK 卖回申请安全标识不匹配"
        }
        return EskSellbackRequest(
            requestId = value.optString("request_id"),
            amount = exactAmount(value, "amount"),
            status = value.optString("status"),
            submittedAt = value.optString("submitted_at"),
        )
    }

    private fun exactAmount(value: JSONObject, key: String): String =
        value.optString(key).takeIf { it.matches(EXACT_AMOUNT) }
            ?: error("ESK 金额格式无效")

    private fun authenticated(builder: Request.Builder): Request =
        AuthManager.applyAuth(context, builder).build()

    private fun execute(request: Request): JSONObject = http.newCall(request).execute().use { response ->
        val text = response.body?.string().orEmpty()
        val json = runCatching { JSONObject(text.ifBlank { "{}" }) }.getOrDefault(JSONObject())
        if (!response.isSuccessful) {
            error(json.optString("error").ifBlank { "请求失败 (${response.code})" }.take(500))
        }
        json
    }

    private fun url(path: String) = serverUrl.trimEnd('/') + path

    private companion object {
        val JSON = "application/json".toMediaType()
        val EXACT_AMOUNT = Regex("^\\d+\\.\\d{6}$")
    }
}
