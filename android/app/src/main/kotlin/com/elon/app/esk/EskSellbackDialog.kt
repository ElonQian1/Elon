package com.elon.app.esk

import android.app.AlertDialog
import android.graphics.Color
import android.text.InputType
import android.view.Gravity
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import java.time.OffsetDateTime
import java.time.format.DateTimeFormatter
import java.util.UUID
import kotlin.concurrent.thread

internal class EskSellbackDialog(
    private val activity: AppCompatActivity,
    private val api: EskAssetApi,
    private val snapshot: EskAssetSnapshot,
    private val onChanged: () -> Unit,
) {
    private lateinit var dialog: AlertDialog
    private lateinit var amountInput: EditText
    private lateinit var status: TextView
    private lateinit var history: LinearLayout
    private var working = false
    private var idempotencyKey = newKey()

    fun show() {
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(8), dp(20), 0)
            addView(text("当前可用：${snapshot.available} ESK", 14f, "#F8FAFC"))
            addView(text("卖回仅提交申请，不代表成交或付款；当前未设置官方卖回价格。", 12f, "#B8B3A8").apply {
                setPadding(0, dp(8), 0, dp(10))
            })
            amountInput = EditText(activity).apply {
                hint = "例如 100.000000"
                inputType = InputType.TYPE_CLASS_NUMBER or InputType.TYPE_NUMBER_FLAG_DECIMAL
                setSingleLine(true)
                contentDescription = "申请卖回 ESK 数量"
            }
            addView(amountInput)
            status = text("", 12f, "#86EFAC").apply { setPadding(0, dp(8), 0, dp(4)) }
            addView(status)
            addView(text("卖回申请记录", 13f, "#F8FAFC").apply { setPadding(0, dp(10), 0, dp(6)) })
            history = LinearLayout(activity).apply { orientation = LinearLayout.VERTICAL }
            addView(history)
        }
        dialog = AlertDialog.Builder(activity)
            .setTitle("申请卖回 ESK")
            .setView(ScrollView(activity).apply { addView(content) })
            .setPositiveButton("提交申请", null)
            .setNegativeButton("关闭", null)
            .create()
        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener { submit() }
            refreshHistory()
        }
        dialog.show()
    }

    private fun submit() {
        val amount = amountInput.text.toString().trim()
        if (!amount.matches(Regex("^\\d+(\\.\\d{1,6})?$")) || amount.matches(Regex("^0+(\\.0+)?$"))) {
            showStatus("请输入大于 0、最多六位小数的 ESK 数量", error = true)
            return
        }
        runAction("正在提交卖回申请…", {
            api.createSellback(amount, idempotencyKey)
        }) {
            idempotencyKey = newKey()
            amountInput.text?.clear()
            showStatus("卖回申请已提交；这不代表成交或付款。")
            onChanged()
            refreshHistory()
        }
    }

    private fun refreshHistory() {
        runAction("正在读取申请记录…", api::requests) { requests ->
            history.removeAllViews()
            if (requests.isEmpty()) {
                history.addView(text("暂无卖回申请", 12f, "#858B96"))
            } else {
                requests.forEach { history.addView(requestRow(it)) }
            }
            showStatus("")
        }
    }

    private fun requestRow(request: EskSellbackRequest): LinearLayout = LinearLayout(activity).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(0, dp(8), 0, dp(8))
        addView(LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(0, -2, 1f)
            orientation = LinearLayout.VERTICAL
            addView(text("${request.amount} ESK", 13f, "#F8FAFC"))
            val state = if (request.status == "submitted") "已提交，等待处理" else "已撤销"
            addView(text("${formatDate(request.submittedAt)} · $state", 10f, "#858B96"))
        })
        if (request.status == "submitted") {
            addView(text("撤销申请", 12f, "#FBBF24").apply {
                setPadding(dp(10), dp(8), dp(10), dp(8))
                setOnClickListener { confirmCancel(request) }
            })
        }
    }

    private fun confirmCancel(request: EskSellbackRequest) {
        AlertDialog.Builder(activity)
            .setTitle("撤销卖回申请")
            .setMessage("撤销 ${request.amount} ESK 的卖回申请后，冻结数量会恢复为可用。")
            .setPositiveButton("确认撤销") { _, _ ->
                runAction("正在撤销申请…", {
                    api.cancelSellback(request.requestId)
                }) {
                    showStatus("卖回申请已撤销，冻结的 ESK 已恢复为可用。")
                    onChanged()
                    refreshHistory()
                }
            }
            .setNegativeButton("返回", null)
            .show()
    }

    private fun <T> runAction(progress: String, action: () -> T, onSuccess: (T) -> Unit) {
        if (working || activity.isFinishing || activity.isDestroyed) return
        working = true
        showStatus(progress)
        dialog.getButton(AlertDialog.BUTTON_POSITIVE)?.isEnabled = false
        thread(name = "esk-sellback-action") {
            val result = runCatching(action)
            activity.runOnUiThread {
                working = false
                dialog.getButton(AlertDialog.BUTTON_POSITIVE)?.isEnabled = true
                result.onSuccess(onSuccess).onFailure {
                    showStatus(it.message ?: "ESK 操作失败", error = true)
                }
            }
        }
    }

    private fun showStatus(value: String, error: Boolean = false) {
        status.text = value
        status.setTextColor(Color.parseColor(if (error) "#FCA5A5" else "#86EFAC"))
    }

    private fun text(value: String, size: Float, color: String) = TextView(activity).apply {
        text = value
        textSize = size
        setTextColor(Color.parseColor(color))
    }

    private fun formatDate(value: String): String = runCatching {
        OffsetDateTime.parse(value).format(DateTimeFormatter.ofPattern("MM-dd HH:mm"))
    }.getOrDefault(value)

    private fun newKey() = "apk-esk-sellback-${UUID.randomUUID()}"
    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density + .5f).toInt()
}
