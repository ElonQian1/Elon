package com.elon.app

import android.content.Context
import okhttp3.HttpUrl.Companion.toHttpUrl
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import java.util.Locale

internal data class TokenUsageSummaryLine(
    val label: String,
    val value: String,
    val note: String? = null
)

internal data class TokenUsageSummary(
    val lines: List<TokenUsageSummaryLine>,
    val days: Int = 30,
    val totalTokens: Long = 0,
    val weekTokens: Long = 0,
    val remainingTokens: Long? = null,
    val limitTokens: Long? = null,
    val resetText: String? = null
)

internal object TokenUsageSummaryClient {
    private val http = OkHttpClient()

    fun fetch(context: Context, days: Int = 30): TokenUsageSummary {
        val token = AuthManager.token(context) ?: error("未登录")
        val userId = AuthManager.effectiveUserId(context)
        val url = BuildConfig.SERVER_URL.trimEnd('/').toHttpUrl()
            .newBuilder()
            .addPathSegments("api/user")
            .addPathSegment(userId)
            .addPathSegments("usage/stats")
            .addQueryParameter("days", days.coerceIn(1, 365).toString())
            .build()
        val request = Request.Builder()
            .url(url)
            .header("Authorization", "Bearer $token")
            .get()
            .build()

        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error("加载失败（${response.code}）")
            return parse(JSONObject(body), days)
        }
    }

    private fun parse(json: JSONObject, days: Int): TokenUsageSummary {
        val total = json.optJSONObject("total") ?: JSONObject()
        val totalTokens = total.optLong("total_tokens", 0)
        val weekTokens = recentDaysTokens(json, 7)
        val quota = firstObject(json, "quota", "usage_quota", "token_quota", "limits")
        val remainingTokens = quota?.longOrNull("remaining_tokens", "tokens_remaining", "remaining")
            ?: json.longOrNull("remaining_tokens", "tokens_remaining")
        val limitTokens = quota?.longOrNull("limit_tokens", "quota_tokens", "max_tokens", "total_limit")
            ?: json.longOrNull("limit_tokens", "quota_tokens", "max_tokens")
        val resetText = quota?.stringOrNull("reset_at", "resets_at", "reset_time", "reset")
            ?: json.stringOrNull("reset_at", "resets_at", "reset_time")

        val lines = mutableListOf<TokenUsageSummaryLine>()
        if (remainingTokens != null) {
            val percent = limitTokens
                ?.takeIf { it > 0 }
                ?.let { "${((remainingTokens.toDouble() / it) * 100).toInt().coerceIn(0, 100)}%" }
            lines += TokenUsageSummaryLine(
                label = "剩余",
                value = listOfNotNull(percent, formatTokens(remainingTokens)).joinToString(" "),
                note = resetText
            )
        } else {
            lines += TokenUsageSummaryLine("剩余额度", "未配置")
        }
        lines += TokenUsageSummaryLine("${days}天已用", formatTokens(totalTokens))
        if (weekTokens != totalTokens) {
            lines += TokenUsageSummaryLine("7天已用", formatTokens(weekTokens))
        }
        return TokenUsageSummary(
            lines = lines,
            days = days,
            totalTokens = totalTokens,
            weekTokens = weekTokens,
            remainingTokens = remainingTokens,
            limitTokens = limitTokens,
            resetText = resetText
        )
    }

    private fun recentDaysTokens(json: JSONObject, count: Int): Long {
        val byDay = json.optJSONArray("by_day") ?: return 0L
        var total = 0L
        for (i in 0 until minOf(count, byDay.length())) {
            total += byDay.optJSONObject(i)?.optLong("total_tokens", 0) ?: 0L
        }
        return total
    }

    private fun firstObject(json: JSONObject, vararg keys: String): JSONObject? {
        for (key in keys) {
            json.optJSONObject(key)?.let { return it }
        }
        return null
    }

    private fun JSONObject.longOrNull(vararg keys: String): Long? {
        for (key in keys) {
            if (has(key) && !isNull(key)) return optLong(key)
        }
        return null
    }

    private fun JSONObject.stringOrNull(vararg keys: String): String? {
        for (key in keys) {
            val value = optString(key, "").trim()
            if (value.isNotEmpty()) return value
        }
        return null
    }

    fun formatTokens(tokens: Long): String = "${formatCount(tokens)} tokens"

    fun formatCount(value: Long): String = when {
        value >= 1_000_000_000 -> String.format(Locale.US, "%.1fB", value / 1_000_000_000.0)
        value >= 1_000_000 -> String.format(Locale.US, "%.1fM", value / 1_000_000.0)
        value >= 1_000 -> String.format(Locale.US, "%.1fK", value / 1_000.0)
        else -> value.toString()
    }
}
