package com.elon.app.chatgptweb

import com.elon.app.WebChatProductionRichCard
import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebRichCardParser {
    fun parse(part: JSONObject): WebChatProductionRichCard? {
        val content = part.optJSONObject("richContent") ?: return null
        if (content.optString("schema") != SCHEMA) return null
        if (content.optString("source") !in SOURCES) return null
        val kind = content.optString("kind")
        if (kind != part.optString("kind")) return null
        val payload = content.optJSONObject("payload") ?: return null
        return when (kind) {
            "finance" -> parseFinance(payload)
            "chart" -> parseChart(payload)
            else -> null
        }
    }

    private fun parseFinance(payload: JSONObject): WebChatProductionRichCard? {
        val title = requiredText(payload, "title", 120) ?: return null
        val primaryValue = requiredText(payload, "primaryValue", 64) ?: return null
        val trend = when (payload.optString("trend")) {
            "positive" -> WebChatProductionRichCard.Trend.POSITIVE
            "negative" -> WebChatProductionRichCard.Trend.NEGATIVE
            "neutral" -> WebChatProductionRichCard.Trend.NEUTRAL
            else -> return null
        }
        val symbol = optionalText(payload, "symbol", 24) ?: if (payload.has("symbol")) return null else null
        val secondaryValue = optionalText(payload, "secondaryValue", 96)
            ?: if (payload.has("secondaryValue")) return null else null
        val periods = optionalArray(payload, "periods", 12, ::parsePeriod) ?: return null
        val metrics = optionalArray(payload, "metrics", 16, ::parseMetric) ?: return null
        val chart = if (payload.has("chart")) parseFinanceChart(payload.optJSONObject("chart")) ?: return null else emptyList()
        return WebChatProductionRichCard(
            kind = WebChatProductionRichCard.Kind.FINANCE,
            title = title,
            symbol = symbol,
            primaryValue = primaryValue,
            secondaryValue = secondaryValue,
            trend = trend,
            periods = periods,
            metrics = metrics,
            series = listOf(WebChatProductionRichCard.Series("value", primaryValue)),
            points = chart,
        )
    }

    private fun parseChart(payload: JSONObject): WebChatProductionRichCard? {
        val title = requiredText(payload, "title", 120) ?: return null
        val description = optionalText(payload, "description", 240)
            ?: if (payload.has("description")) return null else null
        if (payload.optString("chartType") != "line") return null
        val series = requiredArray(payload, "series", 4, ::parseSeries) ?: return null
        if (series.isEmpty()) return null
        val points = requiredArray(payload, "points", 256) { value ->
            parseChartPoint(value, series.size)
        } ?: return null
        if (points.size < 2) return null
        return WebChatProductionRichCard(
            kind = WebChatProductionRichCard.Kind.CHART,
            title = title,
            description = description,
            series = series,
            points = points,
        )
    }

    private fun parsePeriod(value: JSONObject): WebChatProductionRichCard.Period? {
        val id = requiredText(value, "id", 16) ?: return null
        val label = requiredText(value, "label", 16) ?: return null
        val selected = value.opt("selected") as? Boolean ?: return null
        return WebChatProductionRichCard.Period(id, label, selected)
    }

    private fun parseMetric(value: JSONObject): WebChatProductionRichCard.Metric? {
        val label = requiredText(value, "label", 64) ?: return null
        val metricValue = requiredText(value, "value", 96) ?: return null
        return WebChatProductionRichCard.Metric(label, metricValue)
    }

    private fun parseFinanceChart(value: JSONObject?): List<WebChatProductionRichCard.Point>? {
        value ?: return null
        if (value.optString("kind") != "line") return null
        val points = requiredArray(value, "points", 512) { item ->
            val label = requiredText(item, "x", 64) ?: return@requiredArray null
            val number = finiteNumber(item.opt("y")) ?: return@requiredArray null
            WebChatProductionRichCard.Point(label, listOf(number))
        } ?: return null
        return points.takeIf { it.size >= 2 }
    }

    private fun parseSeries(value: JSONObject): WebChatProductionRichCard.Series? {
        val key = requiredText(value, "key", 48) ?: return null
        val label = requiredText(value, "label", 64) ?: return null
        val prefix = optionalText(value, "valuePrefix", 16)
            ?: if (value.has("valuePrefix")) return null else null
        val suffix = optionalText(value, "valueSuffix", 16)
            ?: if (value.has("valueSuffix")) return null else null
        return WebChatProductionRichCard.Series(key, label, prefix, suffix)
    }

    private fun parseChartPoint(value: JSONObject, seriesCount: Int): WebChatProductionRichCard.Point? {
        val label = requiredText(value, "x", 64) ?: return null
        val values = value.optJSONArray("values") ?: return null
        if (values.length() != seriesCount) return null
        val numbers = buildList {
            for (index in 0 until values.length()) {
                add(finiteNumber(values.opt(index)) ?: return null)
            }
        }
        return WebChatProductionRichCard.Point(label, numbers)
    }

    private fun requiredText(value: JSONObject, key: String, maximum: Int): String? =
        (value.opt(key) as? String)?.trim()?.takeIf { it.isNotEmpty() && it.length <= maximum }

    private fun optionalText(value: JSONObject, key: String, maximum: Int): String? {
        if (!value.has(key)) return null
        val text = value.opt(key) as? String ?: return null
        return text.trim().takeIf { it.length <= maximum }
    }

    private fun finiteNumber(value: Any?): Double? =
        (value as? Number)?.toDouble()?.takeIf(Double::isFinite)

    private fun <T> optionalArray(
        value: JSONObject,
        key: String,
        maximum: Int,
        parser: (JSONObject) -> T?,
    ): List<T>? {
        if (!value.has(key)) return emptyList()
        return parseArray(value.optJSONArray(key) ?: return null, maximum, parser)
    }

    private fun <T> requiredArray(
        value: JSONObject,
        key: String,
        maximum: Int,
        parser: (JSONObject) -> T?,
    ): List<T>? {
        val array = value.optJSONArray(key) ?: return null
        return parseArray(array, maximum, parser)
    }

    private fun <T> parseArray(
        array: JSONArray,
        maximum: Int,
        parser: (JSONObject) -> T?,
    ): List<T>? {
        if (array.length() > maximum) return null
        return buildList {
            for (index in 0 until array.length()) {
                add(parser(array.optJSONObject(index) ?: return null) ?: return null)
            }
        }
    }

    private const val SCHEMA = "yilong.rich-content.v1"
    private val SOURCES = setOf("official_dom", "private_response", "cache")
}
