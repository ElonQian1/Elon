package com.elon.app.grid.manage

import android.app.Activity
import android.content.res.ColorStateList
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.TextView

internal class BinanceManageAppearance(private val activity: Activity) {
    val background=0xFF0B111B.toInt();val surface=0xFF1C2737.toInt()
    val text=0xFFF0F5FA.toInt();val muted=0xFF9DAEC2.toInt();val accent=0xFF3BDAA6.toInt()
    fun dp(value:Int)=(value*activity.resources.displayMetrics.density).toInt()
    fun shape(color:Int)=GradientDrawable().apply{setColor(color);cornerRadius=dp(12).toFloat()}
    fun label(value:String,size:Float)=TextView(activity).apply{
        text=value;textSize=size;setTextColor(this@BinanceManageAppearance.text);isSaveEnabled=false
        setPadding(0,dp(8),0,dp(8));if(size>=20)setTypeface(typeface,Typeface.BOLD)
    }
    fun button(value:String,id:String,action:()->Unit)=Button(activity).apply{
        text=value;textSize=14f;isAllCaps=false;contentDescription=id;isSaveEnabled=false;filterTouchesWhenObscured=true
        minHeight=dp(48);setPadding(dp(12),dp(10),dp(12),dp(10))
        val primary=id=="binance-manage-reload" || id=="binance-manage-read"
        background=shape(if(primary)accent else surface)
        setTextColor(ColorStateList(arrayOf(intArrayOf(android.R.attr.state_enabled),intArrayOf()),intArrayOf(if(primary)this@BinanceManageAppearance.background else this@BinanceManageAppearance.text,muted)))
        setOnClickListener{action()}
        layoutParams=android.widget.LinearLayout.LayoutParams(-1,-2).apply{topMargin=dp(6);bottomMargin=dp(6)}
    }
    fun choices(values:List<String>)=object:ArrayAdapter<String>(activity,android.R.layout.simple_spinner_dropdown_item,values){
        private fun decorate(view:View)=view.apply{(this as TextView).setTextColor(this@BinanceManageAppearance.text);setBackgroundColor(surface);minimumHeight=dp(48);setPadding(dp(12),dp(10),dp(12),dp(10))}
        override fun getView(position:Int,convertView:View?,parent:ViewGroup):View=decorate(super.getView(position,convertView,parent))
        override fun getDropDownView(position:Int,convertView:View?,parent:ViewGroup):View=decorate(super.getDropDownView(position,convertView,parent))
    }
}
