package com.elon.app.grid.manage

import android.app.Activity
import android.view.View
import android.widget.*
import android.text.Editable
import android.text.TextWatcher
import android.text.InputType

internal class BinanceManageForm(private val activity: Activity, private val investmentEnabled:Boolean=false,private val rangeEnabled:Boolean=false, edited: () -> Unit) {
    val root=LinearLayout(activity).apply {orientation=LinearLayout.VERTICAL;isSaveEnabled=false}
    private val appearance=BinanceManageAppearance(activity)
    private val operations=LinearLayout(activity).apply{orientation=LinearLayout.VERTICAL;isSaveEnabled=false}
    private var choices=emptyList<Pair<String,String>>()
    private var verified:Boolean?=null
    private fun select(title: String, id: String, values: List<String>,container:LinearLayout=root): Spinner {
        container.addView(appearance.label(title,14f))
        return Spinner(activity).apply {
            isSaveEnabled=false;contentDescription=id
            adapter=appearance.choices(values)
            container.addView(this)
        }
    }
    private val strategy=select("选择本账号已读取的网格", "binance-manage-strategy", listOf("等待账号与列表核验"))
    private val operation=select("本次操作", "binance-manage-action", listOf("请选择操作","修改终止时平仓设置","结束网格")+
        (if(investmentEnabled)listOf("追加策略投入") else emptyList())+(if(rangeEnabled)listOf("修改区间和格数") else emptyList()),operations)
    private val modeBox=LinearLayout(activity).apply{orientation=LinearLayout.VERTICAL;operations.addView(this)}
    private val mode=select("终止时的仓位处理", "binance-manage-mode", listOf("请选择处理方式","按市价平仓","保留仓位，自行处理"),modeBox)
    private val amountBox=LinearLayout(activity).apply{orientation=LinearLayout.VERTICAL;operations.addView(this)
        addView(appearance.label("追加金额 · USDT",14f))}
    private val amount=EditText(activity).apply {
        isSaveEnabled=false;contentDescription="binance-manage-investment";hint="请输入本次追加金额"
        inputType=InputType.TYPE_CLASS_NUMBER or InputType.TYPE_NUMBER_FLAG_DECIMAL
        setTextColor(appearance.text);setHintTextColor(appearance.muted);backgroundTintList=android.content.res.ColorStateList.valueOf(appearance.accent)
        amountBox.addView(this);amountBox.addView(appearance.label("追加资金用于提高网格投入，可能改变每格下单数量；这不是只调整现有仓位的保证金。",14f))
    }
    private val range=BinanceRangeFields(activity,edited).also{operations.addView(it.root)}
    private fun visibility() {
        val investing=investmentEnabled && operation.selectedItemPosition==3
        modeBox.visibility=if(operation.selectedItemPosition in 1..2)View.VISIBLE else View.GONE
        amountBox.visibility=if(investing)View.VISIBLE else View.GONE
        range.root.visibility=if(rangeEnabled && operation.selectedItemPosition==4)View.VISIBLE else View.GONE
    }
    init {
        root.addView(operations)
        listOf(strategy,operation,mode).forEach {it.onItemSelectedListener=object:AdapterView.OnItemSelectedListener {
            override fun onItemSelected(parent:AdapterView<*>?,view:View?,position:Int,id:Long){visibility();edited()}
            override fun onNothingSelected(parent:AdapterView<*>?)=edited()
        }}
        amount.addTextChangedListener(object:TextWatcher {
            override fun beforeTextChanged(s:CharSequence?,start:Int,count:Int,after:Int){}
            override fun onTextChanged(s:CharSequence?,start:Int,before:Int,count:Int)=edited()
            override fun afterTextChanged(s:Editable?){}
        })
        visibility()
    }
    fun refresh(values: List<Pair<String,String>>,current:Boolean) {
        operations.visibility=if(current && id()!=null)View.VISIBLE else View.GONE
        if(values==choices && verified==current)return
        verified=current
        val previous=id();choices=values
        strategy.isEnabled=current && values.isNotEmpty()
        strategy.adapter=appearance.choices(listOf(if(!current) "等待账号与列表核验" else if(values.isEmpty()) "本次列表暂无网格" else "请选择策略")+values.map{it.second})
        strategy.setSelection(values.indexOfFirst{it.first==previous}+1)
    }
    fun selectIndex(index:Int):Boolean {
        if(!strategy.isEnabled || index !in choices.indices)return false
        strategy.setSelection(index+1);return true
    }
    fun id()=choices.getOrNull(strategy.selectedItemPosition-1)?.first
    fun action()=when(operation.selectedItemPosition){1->"settings";2->"close";3->"investment".also{require(investmentEnabled)};4->"range".also{require(rangeEnabled)};else->error("请选择本次操作")}
    fun cps()=if(action() in setOf("investment","range"))false else when(mode.selectedItemPosition){1->true;2->false;else->error("请选择终止时的仓位处理")}
    fun investmentDelta()=if(action()=="investment")BinanceInvestment.amount(amount.text.toString()) else ""
    fun rangeDraft()=if(action()=="range")range.draft() else null
    fun updateRange(snapshot:BinanceRangeSnapshot?)=range.update(id(),snapshot)
}
