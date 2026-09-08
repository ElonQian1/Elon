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
class BinanceGridManageActivity : Activity() {
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
        val root=LinearLayout(this).apply{orientation=LinearLayout.VERTICAL;setPadding(20,20,20,12)}
        root.addView(label("管理本人币安 U 本位网格",21f))
        status=label("正在读取本人账号",15f);root.addView(status)
        val content=LinearLayout(this).apply{orientation=LinearLayout.VERTICAL;isSaveEnabled=false}
        form=BinanceManageForm(this){if(ready && !state.unresolved){session?.cancel();confirm.isChecked=false;render()}}
        content.addView(form.root)
        read=button("读取当前策略／查询本次结果","binance-manage-read") {
            act{confirm.isChecked=false;session?.read(if(state.unresolved)state.id else form.id() ?: error("请选择策略"))}
        };content.addView(read)
        prepare=button("检查本次管理操作","binance-manage-prepare") {
            act{confirm.isChecked=false;session?.prepare(form.id() ?: error("请选择策略"),form.action(),form.cps())}
        };content.addView(prepare)
        summary=label("",16f);content.addView(summary)
        confirm=CheckBox(this).apply {
            text="我已核对本账号、策略编号及仓位处理；本次由我提交真实操作。"
            isSaveEnabled=false;filterTouchesWhenObscured=true;contentDescription="binance-manage-confirm"
            setOnCheckedChangeListener{_,_->if(ready)render()}
        };content.addView(confirm)
        submit=button("确认执行本次操作","binance-manage-submit") {
            if(hasWindowFocus() && BinanceHostCaller.activity(this) && confirm.isChecked && !corrupt && !creationPending) act{session?.submit()}
        };content.addView(submit)
        resolve=button("已在官网核对，结束本机记录","binance-manage-resolve"){acknowledge()};content.addView(resolve)
        content.addView(button("查看／收起币安官网","binance-manage-official"){
            official.visibility=if(official.visibility==View.VISIBLE)View.GONE else View.VISIBLE
        })
        official=FrameLayout(this).apply{visibility=View.GONE};content.addView(official,LinearLayout.LayoutParams(-1,(resources.displayMetrics.density*520).toInt()))
        content.addView(button("重新加载官网并确认登录","binance-manage-reload"){
            if(state.status!="submitting") {session?.cancel();confirm.isChecked=false;official.visibility=View.VISIBLE;host?.view?.loadUrl(BinanceHostRuntime.ENTRY)}
        })
        content.addView(label("区间、格数、追加保证金和止盈止损请在官网操作。结束状态不证明挂单已撤销、仓位归零或资金结清。",14f))
        root.addView(ScrollView(this).apply{isSaveEnabled=false;addView(content)},LinearLayout.LayoutParams(-1,0,1f))
        root.addView(button("返回量化应用","binance-manage-return"){returnResult()})
        setContentView(root);ready=true
        runCatching{BinanceHostRuntime.onMain(this){runtime->
            host=runtime
            if(runtime.begin()) {
                session=BinanceManageSession(runtime,state,::persist,::render)
                runtime.onCreateObservation={session?.observed(it)};runtime.onChanged=::render
                runtime.view?.let{view->(view.parent as? ViewGroup)?.removeView(view);official.addView(view,FrameLayout.LayoutParams(-1,-1))}
            }
        }}.onFailure{status.text="官网连接未就绪，请检查系统WebView。"}
        render()
    }
    private fun persist():Boolean = if(resolvedRecord)true else if(state.unresolved)journal.save(state.journal()) else if(!corrupt)journal.save(null) else false
    private fun act(action:()->Unit) {runCatching(action).onFailure{status.text=it.message ?: "操作未完成"}}
    private fun render() {
        if(!ready)return
        val h=host
        val blocked=state.unresolved || corrupt || creationPending
        val same=h?.live()==true && h.state.account==state.account
        val kind=when(h?.state?.accountKind){"sub"->"币安子账户";"primary"->"币安主账户";else->"账号未确认"}
        status.text="$kind${h?.state?.account?.takeLast(6)?.let{" · 本机标记 $it"} ?: ""}\n${h?.status ?: "官网未连接"}\n${session?.message ?: "请确认登录"}"
        form.root.visibility=if(blocked)View.GONE else View.VISIBLE
        form.refresh(h?.state?.managementChoices().orEmpty())
        read.isEnabled=!corrupt && !creationPending && session?.busy==false && h?.state?.fresh()==true && (!state.unresolved || same)
        prepare.isEnabled=!blocked && session?.busy==false && h?.state?.fresh()==true
        confirm.visibility=if(state.status=="prepared" && !blocked)View.VISIBLE else View.GONE
        submit.visibility=confirm.visibility;submit.isEnabled=confirm.isChecked && !blocked && session?.canSubmit()==true
        val s=state.snapshot?.takeIf{state.unresolved || it.id==form.id()}
        summary.text=when {
            creationPending->"存在未核对的创建记录，请先返回创建页核对并结束本机记录。"
            corrupt->"本机管理记录无法恢复，请先在官网核对，系统不会重发。"
            state.unresolved && !same->"请登录原账号后查询本次记录；不会在另一账号重新执行。"
            else->buildString {
                if(state.unresolved)append("本次${if(state.action=="close")"结束" else "设置修改"}：${state.status}\n策略编号：${state.id}\n币安状态：${state.providerStatus.ifEmpty{"未知"}}\n")
                if(s!=null)append("${s.symbol} · ${s.id}\n当前状态：${s.status}\n终止时：${mode(s.cps)}\n取消合约委托：${if(s.cos)"是，请核对作用范围" else "否，请自行核对委托"}\n")
                if(state.status=="prepared")append("本次：${if(state.action=="close")"结束网格" else "修改终止处理设置"}\n已确认处理：${mode(state.cps)}\n${if(state.action=="settings")"仅更改终止处理；不立即结束或平仓。分享和跟踪设置保持当前值。" else "按当前已确认设置结束；不会先自动修改设置。"}")
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
        val result=if(corrupt || (!same && state.unresolved))"unknown" else state.status.takeIf{it in setOf("accepted","observed","unknown","rejected")} ?: "not_sent"
        if(BinanceHostCaller.activity(this))setResult(RESULT_OK,Intent().putExtra("schema","yilong.binance_manage_result.v1").putExtra("nonce",nonce)
            .putExtra("status",result).putExtra("action",state.action).putExtra("strategy_id",if(same)state.id else "").putExtra("provider_status",if(same)state.providerStatus else ""))
        finish()
    }
    override fun onPause(){if(ready && state.status!="submitting"){session?.cancel();confirm.isChecked=false};super.onPause()}
    override fun onSaveInstanceState(outState:Bundle){super.onSaveInstanceState(outState);outState.clear()}
    @Deprecated("Deprecated in Java") override fun onBackPressed()=returnResult()
    override fun dispatchTouchEvent(event:MotionEvent):Boolean {
        if(event.flags and (MotionEvent.FLAG_WINDOW_IS_OBSCURED or MotionEvent.FLAG_WINDOW_IS_PARTIALLY_OBSCURED)!=0)return true
        return super.dispatchTouchEvent(event)
    }
    override fun onDestroy(){if(ownsSlot){session?.close();host?.let{it.onChanged=null;it.onCreateObservation=null;it.view?.let{v->(v.parent as? ViewGroup)?.removeView(v)}};BinanceCreateSlot.shared.release(this)};super.onDestroy()}
    private fun mode(value:Boolean)=if(value)"按市价平仓" else "保留仓位，需要自行处理"
    private fun label(value:String,size:Float)=TextView(this).apply{text=value;textSize=size;isSaveEnabled=false;setPadding(0,8,0,8)}
    private fun button(value:String,id:String,action:()->Unit)=Button(this).apply{text=value;contentDescription=id;isSaveEnabled=false;filterTouchesWhenObscured=true;setOnClickListener{action()}}
}
