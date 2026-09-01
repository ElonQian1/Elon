package com.elon.app.esk

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.AuthManager
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class EskAssetCard(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    http: OkHttpClient,
    serverUrl: String,
) {
    private val api = EskAssetApi(activity.applicationContext, http, serverUrl)
    private var root: LinearLayout? = null
    private var loadSerial = 0
    private var snapshot: EskAssetSnapshot? = null
    private lateinit var totalValue: TextView
    private lateinit var availableValue: TextView
    private lateinit var reservedValue: TextView
    private lateinit var statusValue: TextView
    private lateinit var requestButton: TextView

    fun attachAndRefresh() {
        val host = binding.profileEskAssetContainer
        val card = root ?: buildCard().also { root = it }
        if (card.parent !== host) {
            (card.parent as? ViewGroup)?.removeView(card)
            host.removeAllViews()
            host.addView(card)
        }
        refresh()
    }

    private fun refresh() {
        if (!AuthManager.isLoggedIn(activity)) {
            renderUnavailable("登录后查看 ESK 资产")
            return
        }
        val serial = ++loadSerial
        totalValue.text = "… ESK"
        statusValue.text = "正在读取 ESK 资产…"
        requestButton.isEnabled = false
        thread(name = "profile-esk-asset") {
            val result = runCatching(api::account)
            activity.runOnUiThread {
                if (serial != loadSerial || activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                result.onSuccess(::render).onFailure {
                    renderUnavailable(it.message ?: "ESK 资产暂不可用")
                }
            }
        }
    }

    private fun render(value: EskAssetSnapshot) {
        snapshot = value
        totalValue.text = "${value.total} ESK"
        availableValue.text = "${value.available} ESK"
        reservedValue.text = "${value.reserved} ESK"
        statusValue.text = value.statusMessage + "\n未设置官方卖回价格；申请不代表成交或付款。"
        root?.contentDescription = "我的 ESK 资产，Paper 登记，${if (value.chainStatus == "not_deployed") "尚未上链" else "上链状态未知"}"
        requestButton.isEnabled = value.mode == "paper" && value.enabled
        requestButton.alpha = if (requestButton.isEnabled) 1f else .45f
    }

    private fun renderUnavailable(message: String) {
        snapshot = null
        totalValue.text = "— ESK"
        availableValue.text = "—"
        reservedValue.text = "—"
        statusValue.text = message
        requestButton.isEnabled = false
        requestButton.alpha = .45f
    }

    private fun openSellback() {
        val current = snapshot ?: return
        EskSellbackDialog(activity, api, current, ::refresh).show()
    }

    private fun buildCard(): LinearLayout = LinearLayout(activity).apply {
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT,
        )
        orientation = LinearLayout.VERTICAL
        setPadding(dp(16), dp(17), dp(16), dp(16))
        background = GradientDrawable(
            GradientDrawable.Orientation.TL_BR,
            intArrayOf(Color.parseColor("#21342D"), Color.parseColor("#202126")),
        ).apply {
            cornerRadius = dp(16).toFloat()
            setStroke(dp(1), Color.parseColor("#325D49"))
        }
        contentDescription = "我的 ESK 资产，Paper 登记，尚未上链"

        addView(LinearLayout(activity).apply {
            gravity = Gravity.CENTER_VERTICAL
            orientation = LinearLayout.HORIZONTAL
            addView(TextView(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(48), dp(48))
                gravity = Gravity.CENTER
                text = "E"
                textSize = 23f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#86EFAC"))
                background = pill("#173B29", "#47765F", 14)
            })
            addView(LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f).apply {
                    marginStart = dp(13)
                }
                orientation = LinearLayout.VERTICAL
                addView(label("一龙 ESK", 12f, "#AAB0B8", false))
                totalValue = label("… ESK", 25f, "#F8FAFC", true)
                addView(totalValue)
                addView(label("我的 ESK 总持有量", 10f, "#858B96", false))
            })
            addView(label("刷新", 11f, "#86EFAC", true).apply {
                setPadding(dp(10), dp(8), dp(10), dp(8))
                background = pill("#17251F", "#385648", 9)
                setOnClickListener { refresh() }
            })
        })

        addView(LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(13) }
            orientation = LinearLayout.HORIZONTAL
            addView(chip("Paper 登记", "#FDE68A", "#493A1A"))
            addView(chip("尚未上链", "#CBD5E1", "#30363F"))
            addView(chip("未划转资金", "#86EFAC", "#173B29"))
        })

        addView(LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(14) }
            orientation = LinearLayout.HORIZONTAL
            availableValue = metric("当前可用")
            reservedValue = metric("卖回申请冻结")
            addView(availableValue.parent as View, LinearLayout.LayoutParams(0, -2, 1f))
            addView(reservedValue.parent as View, LinearLayout.LayoutParams(0, -2, 1f).apply { marginStart = dp(8) })
        })

        statusValue = label("正在读取 ESK 资产…", 11f, "#D6D3D1", false).apply {
            layoutParams = LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(13) }
            setPadding(dp(11), dp(10), dp(11), dp(10))
            setLineSpacing(0f, 1.18f)
            background = pill("#342D1C", "#5A4922", 9)
        }
        addView(statusValue)

        requestButton = label("申请卖回 ESK", 13f, "#07150D", true).apply {
            layoutParams = LinearLayout.LayoutParams(-1, dp(42)).apply { topMargin = dp(13) }
            gravity = Gravity.CENTER
            background = pill("#4ADE80", "#4ADE80", 10)
            setOnClickListener { openSellback() }
        }
        addView(requestButton)
    }

    private fun metric(title: String): TextView {
        val value = label("—", 13f, "#F8FAFC", true)
        val parent = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(11), dp(10), dp(11), dp(10))
            background = pill("#191D20", "#30363B", 9)
            addView(label(title, 10f, "#858B96", false))
            addView(value)
        }
        value.tag = parent
        return value
    }

    private fun label(value: String, size: Float, color: String, bold: Boolean) = TextView(activity).apply {
        includeFontPadding = false
        text = value
        textSize = size
        setTextColor(Color.parseColor(color))
        if (bold) setTypeface(typeface, Typeface.BOLD)
    }

    private fun chip(value: String, color: String, backgroundColor: String) =
        label(value, 10f, color, true).apply {
            layoutParams = LinearLayout.LayoutParams(-2, -2).apply { marginEnd = dp(6) }
            setPadding(dp(8), dp(5), dp(8), dp(5))
            background = pill(backgroundColor, backgroundColor, 20)
        }

    private fun pill(fill: String, stroke: String, radius: Int) = GradientDrawable().apply {
        setColor(Color.parseColor(fill))
        cornerRadius = dp(radius).toFloat()
        setStroke(dp(1), Color.parseColor(stroke))
    }

    private fun dp(value: Int) = (value * activity.resources.displayMetrics.density + .5f).toInt()
}
