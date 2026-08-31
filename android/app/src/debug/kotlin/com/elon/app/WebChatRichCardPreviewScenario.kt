package com.elon.app

import android.graphics.Color
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import com.elon.uiruntime.view.UiRuntimePreviewRequest
import com.elon.uiruntime.view.UiRuntimePreviewScenario
import com.elon.uiruntime.view.uiNode

internal fun webChatRichCardPreviewScenario() = object : UiRuntimePreviewScenario {
    override val screenId = "elon.web_chat.rich_card"
    override val supportedScenarios = setOf("finance", "chart")

    override fun createView(context: android.content.Context, request: UiRuntimePreviewRequest): View {
        val card = if (request.scenario == "finance") financeCard() else chartCard()
        val container = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(context, 20), dp(context, 40), dp(context, 20), dp(context, 40))
            setBackgroundColor(Color.rgb(7, 9, 13))
        }
        container.addView(WebChatProductionRichCardViews.inline(
            container = container,
            card = card,
            contentDescription = "preview-rich-card:${request.scenario}",
            onClick = {
                container.removeAllViews()
                container.addView(WebChatProductionRichCardViews.detail(context, card))
            },
        ))
        return ScrollView(context).apply {
            setBackgroundColor(Color.rgb(7, 9, 13))
            addView(container)
        }.uiNode("web_chat.rich_card.${request.scenario}")
    }

    private fun financeCard() = WebChatProductionRichCard(
        kind = WebChatProductionRichCard.Kind.FINANCE,
        title = "示例行情",
        symbol = "DEMO",
        primaryValue = "123.45",
        secondaryValue = "+1.20  (+0.98%)",
        trend = WebChatProductionRichCard.Trend.POSITIVE,
        periods = listOf(
            WebChatProductionRichCard.Period("1d", "1日", true),
            WebChatProductionRichCard.Period("5d", "5日", false),
            WebChatProductionRichCard.Period("1m", "1月", false),
        ),
        metrics = listOf(
            WebChatProductionRichCard.Metric("开盘", "122.10"),
            WebChatProductionRichCard.Metric("成交量", "10.2M"),
            WebChatProductionRichCard.Metric("最高", "124.00"),
            WebChatProductionRichCard.Metric("最低", "121.80"),
        ),
        series = listOf(WebChatProductionRichCard.Series("value", "价格")),
        points = listOf(121.9, 122.4, 122.1, 123.0, 123.45).mapIndexed { index, value ->
            WebChatProductionRichCard.Point("T$index", listOf(value))
        },
    )

    private fun chartCard() = WebChatProductionRichCard(
        kind = WebChatProductionRichCard.Kind.CHART,
        title = "季度趋势",
        description = "固定测试数据，不会访问网络或用户会话。",
        series = listOf(
            WebChatProductionRichCard.Series("revenue", "收入"),
            WebChatProductionRichCard.Series("cost", "成本"),
        ),
        points = listOf(
            WebChatProductionRichCard.Point("Q1", listOf(10.0, 7.0)),
            WebChatProductionRichCard.Point("Q2", listOf(12.5, 7.8)),
            WebChatProductionRichCard.Point("Q3", listOf(13.2, 8.1)),
            WebChatProductionRichCard.Point("Q4", listOf(15.0, 8.8)),
        ),
    )

    private fun dp(context: android.content.Context, value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()
}
