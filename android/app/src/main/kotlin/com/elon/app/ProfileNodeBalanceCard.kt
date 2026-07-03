package com.elon.app

import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONObject
import kotlin.concurrent.thread

/**
 * 我的页内联积分余额卡片。
 * 无需打开 PC 节点大厅即可一眼看到余额和累计收益。
 */
internal class ProfileNodeBalanceCard(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val openTransactions: () -> Unit = {}
) {
    private var root: LinearLayout? = null
    private var loadSerial = 0

    private lateinit var balanceValue: TextView
    private lateinit var lifetimeValue: TextView
    private lateinit var statusLabel: TextView

    // ─── 公开 API ────────────────────────────────────────────────────────────

    fun attachAndRefresh() {
        val host = binding.profileNodeBalanceContainer
        val card = root ?: buildCard().also { root = it }
        if (card.parent !== host) {
            (card.parent as? ViewGroup)?.removeView(card)
            host.removeAllViews()
            host.addView(card)
        }
        refresh()
    }

    // ─── 刷新逻辑 ────────────────────────────────────────────────────────────

    private fun refresh() {
        if (!AuthManager.isLoggedIn(activity)) {
            renderValues("--", "--")
            statusLabel.text = "未登录"
            return
        }
        val serial = ++loadSerial
        statusLabel.text = "加载中…"
        statusLabel.visibility = View.VISIBLE
        balanceValue.text = "…"
        lifetimeValue.text = "…"
        val ctx = activity.applicationContext
        thread(name = "profile-node-balance") {
            val result = runCatching { fetchBalance(ctx) }
            activity.runOnUiThread {
                if (serial != loadSerial || activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                result
                    .onSuccess {
                        renderValues(formatBalance(it.balance), formatBalance(it.lifetime))
                        statusLabel.visibility = View.GONE
                    }
                    .onFailure {
                        renderValues("--", "--")
                        statusLabel.text = "暂不可用"
                    }
            }
        }
    }

    private fun renderValues(balance: String, lifetime: String) {
        balanceValue.text = balance
        lifetimeValue.text = lifetime
    }

    // ─── 网络 ────────────────────────────────────────────────────────────────

    private fun fetchBalance(ctx: Context): NodeBalance {
        val req = AuthManager.applyAuth(
            ctx,
            Request.Builder().url("$serverUrl/api/me/node-balance").get()
        ).build()
        val body = http.newCall(req).execute().use { it.body?.string() ?: "{}" }
        val json = JSONObject(body)
        return NodeBalance(
            balance = json.optDouble("balance", 0.0),
            lifetime = json.optDouble("lifetime_earned", 0.0)
        )
    }

    private fun formatBalance(v: Double): String = when {
        v >= 1_000_000 -> String.format("%.1fM", v / 1_000_000)
        v >= 1_000     -> String.format("%.1fK", v / 1_000)
        else           -> String.format("%.0f", v)
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density + 0.5f).toInt()

    // ─── UI 构建 ─────────────────────────────────────────────────────────────

    private fun buildCard(): LinearLayout {
        lateinit var card: LinearLayout
        card = LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.setMargins(0, dp(16), 0, 0) }
            orientation = LinearLayout.VERTICAL
            setPadding(0, 0, 0, 0)
            isClickable = false

            // 标题行
            addView(LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                )
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL

                addView(TextView(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
                    includeFontPadding = false
                    text = "节点积分"
                    setTextColor(Color.parseColor("#D6D6D6"))
                    textSize = 16f
                    setTypeface(typeface, Typeface.BOLD)
                })

                statusLabel = TextView(activity).apply {
                    includeFontPadding = false
                    gravity = Gravity.CENTER
                    setPadding(dp(10), dp(4), dp(10), dp(4))
                    text = "加载中…"
                    textSize = 11f
                    setTextColor(Color.parseColor("#8DDC9B"))
                    background = GradientDrawable().apply {
                        setColor(Color.parseColor("#16251A"))
                        cornerRadius = dp(8).toFloat()
                    }
                }
                addView(statusLabel)
            })

            // 余额行
            addView(LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(14) }
                orientation = LinearLayout.HORIZONTAL

                addView(buildMetricBlock("可用余额", "…") { balanceValue = it })
                addView(View(activity).apply {
                    layoutParams = LinearLayout.LayoutParams(0, 1, 1f)
                })
                addView(buildMetricBlock("累计收益", "…") { lifetimeValue = it })
            })

            // 底部 CTA 行：积分明细
            addView(LinearLayout(activity).apply {
                layoutParams = LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).also { it.topMargin = dp(12) }
                orientation = LinearLayout.HORIZONTAL
                gravity = Gravity.CENTER_VERTICAL or Gravity.END

                addView(TextView(activity).apply {
                    includeFontPadding = false
                    text = "积分明细"
                    setTextColor(Color.parseColor("#8DDC9B"))
                    textSize = 12f
                    setPadding(dp(10), dp(4), dp(10), dp(4))
                    background = GradientDrawable().apply {
                        setColor(Color.parseColor("#16251A"))
                        cornerRadius = dp(8).toFloat()
                    }
                    isClickable = true
                    isFocusable = true
                    setOnClickListener { openTransactions() }
                })
            })
        }
        return card
    }

    /** 构建"标签 + 大数值"竖排 block，并通过 onValue 回调交出数值 TextView 引用。 */
    private fun buildMetricBlock(label: String, initValue: String, onValue: (TextView) -> Unit): LinearLayout {
        val valueView = TextView(activity).apply {
            includeFontPadding = false
            text = initValue
            setTextColor(Color.parseColor("#D6D6D6"))
            textSize = 26f
            setTypeface(typeface, Typeface.BOLD)
        }
        onValue(valueView)
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.START
            addView(TextView(activity).apply {
                includeFontPadding = false
                text = label
                setTextColor(Color.parseColor("#777777"))
                textSize = 11f
            })
            addView(valueView)
        }
    }
}
