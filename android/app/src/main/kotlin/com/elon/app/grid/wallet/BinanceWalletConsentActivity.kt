package com.elon.app.grid.wallet

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.MotionEvent
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.elon.app.grid.host.BinanceHostCaller
import com.elon.app.grid.host.BinanceHostRuntime
import com.elon.app.grid.ui.BinanceGridAppearance

/** Explicit wallet-only consent, separate from the existing grid authorization. */
class BinanceWalletConsentActivity:Activity() {
    private val ui by lazy {BinanceGridAppearance(this)}
    private var host:BinanceHostRuntime?=null
    private var nonce=""
    private lateinit var status:TextView
    private lateinit var approve:Button
    private lateinit var official:FrameLayout
    override fun onCreate(savedInstanceState:Bundle?) {
        super.onCreate(savedInstanceState);setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        if(savedInstanceState!=null || !BinanceHostCaller.activity(this))return finish()
        if(intent.data!=null||intent.clipData!=null||intent.selector!=null||intent.extras?.keySet()!=setOf("nonce","purpose"))return finish()
        if(intent.getStringExtra("purpose")!=BinanceWalletState.PURPOSE)return finish()
        nonce=intent.getStringExtra("nonce")?.takeIf {BinanceWalletState.ID.matches(it)}?:return finish()
        val body=LinearLayout(this).apply {orientation=LinearLayout.VERTICAL;setPadding(ui.dp(20),ui.dp(20),ui.dp(20),ui.dp(20));setBackgroundColor(ui.background)}
        body.addView(ui.label("授权读取币安钱包概览",23f))
        status=ui.label("正在确认当前币安账户…",16f).apply {contentDescription="binance-wallet-consent-status"};body.addView(status)
        body.addView(ui.label("量化应用将查看当前账户各钱包的估值和更新时间。币种持仓明细、登录凭据、交易、划转及提现不在本次授权范围内。",15f))
        body.addView(ui.label("同意后保持只读连接，重新打开量化可自动恢复。你可以在量化资产页撤销；切换账户需要重新授权。现有网格读取授权保持独立。",14f))
        approve=ui.button("同意并保持钱包只读连接","binance-wallet-approve",primary=true){approve()}.apply {isEnabled=false};body.addView(approve)
        body.addView(ui.button("重新识别账户","binance-wallet-retry"){attach(retry=true)})
        body.addView(ui.button("币安官网登录／查看账户","binance-wallet-official"){
            host?.view?.let {view->
                if(view.parent===official)official.removeView(view)
                else {(view.parent as? ViewGroup)?.removeView(view);official.addView(view,FrameLayout.LayoutParams(-1,ui.dp(520)))}
            }
        })
        official=FrameLayout(this);body.addView(official)
        body.addView(ui.button("取消并返回量化","binance-wallet-cancel"){finish()})
        setContentView(ScrollView(this).apply {isSaveEnabled=false;addView(body)})
        attach()
    }
    private fun attach(retry:Boolean=false) {
        runCatching {BinanceHostRuntime.onMain(this){runtime->
            host=runtime;runtime.wallet.onChanged=::render
            runtime.wallet.identify(retry)
        }}.onFailure {status.text="暂时无法识别，请检查币安登录或稍后重试。"}
        render()
    }
    private fun render() {
        val runtime=host?:return;val state=runtime.wallet.state
        val ready=runtime.live()&&state.identityFresh()&&!state.identifying
        approve.isEnabled=ready
        status.text=if(ready)when(state.accountKind){"sub"->"已确认：币安子账户";"primary"->"已确认：币安主账户";else->"已确认当前币安账户"}
            else if(state.status=="error")"账户识别暂不可用，请确认币安登录后重试。" else "正在确认当前币安账户…"
    }
    private fun approve() {
        if(!hasWindowFocus()||!BinanceHostCaller.activity(this))return
        val grant=runCatching {host?.wallet?.approve()}.getOrElse {
            status.text="授权未完成，请重新识别账户后再试。";return
        }?:return render()
        setResult(RESULT_OK,Intent().putExtra("nonce",nonce).putExtra("grant",grant).putExtra("schema","yilong.binance_wallet_grant.v1"));finish()
    }
    override fun onNewIntent(intent:Intent?){super.onNewIntent(intent);finish()}
    override fun onSaveInstanceState(outState:Bundle){super.onSaveInstanceState(outState);outState.clear()}
    override fun dispatchTouchEvent(event:MotionEvent):Boolean {
        if(event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED)!=0)return true
        return super.dispatchTouchEvent(event)
    }
    override fun onDestroy() {
        host?.let {runtime->runtime.wallet.onChanged=null;runtime.view?.let {if(::official.isInitialized&&it.parent===official)official.removeView(it)}}
        host=null;super.onDestroy()
    }
}
