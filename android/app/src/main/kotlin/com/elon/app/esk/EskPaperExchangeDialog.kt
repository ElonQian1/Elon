package com.elon.app.esk

import android.app.AlertDialog
import android.graphics.Color
import android.text.Editable
import android.text.InputType
import android.text.TextWatcher
import android.view.Gravity
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.RadioGroup
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import java.time.OffsetDateTime
import java.time.format.DateTimeFormatter
import java.util.UUID
import kotlin.concurrent.thread

internal class EskPaperExchangeDialog(
    private val activity: AppCompatActivity,
    private val api: EskAssetApi,
    private val onChanged: () -> Unit,
) {
    private lateinit var dialog: AlertDialog
    private lateinit var balances: TextView
    private lateinit var policy: TextView
    private lateinit var amountInput: EditText
    private lateinit var quoteDetails: TextView
    private lateinit var status: TextView
    private var direction = "usdt_to_esk"
    private var quote: EskExchangeQuote? = null
    private var account: EskExchangeAccount? = null
    private var working = false

    fun show() {
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(6), dp(20), 0)
            addView(text("Paper 模拟 · 未上链 · 不移动真实资金", 12f, "#86EFAC"))
            balances = text("正在读取 ESK / USDT Paper 余额…", 14f, "#F8FAFC").apply {
                setPadding(0, dp(10), 0, dp(6))
            }
            addView(balances)
            policy = text("手续费和价格读取中…", 11f, "#B8B3A8")
            addView(policy)
            addView(directionChoices())
            amountInput = EditText(activity).apply {
                hint = "0.000000"
                inputType = InputType.TYPE_CLASS_NUMBER or InputType.TYPE_NUMBER_FLAG_DECIMAL
                setSingleLine(true)
                contentDescription = "Paper 兑换支付数量"
                addTextChangedListener(object : TextWatcher {
                    override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
                    override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = clearQuote()
                    override fun afterTextChanged(s: Editable?) = Unit
                })
            }
            addView(amountInput)
            quoteDetails = text("先输入数量并获取 60 秒有效报价。", 12f, "#D6D3D1").apply {
                setPadding(0, dp(10), 0, dp(4))
            }
            addView(quoteDetails)
            status = text("", 12f, "#86EFAC").apply { setPadding(0, dp(6), 0, 0) }
            addView(status)
        }
        dialog = AlertDialog.Builder(activity)
            .setTitle("USDT / ESK Paper 兑换")
            .setView(ScrollView(activity).apply { addView(content) })
            .setPositiveButton("获取精确报价", null)
            .setNegativeButton("关闭", null)
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener { primaryAction() }
            refreshAccount()
        }
        dialog.show()
    }

    private fun directionChoices() = RadioGroup(activity).apply {
        orientation = RadioGroup.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(0, dp(12), 0, dp(8))
        addView(choice("USDT → ESK", "usdt_to_esk", checked = true), LinearLayout.LayoutParams(0, -2, 1f))
        addView(choice("ESK → USDT", "esk_to_usdt", checked = false), LinearLayout.LayoutParams(0, -2, 1f))
        setOnCheckedChangeListener { _, checkedId ->
            direction = findViewById<RadioButton>(checkedId)?.tag as? String ?: "usdt_to_esk"
            amountInput.text?.clear()
            clearQuote()
            renderAccount()
        }
    }

    private fun choice(label: String, value: String, checked: Boolean) = RadioButton(activity).apply {
        id = android.view.View.generateViewId()
        text = label
        tag = value
        isChecked = checked
        setTextColor(Color.parseColor("#F8FAFC"))
    }

    private fun refreshAccount() = runAction("正在读取 Paper 兑换账户…", api::exchangeAccount) {
        account = it
        renderAccount()
        showStatus(if (it.enabled) "" else it.statusMessage, error = !it.enabled)
    }

    private fun renderAccount() {
        val value = account
        if (value == null) {
            balances.text = "ESK / USDT Paper 余额暂不可用"
            return
        }
        balances.text = "Paper USDT：${value.usdtAvailable} USDT\n可用 ESK：${value.eskAvailable} ESK"
        val rate = value.usdtPerEsk?.let { "1 ESK = $it USDT" } ?: "价格未配置"
        policy.text = "$rate · 平台手续费 ${value.feePercent ?: "未配置"}\n手续费从目标资产毛额中扣除，报价过期不会成交。"
        val source = if (direction == "usdt_to_esk") "USDT" else "ESK"
        amountInput.hint = "支付 $source，例如 10.000000"
        dialog.getButton(AlertDialog.BUTTON_POSITIVE)?.isEnabled = value.enabled && !working
    }

    private fun primaryAction() {
        val currentQuote = quote
        if (currentQuote == null) requestQuote() else execute(currentQuote)
    }

    private fun requestQuote() {
        val amount = amountInput.text.toString().trim()
        if (!amount.matches(Regex("^\\d+(\\.\\d{1,6})?$")) || amount.matches(Regex("^0+(\\.0+)?$"))) {
            showStatus("请输入大于 0、最多六位小数的兑换数量", error = true)
            return
        }
        runAction("正在获取 60 秒 Paper 报价…", { api.createExchangeQuote(direction, amount) }) {
            quote = it
            quoteDetails.text = "支付：${it.inputAmount} ${it.inputAsset}\n" +
                "兑换毛额：${it.grossOutputAmount} ${it.outputAsset}\n" +
                "平台手续费：-${it.feeAmount} ${it.outputAsset}\n" +
                "预计到账：${it.netOutputAmount} ${it.outputAsset}\n" +
                "有效至：${formatDate(it.expiresAt)}"
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).text = "确认 Paper 模拟兑换"
            showStatus("请核对金额；再次确认后才会写入模拟账本。")
        }
    }

    private fun execute(current: EskExchangeQuote) {
        runAction("正在原子写入 Paper 兑换流水…", {
            api.executeExchange(current.quoteId, "apk-esk-paper-exchange-${UUID.randomUUID()}")
        }) {
            showStatus("Paper 兑换完成：${it.quote.netOutputAmount} ${it.quote.outputAsset} 已记入模拟账本。")
            quote = null
            amountInput.text?.clear()
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).text = "获取精确报价"
            onChanged()
            refreshAccount()
        }
    }

    private fun clearQuote() {
        quote = null
        if (::quoteDetails.isInitialized) quoteDetails.text = "先输入数量并获取 60 秒有效报价。"
        if (::dialog.isInitialized) dialog.getButton(AlertDialog.BUTTON_POSITIVE)?.text = "获取精确报价"
    }

    private fun <T> runAction(progress: String, action: () -> T, onSuccess: (T) -> Unit) {
        if (working || activity.isFinishing || activity.isDestroyed) return
        working = true
        showStatus(progress)
        if (::dialog.isInitialized) dialog.getButton(AlertDialog.BUTTON_POSITIVE)?.isEnabled = false
        thread(name = "esk-paper-exchange") {
            val result = runCatching(action)
            activity.runOnUiThread {
                working = false
                dialog.getButton(AlertDialog.BUTTON_POSITIVE)?.isEnabled = account?.enabled == true
                result.onSuccess(onSuccess).onFailure {
                    showStatus(it.message ?: "Paper 兑换操作失败；余额没有被乐观修改", error = true)
                }
            }
        }
    }

    private fun showStatus(value: String, error: Boolean = false) {
        if (!::status.isInitialized) return
        status.text = value
        status.setTextColor(Color.parseColor(if (error) "#FCA5A5" else "#86EFAC"))
    }

    private fun text(value: String, size: Float, color: String) = TextView(activity).apply {
        text = value
        textSize = size
        setTextColor(Color.parseColor(color))
        setLineSpacing(0f, 1.16f)
    }

    private fun formatDate(value: String): String = runCatching {
        OffsetDateTime.parse(value).format(DateTimeFormatter.ofPattern("MM-dd HH:mm:ss"))
    }.getOrDefault(value)

    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density + .5f).toInt()
}
