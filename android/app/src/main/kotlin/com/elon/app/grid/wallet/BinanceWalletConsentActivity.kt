package com.elon.app.grid.wallet

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
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
    private val gate=BinanceWalletConsentGate(SystemClock::elapsedRealtime)
    private val handler=Handler(Looper.getMainLooper())
    private val deadline=Runnable {render()}
    private var resumed=false
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
        body.addView(ui.button("重新识别账户","binance-wallet-retry"){gate.cancel();attach(retry=true)})
        body.addView(ui.button("币安官网登录／查看账户","binance-wallet-official"){
            gate.cancel()
            host?.view?.let {view->
                if(view.parent===official)official.removeView(view)
                else {(view.parent as? ViewGroup)?.removeView(view);official.addView(view,FrameLayout.LayoutParams(-1,ui.dp(520)))}
            }
            render()
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
        }}.onFailure {gate.cancel("暂时无法识别，请检查币安登录或稍后重试。")}
        render()
    }
    private fun identity():BinanceWalletConsentGate.Identity? {
        val runtime=host?:return null
        if(!runtime.live())return null
        return BinanceWalletConsentGate.Identity(runtime.owner()?:return null,
            runtime.wallet.state.account?:return null,runtime.wallet.state.accountKind)
    }
    private fun render() {
        handler.removeCallbacks(deadline)
        if(!::status.isInitialized||isFinishing)return
        val runtime=host
        if(runtime==null) {
            approve.isEnabled=false
            status.text=gate.notice?:"正在连接币安账户…"
            return
        }
        val state=runtime.wallet.state
        val ready=runtime.live()&&state.identityFresh()&&!state.identifying
        when(val decision=gate.observe(identity(),ready,state.identifying,state.status=="error")) {
            is BinanceWalletConsentGate.Decision.Approve -> {completeApproval(decision.identity);return}
            else -> Unit
        }
        approve.isEnabled=resumed&&hasWindowFocus()&&!gate.waiting&&!state.identifying&&identity()!=null
        approve.text=when {gate.waiting->"正在重新确认账户…";ready->"同意并保持钱包只读连接";else->"重新确认账户并同意"}
        status.text=gate.notice?:when {
            gate.waiting->"正在重新确认当前账户，确认一致后将返回量化。"
            ready->when(state.accountKind){"sub"->"已确认：币安子账户";"primary"->"已确认：币安主账户";else->"已确认当前币安账户"}
            state.status=="error"->"账户识别暂不可用，请确认币安登录后重试。"
            state.identifying->"正在确认当前币安账户…"
            identity()!=null->"账户确认已过期。点击下方按钮会重新确认账户并继续本次授权。"
            else->"尚未确认账户，请重新识别或检查币安登录。"
        }
        if(resumed&&hasWindowFocus()) {
            val delay=if(gate.waiting)gate.remaining() else state.identityRemainingMs()
            if(delay>0)handler.postDelayed(deadline,delay)
        }
    }
    private fun approve() {
        if(!resumed||!hasWindowFocus()||!BinanceHostCaller.activity(this))return
        val runtime=host?:return
        val state=runtime.wallet.state
        when(val decision=gate.click(identity(),state.identityFresh(),state.identifying)) {
            is BinanceWalletConsentGate.Decision.Approve -> completeApproval(decision.identity)
            BinanceWalletConsentGate.Decision.Refresh -> attach(retry=true)
            BinanceWalletConsentGate.Decision.None -> render()
        }
    }
    private fun completeApproval(expected:BinanceWalletConsentGate.Identity) {
        if(!resumed||!hasWindowFocus()||!BinanceHostCaller.activity(this)||identity()!=expected) {
            gate.cancel("账户或页面状态发生变化，请核对后重新同意。");render();return
        }
        val grant=runCatching {host?.wallet?.approve()}.getOrElse {
            gate.cancel("授权未完成，请重新确认账户后再试。");render();return
        }?:return render()
        setResult(RESULT_OK,Intent().putExtra("nonce",nonce).putExtra("grant",grant).putExtra("schema","yilong.binance_wallet_grant.v1"));finish()
    }
    override fun onResume() {
        super.onResume();resumed=true;gate.setActive(hasWindowFocus())
        if(!isFinishing&&::status.isInitialized)attach(retry=true)
    }
    override fun onPause() {
        resumed=false;gate.setActive(false);handler.removeCallbacks(deadline);super.onPause()
    }
    override fun onWindowFocusChanged(hasFocus:Boolean) {
        super.onWindowFocusChanged(hasFocus);gate.setActive(resumed&&hasFocus);render()
    }
    override fun onNewIntent(intent:Intent?){super.onNewIntent(intent);finish()}
    override fun onSaveInstanceState(outState:Bundle){super.onSaveInstanceState(outState);outState.clear()}
    override fun dispatchTouchEvent(event:MotionEvent):Boolean {
        if(event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED)!=0)return true
        return super.dispatchTouchEvent(event)
    }
    override fun onDestroy() {
        gate.setActive(false);handler.removeCallbacks(deadline)
        host?.let {runtime->runtime.wallet.onChanged=null;runtime.view?.let {if(::official.isInitialized&&it.parent===official)official.removeView(it)}}
        host=null;super.onDestroy()
    }
}
