package com.elon.app.grid.create

import android.app.Activity
import android.app.Dialog
import android.os.Handler
import android.os.Looper
import android.text.Editable
import android.text.TextWatcher
import android.view.View
import android.view.ViewGroup
import android.view.Window
import android.view.WindowManager
import android.widget.*
import com.elon.app.grid.ui.BinanceGridAppearance
import java.util.concurrent.Executors

/** Native contract browser; selecting a row only edits the existing draft. */
internal class BinanceSymbolPicker(private val activity: Activity, private val refresh: () -> Unit,
    private val selected: (String) -> Unit) {
    private val ui = BinanceGridAppearance(activity)
    private val prefs = activity.getSharedPreferences("binance_usdt_symbol_picker_v1", Activity.MODE_PRIVATE)
    private val worker = Executors.newSingleThreadExecutor()
    private val handler = Handler(Looper.getMainLooper())
    private var favorites = prefs.getStringSet("favorites", emptySet()).orEmpty().filter(BinanceSymbolCatalog::valid).take(100).toSet()
    private var recent = prefs.getString("recent", "").orEmpty().split(',').filter(BinanceSymbolCatalog::valid).distinct().take(12)
    private var rules: List<BinanceGridRule> = emptyList()
    private var tickers: Map<String, BinanceSymbolTicker> = emptyMap()
    private var catalogFailed = false
    private var quotesLoading = false
    private var quoteFailed = false
    private var closed = false
    private var dialog: Dialog? = null
    private var render: (() -> Unit)? = null
    private var populateCategories: (() -> Unit)? = null
    private val expiry = object : Runnable {
        override fun run() { if (dialog?.isShowing == true) { render?.invoke(); handler.postDelayed(this, 30_000) } }
    }
    fun catalog(value: List<BinanceGridRule>) {
        rules = value; catalogFailed = false; populateCategories?.invoke(); render?.invoke()
    }
    fun failed() { catalogFailed = true; render?.invoke() }
    fun show(current: String) {
        if (closed || dialog?.isShowing == true) return
        var scope = BinanceSymbolCatalog.Scope.ALL
        var category: String? = null
        var sort = BinanceSymbolCatalog.Sort.NAME
        var rows: List<BinanceGridRule> = emptyList()
        val page = Dialog(activity).apply { requestWindowFeature(Window.FEATURE_NO_TITLE) }
        dialog = page
        val root = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL; setBackgroundColor(ui.background)
            setPadding(ui.dp(16), ui.dp(12), ui.dp(16), ui.dp(12)); fitsSystemWindows = true
        }
        root.addView(LinearLayout(activity).apply {
            addView(ui.button("‹", "binance-symbol-back") { page.dismiss() }, LinearLayout.LayoutParams(ui.dp(48), -2))
            addView(ui.label("选择合约", 24f), LinearLayout.LayoutParams(0, -2, 1f).apply { marginStart = ui.dp(12) })
        })
        root.addView(ui.label("币安 · USDT 永续 · 当前 $current", 13f))
        val search = EditText(activity).apply {
            hint = "搜索币种或合约，如 NEAR、BTC/USDT"; isSingleLine = true; isSaveEnabled = false
            contentDescription = "binance-symbol-search"; importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO
            filters = arrayOf(android.text.InputFilter.LengthFilter(64))
        }
        root.addView(ui.field(search))
        val scopes = LinearLayout(activity)
        val scopeButtons = linkedMapOf<BinanceSymbolCatalog.Scope, Button>()
        for ((value, label) in listOf(BinanceSymbolCatalog.Scope.ALL to "全部", BinanceSymbolCatalog.Scope.FAVORITES to "自选", BinanceSymbolCatalog.Scope.RECENT to "最近选择")) {
            val button = ui.button(label, "binance-symbol-scope-${value.name.lowercase()}") { scope = value; render?.invoke() }
            scopeButtons[value] = button
            scopes.addView(button, LinearLayout.LayoutParams(0, -2, 1f).apply { marginEnd = ui.dp(4) })
        }
        root.addView(scopes)
        val filters = LinearLayout(activity)
        val categories = Spinner(activity).apply { contentDescription = "binance-symbol-category"; minimumHeight = ui.dp(48); isSaveEnabled = false }
        val sorts = Spinner(activity).apply {
            contentDescription = "binance-symbol-sort"; minimumHeight = ui.dp(48); isSaveEnabled = false
            adapter = ui.choices(listOf("名称排序", "24h成交额 ↓", "24h涨幅 ↓", "24h跌幅 ↓"))
        }
        filters.addView(categories, LinearLayout.LayoutParams(0, -2, 1f))
        filters.addView(sorts, LinearLayout.LayoutParams(0, -2, 1f)); root.addView(filters)
        var categoryValues: List<String?> = listOf(null)
        populateCategories = {
            categoryValues = listOf(null) + rules.flatMap { it.categories }.distinct().sorted() + if (rules.any { it.categories.isEmpty() }) listOf("") else emptyList()
            if (category !in categoryValues) category = null
            categories.adapter = ui.choices(categoryValues.map { if (it == null) "全部分类" else BinanceSymbolCatalog.label(it) })
            categories.setSelection(categoryValues.indexOf(category).coerceAtLeast(0))
        }
        populateCategories?.invoke()
        val status = ui.label("", 13f).apply { contentDescription = "binance-symbol-status" }; root.addView(status)
        val list = ListView(activity).apply { divider = null; isSaveEnabled = false; contentDescription = "binance-symbol-list" }
        val adapter = object : BaseAdapter() {
            override fun getCount() = rows.size
            override fun getItem(position: Int) = rows[position]
            override fun getItemId(position: Int) = position.toLong()
            override fun getView(position: Int, convertView: View?, parent: ViewGroup?): View {
                val rule = rows[position]
                val row = convertView as? LinearLayout ?: LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL; setPadding(0, ui.dp(5), 0, ui.dp(5))
                    addView(ui.button("", "") {}.apply { gravity = android.view.Gravity.START or android.view.Gravity.CENTER_VERTICAL }, LinearLayout.LayoutParams(0, -2, 1f))
                    addView(ui.button("", "") {}, LinearLayout.LayoutParams(ui.dp(52), -2).apply { marginStart = ui.dp(6) })
                }
                val quote = tickers[rule.symbol]?.takeIf { it.fresh(System.currentTimeMillis()) }
                fun number(value: java.math.BigDecimal?) = value?.stripTrailingZeros()?.toPlainString() ?: "—"
                val tags = rule.categories.joinToString(" · ") { BinanceSymbolCatalog.label(it) }.ifEmpty { "未分类" }
                val label = buildString {
                    append(if (rule.symbol == current) "✓ " else ""); append(rule.symbol.removeSuffix("USDT")); append(" / USDT · ")
                    append(tags); append("\n最新 "); append(number(quote?.last)); append(" USDT · 24h "); append(number(quote?.change)); append("%")
                    append("\n24h成交额 "); append(number(quote?.volume)); append(" USDT")
                }
                val choose = row.getChildAt(0) as Button
                choose.text = label; choose.contentDescription = "binance-symbol-select-${rule.symbol}"
                choose.setOnClickListener {
                    recent = BinanceSymbolCatalog.recent(recent, rule.symbol)
                    prefs.edit().putString("recent", recent.joinToString(",")).apply()
                    selected(rule.symbol); page.dismiss()
                }
                val favorite = row.getChildAt(1) as Button
                favorite.text = if (rule.symbol in favorites) "★" else "☆"
                favorite.contentDescription = "binance-symbol-favorite-${rule.symbol}"
                favorite.setOnClickListener {
                    if (rule.symbol !in favorites && favorites.size >= 100) Toast.makeText(activity, "自选最多100个，请先移除一个", Toast.LENGTH_SHORT).show()
                    favorites = BinanceSymbolCatalog.favorites(favorites, rule.symbol)
                    prefs.edit().putStringSet("favorites", favorites).apply(); render?.invoke()
                }
                return row
            }
        }
        list.adapter = adapter; root.addView(list, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(ui.label("分类来自交易所。— 表示行情缺失或过期；成交额和涨跌幅不代表网格预期收益。", 12f))
        val reload = ui.button("刷新合约与行情", "binance-symbol-refresh") { if (!quotesLoading) { refresh(); loadQuotes() } }
        root.addView(reload)
        render = {
            reload.isEnabled = !quotesLoading
            rows = BinanceSymbolCatalog.select(rules, search.text.toString(), scope, category, sort, favorites, recent, tickers, System.currentTimeMillis())
            scopeButtons.forEach { (key, button) ->
                button.isSelected = key == scope
                button.backgroundTintList = android.content.res.ColorStateList.valueOf(if (key == scope) ui.accent else ui.surface)
                button.setTextColor(if (key == scope) ui.background else ui.text)
            }
            status.text = when {
                rules.isEmpty() && catalogFailed -> "合约列表暂不可用，请刷新或返回手动输入。"
                rules.isEmpty() -> "正在读取合约列表…"
                catalogFailed -> "合约刷新失败，显示上次列表；创建前需重新校验。"
                rows.isEmpty() -> "没有匹配合约。可修改搜索、分类或自选条件。"
                else -> "${rows.size} 个合约 · " + when { quotesLoading -> "行情读取中"; quoteFailed -> "行情暂不可用"; else -> "已显示可用行情，缺失项为 —" }
            }
            adapter.notifyDataSetChanged()
        }
        fun listener(action: (Int) -> Unit) = object : AdapterView.OnItemSelectedListener {
            override fun onItemSelected(parent: AdapterView<*>?, view: View?, position: Int, id: Long) { action(position); render?.invoke() }
            override fun onNothingSelected(parent: AdapterView<*>?) {}
        }
        categories.onItemSelectedListener = listener { category = categoryValues.getOrNull(it) }
        sorts.onItemSelectedListener = listener { sort = BinanceSymbolCatalog.Sort.values()[it.coerceIn(0, 3)] }
        search.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) { render?.invoke() }
            override fun afterTextChanged(s: Editable?) {}
        })
        page.setContentView(root)
        page.setOnDismissListener { render = null; populateCategories = null; dialog = null; handler.removeCallbacks(expiry) }
        page.show()
        page.window?.apply { setLayout(-1, -1); setBackgroundDrawableResource(android.R.color.transparent); setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE or WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_HIDDEN) }
        render?.invoke(); handler.postDelayed(expiry, 30_000)
        if (rules.isEmpty()) refresh()
        loadQuotes()
    }
    private fun loadQuotes() {
        if (closed || quotesLoading) return
        quotesLoading = true; quoteFailed = false; render?.invoke()
        worker.execute {
            val result = runCatching { BinanceGridMarket().tickers() }
            handler.post {
                if (!closed) { quotesLoading = false; quoteFailed = result.isFailure; tickers = result.getOrDefault(emptyMap()); render?.invoke() }
            }
        }
    }
    fun close() { closed = true; dialog?.dismiss(); handler.removeCallbacksAndMessages(null); worker.shutdownNow() }
}
