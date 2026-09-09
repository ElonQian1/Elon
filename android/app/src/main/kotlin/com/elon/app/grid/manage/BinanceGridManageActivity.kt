package com.elon.app.grid.manage

import android.app.Activity
import android.app.AlertDialog
import android.content.Intent
import android.os.Bundle
import android.os.SystemClock
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.*
import com.elon.app.grid.create.BinanceCreateJournal
import com.elon.app.grid.create.BinanceCreateSlot
import com.elon.app.grid.host.BinanceHostCaller
import com.elon.app.grid.host.BinanceHostRuntime

/** Explicit, visible user operation; exported only for the verified official quant caller. */
open class BinanceGridManageActivity : Activity() {
    protected open val protocolVersion=1
    private val state=BinanceManageState(SystemClock::elapsedRealtime)
    private val journal by lazy {BinanceCreateJournal(this,"binance-manage-attempt-v1.json")}
    private var host:BinanceHostRuntime?=null
    private var session:BinanceManageSession?=null
    private var nonce=""
    private var ownsSlot=false
    private var ready=false
    private var corrupt=false
    private var creationPending=false
    private var resolvedRecord=false
    private var resumed=false
    private val appearance by lazy{BinanceManageAppearance(this)}
    private val readEndpoint=object:BinanceManageReadEndpoint {
        override fun readFacts()=debugFacts()
        override fun readCommand(request:BinanceManageReadRequest)=debugCommand(request)
    }
    private lateinit var form:BinanceManageForm
    private lateinit var status:TextView
    private lateinit var summary:TextView
    private lateinit var confirm:CheckBox
    private lateinit var submit:Button
    private lateinit var prepare:Button
    private lateinit var read:Button
    private lateinit var resolve:Button
    private lateinit var official:FrameLayout
    override fun onCreate(savedInstanceState:Bundle?) {
        super.onCreate(savedInstanceState);setResult(RESULT_CANCELED)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        if(savedInstanceState!=null || !BinanceHostCaller.activity(this) || intent.data!=null || intent.clipData!=null || intent.selector!=null || intent.extras?.keySet()!=setOf("nonce")) return finish()
        nonce=intent.getStringExtra("nonce")?.takeIf{Regex("[a-f0-9]{64}").matches(it)} ?: return finish()
        if(!BinanceCreateSlot.shared.acquire(this)) return finish()
        ownsSlot=true
        runCatching{journal.read()?.let(state::restore)}.onFailure{corrupt=true}
        creationPending=runCatching{BinanceCreateJournal(this).read()!=null}.getOrDefault(true)
        val root=LinearLayout(this).apply{orientation=LinearLayout.VERTICAL;setPadding(appearance.dp(16),appearance.dp(12),appearance.dp(16),appearance.dp(10));setBackgroundColor(appearance.background)}
        root.addView(label("管理本人币安 U 本位网格",21f))
        status=label("正在读取本人账号",15f);root.addView(status)
        root.addView(button("重新连接币安／更新列表","binance-manage-reload"){act{reconnect()}})
        val content=LinearLayout(this).apply{orientation=LinearLayout.VERTICAL;isSaveEnabled=false}
        form=BinanceManageForm(this,protocolVersion>=2,protocolVersion>=3){if(ready && !state.unresolved){session?.cancel();confirm.isChecked=false;render()}}
        content.addView(form.root)
        read=button("读取当前策略／查询本次结果","binance-manage-read") {
            act{readCurrent()}
        };content.addView(read)
        prepare=button("检查本次管理操作","binance-manage-prepare") {
            act{confirm.isChecked=false;session?.prepare(form.id() ?: error("请选择策略"),form.action(),form.cps(),form.investmentDelta(),form.rangeDraft())}
        };content.addView(prepare)
        summary=label("",16f);content.addView(summary)
        confirm=CheckBox(this).apply {
            text="我已核对本账号、策略及下方操作参数；本次由我提交真实操作。"
            isSaveEnabled=false;filterTouchesWhenObscured=true;contentDescription="binance-manage-confirm"
            setTextColor(appearance.text)
            setOnCheckedChangeListener{_,_->if(ready)render()}
        };content.addView(confirm)
        submit=button("确认执行本次操作","binance-manage-submit") {
            if(hasWindowFocus() && BinanceHostCaller.activity(this) && confirm.isChecked && !corrupt && !creationPending) act{session?.submit()}
        };content.addView(submit)
        resolve=button("已在官网核对，结束本机记录","binance-manage-resolve"){acknowledge()};content.addView(resolve)
        content.addView(button("全屏币安官网与高级功能","binance-manage-official"){
            if(state.status!="submitting") {
                session?.cancel();confirm.isChecked=false
                com.elon.app.grid.ui.showBinanceOfficialPanel(this,host)
            }
        })
        official=FrameLayout(this).apply{visibility=View.GONE};content.addView(official,LinearLayout.LayoutParams(-1,(resources.displayMetrics.density*520).toInt()))
        content.addView(label(if(protocolVersion>=3)"当前区间编辑适用于普通非追踪网格；收益金额止盈止损、仓位保证金和高级编辑可进入官网。结束状态不证明仓位归零或资金结清。" else "区间、格数、仓位保证金和止盈止损可进入全屏官网操作。结束状态不证明挂单已撤销、仓位归零或资金结清。",14f))
        root.addView(ScrollView(this).apply{isSaveEnabled=false;addView(content)},LinearLayout.LayoutParams(-1,0,1f))
        root.addView(button("返回量化应用","binance-manage-return"){returnResult()})
        setContentView(root);ready=true;BinanceManageReadBridge.bind(readEndpoint)
        runCatching{connectPage(false)}.onFailure{status.text="官网连接未就绪，请检查系统WebView。"}
        render()
    }
    private fun connectPage(refresh:Boolean):Boolean=BinanceHostRuntime.onMain(this){runtime->
        host=runtime
        reconnectBinanceManagePage(state.status=="submitting",{session?.cancel();confirm.isChecked=false},
            {runtime.view},{runtime.begin()},{view->
                if(session==null)session=BinanceManageSession(runtime,state,::persist,::render)
                runtime.onCreateObservation={session?.observed(it)};runtime.onChanged=::render
                if(view.parent!==official){(view.parent as? ViewGroup)?.removeView(view);official.removeAllViews();official.addView(view,FrameLayout.LayoutParams(-1,-1))}
            },{view->if(refresh)view.loadUrl(BinanceHostRuntime.ENTRY)})
    }
    private fun reconnect():Boolean {
        val connected=connectPage(true)
        if(connected)official.visibility=View.VISIBLE
        render();return connected
    }
    private fun persist():Boolean = if(resolvedRecord)true else if(state.unresolved)journal.save(state.journal()) else if(!corrupt)journal.save(null) else false
    private fun act(action:()->Unit) {runCatching(action).onFailure{status.text=it.message ?: "操作未完成"}}
    private fun readGate():BinanceManageReadGate {
        val current=host?.live()==true && host?.state?.fresh()==true
        return BinanceManageReadGate(current,if(current)host?.state?.count ?: 0 else 0,
            form.id()!=null,state.unresolved,host?.state?.account==state.account,corrupt || creationPending,
            session?.busy!=false,state.status=="prepared")
    }
    private fun readCurrent() {
        require(readGate().readEnabled){"请等待列表核验并选择已有策略；空列表没有可查询的策略详情。"}
        confirm.isChecked=false;session?.read(if(state.unresolved)state.id else form.id() ?: error("请选择策略"))
    }
    private fun debugFacts():Map<String,Any?> {
        val gate=readGate();val trace=session?.readTrace
        return mapOf("page_open" to ready,"page_resumed" to (resumed && hasWindowFocus()),
            "list_state" to gate.listState,"row_count" to gate.count,"strategy_selected" to gate.selected,
            "read_enabled" to gate.readEnabled,"prepare_enabled" to gate.prepareEnabled,
            "operation_phase" to state.status,"unresolved" to state.unresolved,"busy" to gate.busy,
            "read_sequence" to (trace?.sequence ?: 0L),"read_outcome" to (trace?.outcome ?: "idle"),
            "read_reason" to (trace?.reason ?: "none"),
            "range_details_available" to (state.snapshot?.range!=null && state.snapshot?.investment!=null && session?.detailCurrent(form.id())==true),
            "detail_current" to (session?.detailCurrent(if(state.unresolved)state.id else form.id())==true))
    }
    private fun debugCommand(request:BinanceManageReadRequest):String {
        if(!ready || !resumed || !hasWindowFocus())return "page_not_foreground"
        render()
        if(!readGate().permits(request.action))return "read_action_unavailable"
        return runCatching {
            when(request.action) {
                "select"->if(form.selectIndex(request.index ?: -1)){render();"selected"}else "index_unavailable"
                "read"->{readCurrent();"read_started"}
                "reload"->if(reconnect())"reload_started" else "reload_unavailable"
                else->"unsupported_action"
            }
        }.getOrDefault("read_action_unavailable")
    }
    private fun render() {
        if(!ready)return
        val h=host
        val blocked=state.unresolved || corrupt || creationPending
        val same=h?.live()==true && h.state.account==state.account
        val kind=when(h?.state?.accountKind){"sub"->"币安子账户";"primary"->"币安主账户";else->"账号未确认"}
        status.text="$kind${h?.state?.account?.takeLast(6)?.let{" · 本机标记 $it"} ?: ""}\n${h?.status ?: "官网未连接"}\n${session?.message ?: "请确认登录"}"
        form.root.visibility=if(blocked)View.GONE else View.VISIBLE
        form.refresh(h?.state?.managementChoices().orEmpty(),h?.live()==true && h.state.fresh())
        val gate=readGate()
        read.isEnabled=gate.readEnabled
        prepare.isEnabled=gate.prepareEnabled
        confirm.visibility=if(state.status=="prepared" && !blocked)View.VISIBLE else View.GONE
        submit.visibility=confirm.visibility;submit.isEnabled=confirm.isChecked && !blocked && session?.canSubmit()==true
        val s=state.snapshot?.takeIf{state.unresolved || it.id==form.id()}
        form.updateRange(s?.range)
        summary.text=when {
            creationPending->"存在未核对的创建记录，请先返回创建页核对并结束本机记录。"
            corrupt->"本机管理记录无法恢复，请先在官网核对，系统不会重发。"
            state.unresolved && !same->"请登录原账号后查询本次记录；不会在另一账号重新执行。"
            !state.unresolved && gate.listState=="empty"->"当前账号本次列表暂无网格。连接和账号核验已成功，读取详情需要先有策略。新建后可重新加载列表。"
            !state.unresolved && gate.listState=="unverified"->"正在等待账号与网格列表核验；这不代表账号没有网格。"
            !state.unresolved && form.id()==null->"已读取 ${gate.count} 条网格，请先选择策略，再读取详情。"
            else->buildString {
                if(state.unresolved)append("本次${actionName()}：${state.status}\n策略编号：${state.id}\n币安状态：${state.providerStatus.ifEmpty{"待查询"}}\n")
                if(s!=null)append("${s.symbol} · ${s.id}\n当前状态：${s.status}\n终止时：${mode(s.cps)}\n取消合约委托：${if(s.cos)"是，请核对作用范围" else "否，请自行核对委托"}\n")
                if(s?.investment!=null)append("参考累计投入：${s.investment.invested()} USDT\n")
                if(s?.range!=null)append("当前区间：${s.range.lower} ～ ${s.range.upper} · ${s.range.count}格\n")
                if(state.status=="prepared") {
                    append("\n本次：${actionName()}\n")
                    if(state.action=="range")state.rangeDraft?.let { draft->
                        append("新区间：${draft.lower} ～ ${draft.upper} · ${draft.count}格\n")
                        append("现有仓位：${mode(draft.closePositions)}\n追加投入：${draft.investmentDelta} USDT\n")
                        append("原价格止盈止损保持：下限 ${s?.range?.preserved?.get("stopLowerLimit") ?: "未设置"}，上限 ${s?.range?.preserved?.get("stopUpperLimit") ?: "未设置"}。\n")
                        append("币安会重建订单并校验参数和资金；最低追加金额尚未估算，结果不明时不会自动重发。")
                    }
                    else if(state.action=="investment")append("追加：${state.investmentDelta} USDT\n追加后参考累计投入：${state.investmentPreview()} USDT\n币安将在提交时检查可用余额和风险限额。追加投入不等于设置亏损上限。")
                    else append("已确认处理：${mode(state.cps)}\n${if(state.action=="settings")"仅更改终止处理；不立即结束或平仓。分享和跟踪设置保持当前值。" else "按当前已确认设置结束；不会先自动修改设置。"}")
                }
            }
        }
        resolve.visibility=if(!creationPending && (corrupt || state.unresolved) && state.status!="submitting")View.VISIBLE else View.GONE
        resolve.isEnabled=corrupt || same
    }
    private fun acknowledge() {
        if(!hasWindowFocus() || state.status=="submitting" || (!corrupt && host?.state?.account!=state.account))return
        AlertDialog.Builder(this).setTitle("结束本机核对记录")
            .setMessage("请先在官网核对策略、挂单和仓位。这不会撤单、结束网格或平仓，也不能证明交易所没有执行本次操作。")
            .setNegativeButton("继续核对",null).setPositiveButton("我已核对"){_,_->if(journal.save(null)){resolvedRecord=true;returnResult()}}
            .show().window?.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
    }
    private fun returnResult() {
        session?.close()
        val same=host?.live()==true && host?.state?.account==state.account
        val legacyPending=(protocolVersion==1 && state.action=="investment") || (protocolVersion<3 && state.action=="range")
        val result=if(corrupt || (!same && state.unresolved) || legacyPending)"unknown" else state.status.takeIf{it in setOf("accepted","observed","unknown","rejected")} ?: "not_sent"
        if(BinanceHostCaller.activity(this))setResult(RESULT_OK,Intent().putExtra("schema","yilong.binance_manage_result.v$protocolVersion").putExtra("nonce",nonce)
            .putExtra("status",result).putExtra("action",if(legacyPending)"" else state.action).putExtra("strategy_id",if(same)state.id else "").putExtra("provider_status",if(same)state.providerStatus else ""))
        finish()
    }
    override fun onResume(){super.onResume();resumed=true}
    override fun onUserInteraction(){super.onUserInteraction();host?.keepAlive()}
    override fun onPause(){resumed=false;if(ready && state.status!="submitting"){session?.cancel();confirm.isChecked=false};super.onPause()}
    override fun onSaveInstanceState(outState:Bundle){super.onSaveInstanceState(outState);outState.clear()}
    @Deprecated("Deprecated in Java") override fun onBackPressed()=returnResult()
    override fun dispatchTouchEvent(event:MotionEvent):Boolean {
        if(event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED)!=0)return true
        return super.dispatchTouchEvent(event)
    }
    override fun onDestroy(){if(ownsSlot){session?.close();BinanceManageReadBridge.unbind(readEndpoint);host?.let{it.onChanged=null;it.onCreateObservation=null;it.view?.let{v->(v.parent as? ViewGroup)?.removeView(v)}};BinanceCreateSlot.shared.release(this)};super.onDestroy()}
    private fun mode(value:Boolean)=if(value)"按市价平仓" else "保留仓位，需要自行处理"
    private fun actionName()=when(state.action){"range"->"修改区间和格数";"investment"->"追加策略投入";"close"->"结束网格";else->"修改终止处理设置"}
    private fun label(value:String,size:Float)=appearance.label(value,size)
    private fun button(value:String,id:String,action:()->Unit)=appearance.button(value,id,action)
}
