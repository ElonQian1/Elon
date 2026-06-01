// wxapi/WXPayEntryActivity.kt
// module: payment | layer: presentation | role: 微信支付回调 Activity
// summary: 微信 App Pay 支付结果回调入口（固定包名 {applicationId}.wxapi）

package com.elon.app.wxapi

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import com.tencent.mm.opensdk.constants.ConstantsAPI
import com.tencent.mm.opensdk.modelbase.BaseReq
import com.tencent.mm.opensdk.modelbase.BaseResp
import com.tencent.mm.opensdk.openapi.IWXAPIEventHandler
import com.tencent.mm.opensdk.openapi.WXAPIFactory

/**
 * 微信支付结果回调 Activity。
 *
 * 必须置于 {applicationId}.wxapi 包内，且在 AndroidManifest 中声明
 * android:exported="true"，微信才能正确回调。
 *
 * 支付结果通过广播 [WechatPayClient.ACTION_PAY_RESULT] 通知业务层。
 */
class WXPayEntryActivity : Activity(), IWXAPIEventHandler {

    private val wxApi by lazy {
        WXAPIFactory.createWXAPI(this, WechatPayClient.WX_APP_ID, false)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // 不设置 ContentView，支付结果处理完即关闭
        wxApi.handleIntent(intent, this)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        wxApi.handleIntent(intent, this)
    }

    // 微信请求（本 App 不需要处理来自微信的主动请求）
    override fun onReq(req: BaseReq?) {}

    // 微信支付结果回调
    override fun onResp(resp: BaseResp?) {
        val errCode = resp?.errCode ?: -1
        val outTradeNo = "" // 商户订单号需从本地存储读取（WechatPayClient 保存）

        sendBroadcast(
            Intent(WechatPayClient.ACTION_PAY_RESULT).apply {
                putExtra("err_code", errCode)
                putExtra("err_str", resp?.errStr ?: "")
                // err_code: 0=成功, -1=错误, -2=用户取消
            }
        )
        finish()
    }
}
