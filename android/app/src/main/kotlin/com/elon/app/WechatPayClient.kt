// WechatPayClient.kt
// module: payment | layer: infrastructure | role: 微信 App 支付客户端
// summary: 封装"创建订单→拉起微信收银台→接收结果"完整流程

package com.elon.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.os.Build
import android.util.Log
import com.tencent.mm.opensdk.modelpay.PayReq
import com.tencent.mm.opensdk.openapi.WXAPIFactory
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import org.json.JSONObject
import java.io.OutputStreamWriter
import java.net.HttpURLConnection
import java.net.URL
import kotlin.coroutines.resume

/**
 * 微信 App 支付客户端。
 *
 * 用法示例（在 Activity / Fragment 中）：
 * ```kotlin
 * val result = WechatPayClient.pay(context, serverUrl, token, amountFen = 500)
 * when (result) {
 *     is PayResult.Success -> toast("支付成功，充值 ¥${result.amountYuan}")
 *     is PayResult.Cancelled -> toast("已取消")
 *     is PayResult.Failed -> toast("支付失败: ${result.message}")
 * }
 * ```
 *
 * **注意**：微信 App 支付要求 APK 使用正式签名，并在微信开放平台注册了对应的签名 MD5。
 */
object WechatPayClient {

    private const val TAG = "WechatPayClient"

    /**
     * 微信开放平台 AppID — 与服务端 WECHAT_APP_ID 环境变量保持一致。
     * 投入生产时修改为真实 AppID。
     */
    const val WX_APP_ID = "wx_placeholder_appid"

    /** 支付结果广播 Action（由 WXPayEntryActivity 发出）。 */
    const val ACTION_PAY_RESULT = "com.elon.app.PAY_RESULT"

    // ── 支付结果封装 ──────────────────────────────────────────────────────────

    sealed class PayResult {
        /** 支付成功 */
        data class Success(val amountFen: Long) : PayResult()
        /** 用户主动取消 */
        object Cancelled : PayResult()
        /** 支付失败或未安装微信 */
        data class Failed(val message: String) : PayResult()
    }

    // ── 入口：创建订单 + 拉起微信 + 等待结果 ─────────────────────────────────

    /**
     * 完整支付流程（挂起函数，在协程中调用）。
     *
     * @param context       Activity 上下文（用于注册广播、拉起微信）
     * @param serverBaseUrl 服务器地址（如 "http://43.139.149.158:8080"）
     * @param authToken     用户 JWT Token
     * @param amountFen     充值金额（分，最低 100 = 1元）
     * @param description   商品描述（可选）
     */
    suspend fun pay(
        context: Context,
        serverBaseUrl: String,
        authToken: String,
        amountFen: Long,
        description: String = "一龙AI余额充值",
    ): PayResult {
        // 1. 检查微信是否安装
        val wxApi = WXAPIFactory.createWXAPI(context, WX_APP_ID, true)
        if (!wxApi.isWXAppInstalled) {
            return PayResult.Failed("未安装微信，无法完成支付")
        }

        // 2. 向服务端创建订单，获取签名参数
        val payParamsResult = createOrderFromServer(serverBaseUrl, authToken, amountFen, description)
        val payParams = when (payParamsResult) {
            is ServerOrderResult.Error -> return PayResult.Failed(payParamsResult.message)
            is ServerOrderResult.Ok -> payParamsResult.params
        }

        // 3. 构造微信 PayReq，拉起收银台
        val req = PayReq().apply {
            appId       = payParams.appid
            partnerId   = payParams.partnerid
            prepayId    = payParams.prepayid
            packageValue = payParams.`package`
            nonceStr    = payParams.noncestr
            timeStamp   = payParams.timestamp
            sign        = payParams.sign
        }

        if (!wxApi.sendReq(req)) {
            return PayResult.Failed("拉起微信失败，请确认已安装微信并授权")
        }

        // 4. 等待 WXPayEntryActivity 发来广播结果（超时 5 分钟）
        return waitForPayResult(context, amountFen)
    }

    // ── 内部：向服务端创建订单 ────────────────────────────────────────────────

    private sealed class ServerOrderResult {
        data class Ok(val params: AppPayParams) : ServerOrderResult()
        data class Error(val message: String) : ServerOrderResult()
    }

    private data class AppPayParams(
        val appid: String,
        val partnerid: String,
        val prepayid: String,
        val `package`: String,
        val noncestr: String,
        val timestamp: String,
        val sign: String,
    )

    private suspend fun createOrderFromServer(
        baseUrl: String,
        token: String,
        amountFen: Long,
        description: String,
    ): ServerOrderResult = withContext(Dispatchers.IO) {
        try {
            val url = URL("${baseUrl.trimEnd('/')}/api/me/pay/create_order")
            val conn = url.openConnection() as HttpURLConnection
            conn.requestMethod = "POST"
            conn.setRequestProperty("Content-Type", "application/json; charset=utf-8")
            conn.setRequestProperty("Authorization", "Bearer $token")
            conn.connectTimeout = 15_000
            conn.readTimeout = 15_000
            conn.doOutput = true

            val body = """{"amount_fen":$amountFen,"description":"${description.replace("\"", "\\\"")}"}"""
            OutputStreamWriter(conn.outputStream, Charsets.UTF_8).use { it.write(body) }

            val respCode = conn.responseCode
            val respText = conn.inputStream.bufferedReader().readText()

            if (respCode !in 200..299) {
                val errMsg = runCatching { JSONObject(respText).getString("error") }
                    .getOrDefault("服务器错误 $respCode")
                return@withContext ServerOrderResult.Error(errMsg)
            }

            val json = JSONObject(respText)
            val p = json.getJSONObject("pay_params")
            ServerOrderResult.Ok(AppPayParams(
                appid      = p.getString("appid"),
                partnerid  = p.getString("partnerid"),
                prepayid   = p.getString("prepayid"),
                `package`  = p.getString("package"),
                noncestr   = p.getString("noncestr"),
                timestamp  = p.getString("timestamp"),
                sign       = p.getString("sign"),
            ))
        } catch (e: Exception) {
            Log.e(TAG, "createOrderFromServer error", e)
            ServerOrderResult.Error("网络请求失败: ${e.message}")
        }
    }

    // ── 内部：等待支付结果广播 ────────────────────────────────────────────────

    private suspend fun waitForPayResult(
        context: Context,
        amountFen: Long,
    ): PayResult = suspendCancellableCoroutine { cont ->
        val receiver = object : BroadcastReceiver() {
            override fun onReceive(ctx: Context?, intent: Intent?) {
                if (intent?.action != ACTION_PAY_RESULT) return
                context.unregisterReceiver(this)

                val errCode = intent.getIntExtra("err_code", -1)
                val errStr  = intent.getStringExtra("err_str") ?: ""
                val result = when (errCode) {
                    0    -> PayResult.Success(amountFen)
                    -2   -> PayResult.Cancelled
                    else -> PayResult.Failed("支付失败(errCode=$errCode): $errStr")
                }
                if (cont.isActive) cont.resume(result)
            }
        }

        val filter = IntentFilter(ACTION_PAY_RESULT)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            context.registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            @Suppress("UnspecifiedRegisterReceiverFlag")
            context.registerReceiver(receiver, filter)
        }

        cont.invokeOnCancellation {
            runCatching { context.unregisterReceiver(receiver) }
        }
    }
}
