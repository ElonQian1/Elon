package com.elon.app.grid.create

import android.app.Activity
import android.os.Handler
import android.os.Looper
import android.widget.LinearLayout
import com.elon.app.grid.ui.BinanceGridAppearance
import java.math.BigDecimal
import java.math.RoundingMode
import java.util.concurrent.Executors

internal class BinanceCreateMarketPanel(activity: Activity, private val symbol: () -> String,
    private val catalogFailed: () -> Unit = {}, private val symbolsLoaded: (List<BinanceGridRule>) -> Unit) {
    private val ui = BinanceGridAppearance(activity)
    val root = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL; isSaveEnabled = false }
    private val text = ui.label("正在读取合约与行情…", 14f).apply { contentDescription = "binance-create-market" }
    private val preview = ui.label("填写价格区间和格数后，可查看网格间距预览。", 13f).apply { contentDescription = "binance-create-preview" }
    private val worker = Executors.newSingleThreadExecutor()
    private val handler = Handler(Looper.getMainLooper())
    private val market = BinanceGridMarket()
    @Volatile private var epoch = 0
    @Volatile private var closed = false
    private var rule: BinanceGridRule? = null
    private var quote: BinanceGridQuote? = null
    init {
        root.addView(text)
        root.addView(ui.button("刷新行情与合约列表", "binance-create-market-refresh") { load() })
        root.addView(preview)
        load()
    }
    fun load() {
        if (closed) return
        val id = ++epoch; rule = null; quote = null
        val selected = symbol().trim().uppercase(java.util.Locale.ROOT)
        text.text = "正在读取 $selected 的公开行情…"
        worker.execute {
            val rulesResult = runCatching { market.rules() }
            handler.post {
                // Public catalog is independent of the selected symbol; don't discard it while switching coins.
                if (!closed) rulesResult.fold(symbolsLoaded, { if (id == epoch) catalogFailed() })
            }
            if (closed || id != epoch) return@execute
            val result = runCatching {
                val rules = rulesResult.getOrThrow()
                val chosen = rules.find { it.symbol == selected } ?: error("请从可交易的 U 本位永续合约中选择")
                chosen to runCatching { market.quote(selected) }
            }
            handler.post {
                if (closed || id != epoch) return@post
                result.fold({ (item, quoteResult) ->
                    rule = item
                    val value = quoteResult.getOrNull()
                    if (value == null) {
                        text.text = "$selected · 交易规则已读取\n行情暂不可用，可稍后刷新。价格步长 ${item.tick.stripTrailingZeros().toPlainString()} USDT"
                        return@fold
                    }
                    quote = value
                    val mark = BigDecimal(value.mark).stripTrailingZeros().toPlainString()
                    val funding = BigDecimal(value.funding).multiply(BigDecimal("100")).stripTrailingZeros().toPlainString()
                    val time = java.text.SimpleDateFormat("HH:mm:ss", java.util.Locale.ROOT).format(java.util.Date(value.observed))
                    text.text = "$selected · U 本位永续\n标记价格 $mark USDT\n资金费率 $funding% · 更新 $time\n价格步长 ${item.tick.stripTrailingZeros().toPlainString()} · 最少数量 ${item.minimumQuantity}\n最低名义价值 ${item.minimumNotional ?: "未返回"} USDT"
                }, { text.text = "行情暂不可用。可点击刷新；请在币安官网核对当前价格和交易规则。" })
            }
        }
    }
    fun validate(draft: BinanceGridDraft) {
        val current = rule ?: error("请先刷新行情，确认当前合约的交易规则")
        current.validate(draft)
    }
    fun preview(values: Map<String, String>) {
        preview.text = runCatching {
            val lower = BigDecimal(values["lower"] ?: ""); val upper = BigDecimal(values["upper"] ?: "")
            val count = values["count"]?.toIntOrNull() ?: error("COUNT")
            require(lower.signum() > 0 && upper > lower && count in 2..10_000)
            val spacing = if (values["spacing"] == "ARITH") {
                val gap = upper.subtract(lower).divide(BigDecimal(count), 12, RoundingMode.HALF_UP).stripTrailingZeros().toPlainString()
                "等差间距约 $gap USDT"
            } else if (values["spacing"] == "GEO") {
                val ratio = Math.expm1(Math.log(upper.divide(lower, 20, RoundingMode.HALF_UP).toDouble()) / count) * 100
                require(ratio.isFinite()); "等比间距约 ${String.format(java.util.Locale.ROOT, "%.4f", ratio)}%"
            } else error("SPACING")
            "区间预览：${lower.toPlainString()} — ${upper.toPlainString()}\n$count 格 · $spacing\n间距示意不等于每格收益，未扣除手续费；最终委托数量以币安为准。"
        }.getOrDefault("填写价格上下限、格数和间距后显示区间预览。")
    }
    fun close() { closed = true; epoch++; handler.removeCallbacksAndMessages(null); worker.shutdownNow() }
}
