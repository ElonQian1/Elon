package com.elon.app.grid.manage

import android.app.Activity
import android.view.View
import android.widget.*

internal class BinanceManageForm(private val activity: Activity, edited: () -> Unit) {
    val root=LinearLayout(activity).apply {orientation=LinearLayout.VERTICAL;isSaveEnabled=false}
    private var choices=emptyList<Pair<String,String>>()
    private fun select(title: String, id: String, values: List<String>): Spinner {
        root.addView(TextView(activity).apply {text=title;textSize=16f})
        return Spinner(activity).apply {
            isSaveEnabled=false;contentDescription=id
            adapter=ArrayAdapter(activity,android.R.layout.simple_spinner_dropdown_item,values)
            root.addView(this)
        }
    }
    private val strategy=select("选择本账号已读取的网格", "binance-manage-strategy", listOf("请选择策略"))
    private val operation=select("本次操作", "binance-manage-action", listOf("请选择操作","修改终止时平仓设置","结束网格"))
    private val mode=select("终止时的仓位处理", "binance-manage-mode", listOf("请选择处理方式","按市价平仓","保留仓位，自行处理"))
    init {
        listOf(strategy,operation,mode).forEach {it.onItemSelectedListener=object:AdapterView.OnItemSelectedListener {
            override fun onItemSelected(parent:AdapterView<*>?,view:View?,position:Int,id:Long)=edited()
            override fun onNothingSelected(parent:AdapterView<*>?)=edited()
        }}
    }
    fun refresh(values: List<Pair<String,String>>) {
        if(values==choices)return
        val previous=id();choices=values
        strategy.adapter=ArrayAdapter(activity,android.R.layout.simple_spinner_dropdown_item,listOf(if(values.isEmpty()) "暂无可管理的已验证策略" else "请选择策略")+values.map{it.second})
        strategy.setSelection(values.indexOfFirst{it.first==previous}+1)
    }
    fun id()=choices.getOrNull(strategy.selectedItemPosition-1)?.first
    fun action()=when(operation.selectedItemPosition){1->"settings";2->"close";else->error("请选择本次操作")}
    fun cps()=when(mode.selectedItemPosition){1->true;2->false;else->error("请选择终止时的仓位处理")}
}
