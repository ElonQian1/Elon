package com.elon.app.grid.manage

import android.app.Activity
import android.text.Editable
import android.text.InputType
import android.text.TextWatcher
import android.view.View
import android.widget.*

internal class BinanceRangeFields(private val activity:Activity,private val edited:()->Unit) {
    private val ui=BinanceManageAppearance(activity)
    val root=LinearLayout(activity).apply{orientation=LinearLayout.VERTICAL;isSaveEnabled=false}
    private var current:BinanceRangeSnapshot?=null
    private var boundId:String?=null
    private fun field(label:String,id:String,decimal:Boolean=true):EditText {
        root.addView(ui.label(label,14f))
        return EditText(activity).apply {
            isSaveEnabled=false;setSingleLine();contentDescription="binance-manage-range-$id"
            inputType=InputType.TYPE_CLASS_NUMBER or if(decimal)InputType.TYPE_NUMBER_FLAG_DECIMAL else 0
            setTextColor(ui.text);setHintTextColor(ui.muted)
            addTextChangedListener(object:TextWatcher {
                override fun beforeTextChanged(s:CharSequence?,start:Int,count:Int,after:Int){}
                override fun onTextChanged(s:CharSequence?,start:Int,before:Int,count:Int)=edited()
                override fun afterTextChanged(s:Editable?){}
            });root.addView(this)
        }
    }
    private val lower=field("新区间下限","lower")
    private val upper=field("新区间上限","upper")
    private val count=field("新网格数量","count",false)
    private val fill=ui.button("填入当前区间和格数","binance-manage-range-fill") {
        current?.let{lower.setText(it.lower);upper.setText(it.upper);count.setText(it.count.toString())}
    }.also{root.addView(it)}
    private val amount=field("本次追加投入 · USDT","amount").apply{hint="明确填写金额；不追加填0"}
    private val close=Spinner(activity).apply {
        root.addView(ui.label("修改区间时如何处理现有仓位",14f))
        isSaveEnabled=false;contentDescription="binance-manage-range-close"
        adapter=ui.choices(listOf("请选择处理方式","保留现有仓位","按市价平掉现有仓位"));root.addView(this)
        onItemSelectedListener=object:AdapterView.OnItemSelectedListener {
            override fun onItemSelected(parent:AdapterView<*>?,view:View?,position:Int,id:Long)=edited()
            override fun onNothingSelected(parent:AdapterView<*>?)=edited()
        }
    }
    init {root.addView(ui.label("币安会重建网格订单；按市价平仓会产生实际成交。原价格止盈止损保持原值，预计最低追加额暂不可用，币安会在提交时检查余额、参数及限额。",14f))}
    fun update(id:String?,snapshot:BinanceRangeSnapshot?) {
        current=snapshot
        if(boundId!=id){boundId=id;lower.setText("");upper.setText("");count.setText("");amount.setText("");close.setSelection(0)}
        fill.isEnabled=snapshot!=null
    }
    fun draft():BinanceRangeDraft {
        val number=count.text.toString()
        require(Regex("[1-9][0-9]{0,4}").matches(number)){"请填写整数格数。"}
        val cps=when(close.selectedItemPosition){1->false;2->true;else->error("请选择修改时的仓位处理。")}
        return BinanceRangeDraft(BinanceRange.price(lower.text.toString()),BinanceRange.price(upper.text.toString()),number.toInt(),cps,amount.text.toString())
    }
}
