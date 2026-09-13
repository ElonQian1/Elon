package com.elon.app.exchange.okx

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.text.InputType
import android.view.MotionEvent
import android.view.View
import android.view.WindowManager
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.elon.app.grid.host.BinanceHostCaller
import com.elon.app.grid.ui.BinanceGridAppearance
import java.util.concurrent.Executors

/** The main APK owns API credential entry only; grid product screens remain in the quant APK. */
class OkxConnectActivity : Activity() {
    private val ui by lazy { BinanceGridAppearance(this) }
    private val worker = Executors.newSingleThreadExecutor()
    private var epoch = 0L
    private var nonce = ""
    private var verified: OkxReadHost.Verified? = null
    private lateinit var key: EditText
    private lateinit var secret: EditText
    private lateinit var passphrase: EditText
    private lateinit var status: TextView
    private lateinit var verify: Button
    private lateinit var approve: Button
    private fun trusted(): Boolean = runCatching {
        val caller = callingActivity ?: return false
        require(packageName == "com.elon.app" && callingPackage == "com.elon.quant")
        require(caller.packageName == callingPackage && caller.className == "com.elon.quant.grids.okx.OkxGridsActivity")
        BinanceHostCaller.trusted(this, packageManager.getApplicationInfo(caller.packageName, 0).uid)
    }.getOrDefault(false)
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState); setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)
        if (savedInstanceState != null || !trusted()) return finish()
        if (intent.data != null || intent.clipData != null || intent.selector != null || intent.extras?.keySet() != setOf("nonce")) return finish()
        nonce = intent.getStringExtra("nonce")?.takeIf { Regex("[a-f0-9]{64}").matches(it) } ?: return finish()
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL; setBackgroundColor(ui.background)
            setPadding(ui.dp(18), ui.dp(12), ui.dp(18), ui.dp(12))
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO_EXCLUDE_DESCENDANTS
        }
        root.addView(ui.label("连接欧易", 24f))
        root.addView(ui.label("全球站 · 正式账户 · 仅查看合约网格", 13f).apply { setTextColor(ui.muted) })
        val fields = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; isSaveEnabled = false }
        fields.addView(ui.label("请在欧易 API 管理创建仅有 Read 权限的授权，再填写以下三项。", 15f))
        fun input(label: String, id: String): EditText {
            fields.addView(ui.label(label, 14f))
            return ui.field(EditText(this)).apply {
                contentDescription = id; isSaveEnabled = false; setSingleLine(true)
                inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
                imeOptions = android.view.inputmethod.EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING
                filterTouchesWhenObscured = true
                addTextChangedListener(object : android.text.TextWatcher {
                    override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
                    override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) {
                        verified = null; if (::approve.isInitialized) approve.visibility = View.GONE
                    }
                    override fun afterTextChanged(s: android.text.Editable?) = Unit
                })
                fields.addView(this)
            }
        }
        key = input("API Key", "okx-api-key"); secret = input("Secret Key", "okx-secret"); passphrase = input("Passphrase", "okx-passphrase")
        status = ui.label("授权资料加密保存在本机主应用，量化只接收网格数据。", 13f).apply { contentDescription = "okx-connect-status" }
        fields.addView(status)
        verify = ui.button("验证只读授权", "okx-verify", true) { verify() }; fields.addView(verify)
        approve = ui.button("同意并保持只读连接", "okx-approve", true) { confirmReadAccess() }.apply { visibility = View.GONE }
        fields.addView(approve)
        fields.addView(ui.label("连接后返回量化查看网格；可在量化随时断开并删除本机授权资料。本入口不支持模拟或地区账户切换。", 13f).apply { setTextColor(ui.muted) })
        root.addView(ScrollView(this).apply { isSaveEnabled = false; addView(fields) }, LinearLayout.LayoutParams(-1, 0, 1f))
        root.addView(ui.button("取消并返回量化", "okx-connect-cancel") { finish() })
        setContentView(root)
    }
    private fun busy(value: Boolean) {
        listOf(key, secret, passphrase, verify, approve).forEach { it.isEnabled = !value }
        verify.text = if (value) "正在验证…" else "验证只读授权"
    }
    private fun verify() {
        if (!hasWindowFocus() || !trusted()) return
        val credentials = runCatching { OkxCredentials(key.text.toString(), secret.text.toString(), passphrase.text.toString()) }
            .getOrElse { status.text = "请完整填写 API Key、Secret Key 和 Passphrase。"; return }
        val ticket = ++epoch; verified = null; approve.visibility = View.GONE; busy(true)
        worker.execute {
            val result = runCatching { OkxReadHost.get(this).verify(credentials) }
            runOnUiThread {
                if (isFinishing || isDestroyed || ticket != epoch) return@runOnUiThread
                busy(false)
                result.onSuccess {
                    verified = it; status.text = "已核验${if (it.kind == "sub") "欧易子账户" else "欧易主账户"}，授权仅有读取权限。"
                    approve.visibility = View.VISIBLE
                }.onFailure { status.text = message(it) }
            }
        }
    }
    private fun confirmReadAccess() {
        val candidate = verified ?: return
        if (!hasWindowFocus() || !trusted()) return
        val ticket = ++epoch; busy(true)
        worker.execute {
            val result = runCatching { OkxReadHost.get(this).approve(candidate) }
            runOnUiThread {
                if (isFinishing || isDestroyed || ticket != epoch) return@runOnUiThread
                result.onSuccess { grant ->
                    setResult(RESULT_OK, Intent().putExtra("nonce", nonce).putExtra("schema", OkxReadProtocol.SCHEMA).putExtra("grant", grant)); finish()
                }.onFailure { busy(false); status.text = message(it) }
            }
        }
    }
    private fun message(error: Throwable) = when ((error as? OkxReadException)?.reason) {
        OkxReadFailure.HOST_SESSION_REQUIRED -> "请先登录主应用的一龙账号，再返回量化连接。"
        OkxReadFailure.READ_ONLY_KEY_REQUIRED -> "请使用仅有 Read 权限的 API 授权，取消 Trade 和 Withdraw 权限后再试。"
        OkxReadFailure.INVALID_CREDENTIALS -> "授权验证失败，请核对三项资料、IP 限制以及全球正式账户环境。"
        OkxReadFailure.CLOCK_SKEW -> "手机时间与欧易不一致，请开启自动日期和时间后重试。"
        OkxReadFailure.RATE_LIMITED -> "欧易暂时限制请求，请稍后重试。"
        OkxReadFailure.CONNECTION_CHANGED -> "一龙登录状态已变化，请返回量化重新连接。"
        else -> "连接未完成，请检查网络或授权资料后重试。"
    }
    override fun dispatchTouchEvent(event: MotionEvent): Boolean {
        if (event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED) != 0) return true
        return super.dispatchTouchEvent(event)
    }
    override fun onNewIntent(intent: Intent?) { super.onNewIntent(intent); finish() }
    override fun onDestroy() {
        epoch++; verified = null
        if (::key.isInitialized) listOf(key, secret, passphrase).forEach { it.text.clear() }
        worker.shutdownNow(); super.onDestroy()
    }
}
