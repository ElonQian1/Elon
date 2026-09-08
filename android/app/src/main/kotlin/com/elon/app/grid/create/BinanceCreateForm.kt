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
    private var initialized = false
    init {
        text("symbol", "合约", "例如 NEARUSDT", "NEARUSDT")
        choice("direction", "方向", listOf("做多" to "LONG", "做空" to "SHORT"))
        text("margin", "投入保证金（USDT，本轮上限 2000）", "请填写投入金额")
        text("leverage", "杠杆倍数", "请填写整数倍数")
        choice("marginType", "保证金模式", listOf("逐仓" to "ISOLATED", "全仓" to "CROSSED"))
        text("lower", "网格价格下限（USDT）", "请填写下限")
        text("upper", "网格价格上限（USDT）", "请填写上限")
        text("count", "网格数量", "请填写整数格数")
        choice("spacing", "间距", listOf("等差" to "ARITH", "等比" to "GEO"))
        choice("autoInit", "创建时立即建仓", listOf("是" to "true", "否" to "false"))
        choice("closeOnStop", "终止网格时市价平仓", listOf("是" to "true", "否，保留仓位" to "false"))
        initialized = true
    }
    fun draft() = BinanceGridDraft.parse(readers.mapValues { it.value() })
    private fun text(key: String, title: String, hintText: String, initial: String = "") {
        root.addView(ui.label(title, 15f))
        val field = EditText(activity).apply {
            hint = hintText; setText(initial); isSingleLine = true; isSaveEnabled = false
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO
            contentDescription = "binance-create-$key"
            inputType = if (key == "symbol") android.text.InputType.TYPE_CLASS_TEXT or android.text.InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS
                else android.text.InputType.TYPE_CLASS_NUMBER or android.text.InputType.TYPE_NUMBER_FLAG_DECIMAL
            filters = arrayOf(android.text.InputFilter.LengthFilter(48))
            addTextChangedListener(object : TextWatcher {
                override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) {}
                override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) { if (initialized) edited() }
                override fun afterTextChanged(s: Editable?) {}
            })
        }
        readers[key] = { field.text.toString() }; root.addView(ui.field(field))
    }
    private fun choice(key: String, title: String, options: List<Pair<String, String>>) {
        root.addView(ui.label(title, 15f))
        val field = Spinner(activity).apply {
            isSaveEnabled = false; contentDescription = "binance-create-$key"
            adapter = ui.choices(listOf("请选择") + options.map { it.first })
            backgroundTintList = android.content.res.ColorStateList.valueOf(ui.muted)
            onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
                override fun onItemSelected(parent: AdapterView<*>?, view: View?, position: Int, id: Long) { if (initialized) edited() }
                override fun onNothingSelected(parent: AdapterView<*>?) { if (initialized) edited() }
            }
        }
        readers[key] = { options.getOrNull(field.selectedItemPosition - 1)?.second ?: "" }; root.addView(field)
    }
}
