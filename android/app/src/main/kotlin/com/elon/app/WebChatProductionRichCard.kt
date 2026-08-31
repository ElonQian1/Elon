package com.elon.app

data class WebChatProductionRichCard(
    val kind: Kind,
    val title: String,
    val description: String? = null,
    val symbol: String? = null,
    val primaryValue: String? = null,
    val secondaryValue: String? = null,
    val trend: Trend? = null,
    val periods: List<Period> = emptyList(),
    val metrics: List<Metric> = emptyList(),
    val series: List<Series> = emptyList(),
    val points: List<Point> = emptyList(),
) {
    enum class Kind { FINANCE, CHART }

    enum class Trend { POSITIVE, NEGATIVE, NEUTRAL }

    data class Period(
        val id: String,
        val label: String,
        val selected: Boolean,
    )

    data class Metric(
        val label: String,
        val value: String,
    )

    data class Series(
        val key: String,
        val label: String,
        val valuePrefix: String? = null,
        val valueSuffix: String? = null,
    )

    data class Point(
        val label: String,
        val values: List<Double>,
    )
}
