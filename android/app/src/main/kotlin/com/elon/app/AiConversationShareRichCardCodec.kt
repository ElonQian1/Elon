package com.elon.app

import org.json.JSONArray
import org.json.JSONObject

/** Explicit display-only fields; no provider handles, commands or URLs cross this boundary. */
internal object AiConversationShareRichCardCodec {
    fun encode(card: WebChatProductionRichCard): JSONObject = JSONObject()
        .put("kind", card.kind.name.lowercase()).put("title", card.title)
        .put("description", card.description).put("symbol", card.symbol)
        .put("primary_value", card.primaryValue).put("secondary_value", card.secondaryValue)
        .put("trend", card.trend?.name?.lowercase())
        .put("periods", JSONArray(card.periods.map {
            JSONObject().put("id", it.id).put("label", it.label).put("selected", it.selected)
        })).put("metrics", JSONArray(card.metrics.map { JSONObject().put("label", it.label).put("value", it.value) }))
        .put("series", JSONArray(card.series.map {
            JSONObject().put("key", it.key).put("label", it.label)
                .put("value_prefix", it.valuePrefix).put("value_suffix", it.valueSuffix)
        })).put("points", JSONArray(card.points.map {
            JSONObject().put("label", it.label).put("values", JSONArray(it.values))
        }))

    fun decode(value: JSONObject): WebChatProductionRichCard {
        val kind = when (value.getString("kind")) {
            "finance" -> WebChatProductionRichCard.Kind.FINANCE
            "chart" -> WebChatProductionRichCard.Kind.CHART
            else -> error("分享图表格式无效")
        }
        return WebChatProductionRichCard(kind, value.getString("title"),
            optional(value, "description"), optional(value, "symbol"), optional(value, "primary_value"),
            optional(value, "secondary_value"), optional(value, "trend")?.let {
                WebChatProductionRichCard.Trend.valueOf(it.uppercase())
            }, rows(value, "periods", 12) {
                WebChatProductionRichCard.Period(it.getString("id"), it.getString("label"), it.getBoolean("selected"))
            }, rows(value, "metrics", 16) {
                WebChatProductionRichCard.Metric(it.getString("label"), it.getString("value"))
            }, rows(value, "series", 4) {
                WebChatProductionRichCard.Series(it.getString("key"), it.getString("label"),
                    optional(it, "value_prefix"), optional(it, "value_suffix"))
            }, rows(value, "points", 512) { point ->
                val numbers = point.getJSONArray("values").also { require(it.length() <= 4) }
                WebChatProductionRichCard.Point(point.getString("label"), List(numbers.length()) {
                    numbers.getDouble(it).also { number -> require(number.isFinite()) }
                })
            })
    }

    private fun optional(value: JSONObject, key: String) = value.opt(key) as? String
    private fun <T> rows(value: JSONObject, key: String, max: Int, parse: (JSONObject) -> T): List<T> {
        val rows = value.optJSONArray(key) ?: return emptyList()
        require(rows.length() <= max)
        return List(rows.length()) { parse(rows.getJSONObject(it)) }
    }
}
