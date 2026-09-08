package com.elon.app.grid.create

import android.app.Activity
import android.text.Editable
import android.text.TextWatcher
import android.view.View
import android.widget.*
import com.elon.app.grid.ui.BinanceGridAppearance

/** UI strings are mapped to typed choices; the immutable draft owns validation and request semantics. */
internal class BinanceCreateForm(private val activity: Activity, private val edited: () -> Unit) {
    private val ui = BinanceGridAppearance(activity)
    val root = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL; isSaveEnabled = false }
    private val readers = linkedMapOf<String, () -> String>()
    private var target = root
    private val inputs = mutableMapOf<String, EditText>()
    private lateinit var market: BinanceCreateMarketPanel
    private var initialized = false
    init {
        text("symbol", "合约", "例如 NEARUSDT", "NEARUSDT")
        market = BinanceCreateMarketPanel(activity, { readers["symbol"]?.invoke().orEmpty() }) { names ->
            (inputs["symbol"] as? AutoCompleteTextView)?.setAdapter(ui.choices(names))
        }
        root.addView(market.root)
        (inputs["symbol"] as? AutoCompleteTextView)?.setOnItemClickListener { _, _, _, _ -> market.load() }
        choice("direction", "方向", listOf("做多" to "LONG", "做空" to "SHORT", "中性" to "NEUTRAL"))
        text("margin", "投入保证金（USDT，本轮上限 2000）", "请填写投入金额")
        text("leverage", "杠杆倍数", "请填写整数倍数")
        choice("marginType", "保证金模式", listOf("逐仓" to "ISOLATED", "全仓" to "CROSSED"))
        text("lower", "网格价格下限（USDT）", "请填写下限")
        text("upper", "网格价格上限（USDT）", "请填写上限")
        text("count", "网格数量", "请填写整数格数")
        choice("spacing", "间距", listOf("等差" to "ARITH", "等比" to "GEO"))
        choice("autoInit", "创建时立即建仓", listOf("是" to "true", "否" to "false"))
        choice("closeOnStop", "终止网格时市价平仓", listOf("是" to "true", "否，保留仓位" to "false"))
        val advanced = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL; visibility = View.GONE; isSaveEnabled = false }
        root.addView(ui.button("高级设置 · 触发、止盈止损与追踪", "binance-create-advanced") {
            advanced.visibility = if (advanced.visibility == View.VISIBLE) View.GONE else View.VISIBLE
        })
        root.addView(advanced); target = advanced
        target.addView(ui.label("可留空。做多：终止下限为止损，上限为止盈；做空相反；中性按上下限终止。", 13f))
        text("triggerPrice", "延后启动的触发价 / USDT", "不填则不设触发价")
        choice("triggerType", "启动价格依据", listOf("标记价格" to "MARK_PRICE", "最新成交价" to "CONTRACT_PRICE"), "MARK_PRICE")
        choice("stopMode", "止盈止损依据", listOf("价格上下限" to "PRICE", "盈亏金额 / USDT" to "PNL", "相对投入保证金的收益率 / %" to "ROI"), "PRICE")
        text("stopLower", "终止下限价格 / USDT", "不填则不设下限终止")
        text("stopUpper", "终止上限价格 / USDT", "不填则不设上限终止")
        text("stopProfit", "止盈金额或收益率（按上方选择）", "均填正数；价格模式请留空")
        text("stopLoss", "止损金额或亏损率（按上方选择）", "均填正数；价格模式请留空")
        choice("stopType", "终止价格依据", listOf("标记价格" to "MARK_PRICE", "最新成交价" to "CONTRACT_PRICE"), "MARK_PRICE")
        choice("closeOnTpSl", "止盈止损触发后市价平仓", listOf("是" to "true", "否，保留仓位" to "false"), "true")
        choice("trailingUp", "追踪上涨", listOf("关闭" to "false", "开启" to "true"), "false")
        text("trailingUpPrice", "追踪上涨的最高限制价", "可留空，仅启用上涨追踪时填写")
        choice("trailingDown", "追踪下跌", listOf("关闭" to "false", "开启" to "true"), "false")
        text("trailingDownPrice", "追踪下跌的最低限制价", "可留空，仅启用下跌追踪时填写")
        choice("autoAddMargin", "自动追加保证金", listOf("关闭" to "false", "开启（可能追加使用账户资金）" to "true"), "false")
        target = root
        initialized = true
    }
    fun draft() = BinanceGridDraft.parse(readers.mapValues { it.value() })
    fun checkedDraft() = draft().also { market.validate(it) }
    fun close() { if (::market.isInitialized) market.close() }
    private fun text(key: String, title: String, hintText: String, initial: String = "") {
        target.addView(ui.label(title, 15f))
        val field = (if (key == "symbol") AutoCompleteTextView(activity).apply { threshold = 1 } else EditText(activity)).apply {
            hint = hintText; setText(initial); isSingleLine = true; isSaveEnabled = false
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO
            contentDescription = "binance-create-$key"
            inputType = if (key == "symbol") android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS
                else android.text.InputType.TYPE_CLASS_NUMBER or android.text.InputType.TYPE_NUMBER_FLAG_DECIMAL
            filters = arrayOf(android.text.InputFilter.LengthFilter(48))
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                    if (initialized) { edited(); market.preview(readers.mapValues { it.value() }) }
                }
                override fun afterTextChanged(s: Editable?) {}
            })
        }
        inputs[key] = field; readers[key] = { field.text.toString().let { if (key == "symbol") it.uppercase(java.util.Locale.ROOT) else it } }; target.addView(ui.field(field))
    }
    private fun choice(key: String, title: String, options: List<Pair<String, String>>, initial: String = "") {
        target.addView(ui.label(title, 15f))
        val field = Spinner(activity).apply {
            isSaveEnabled = false; contentDescription = "binance-create-$key"
            adapter = ui.choices(listOf("请选择") + options.map { it.first })
            if (initial.isNotEmpty()) setSelection(options.indexOfFirst { it.second == initial } + 1)
            backgroundTintList = android.content.res.ColorStateList.valueOf(ui.muted)
            onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
                override fun onItemSelected(parent: AdapterView<*>?, view: View?, position: Int, id: Long) { if (initialized) { edited(); market.preview(readers.mapValues { it.value() }) } }
                override fun onNothingSelected(parent: AdapterView<*>?) { if (initialized) edited() }
            }
        }
        readers[key] = { options.getOrNull(field.selectedItemPosition - 1)?.second ?: "" }; target.addView(field)
    }
}
