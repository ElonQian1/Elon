package com.elon.app.grid.ui

import android.app.Activity
import android.content.res.ColorStateList
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView

/** Shared appearance for the host's existing compatibility pages. */
internal class BinanceGridAppearance(private val activity: Activity) {
    private val uiColors = com.elon.app.MobileColors(activity)
    val background = uiColors.surface
    val surface = uiColors.container
    val text = uiColors.text
    val muted = uiColors.muted
    val accent = uiColors.primary
    fun dp(value: Int) = (value * activity.resources.displayMetrics.density).toInt()
    fun shape(color: Int) = GradientDrawable().apply {
        setColor(color); cornerRadius = dp(12).toFloat()
    }
    fun label(value: String, size: Float) = TextView(activity).apply {
        text = value; textSize = size; setTextColor(this@BinanceGridAppearance.text)
        isSaveEnabled = false; setPadding(0, dp(6), 0, dp(6))
        if (size >= 20) setTypeface(typeface, Typeface.BOLD)
    }
    fun button(value: String, id: String, primary: Boolean = false, action: () -> Unit) = Button(activity).apply {
        text = value; textSize = 14f; isAllCaps = false; contentDescription = id
        isSaveEnabled = false; filterTouchesWhenObscured = true; minHeight = dp(48)
        setPadding(dp(12), dp(10), dp(12), dp(10))
        background = shape(if (primary) accent else surface)
        backgroundTintList = ColorStateList(
            arrayOf(intArrayOf(android.R.attr.state_enabled), intArrayOf()),
            intArrayOf(if (primary) accent else surface, surface))
        setTextColor(ColorStateList(
            arrayOf(intArrayOf(android.R.attr.state_enabled), intArrayOf()),
            intArrayOf(if (primary) uiColors.onPrimary else this@BinanceGridAppearance.text, muted)))
        layoutParams = LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(5); bottomMargin = dp(5) }
        setOnClickListener { action() }
    }
    fun field(field: EditText) = field.apply {
        setTextColor(this@BinanceGridAppearance.text); setHintTextColor(muted)
        textSize = 16f; minimumHeight = dp(48)
        background = shape(surface); backgroundTintList = null
        setPadding(dp(12), dp(10), dp(12), dp(10))
        layoutParams = LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(8) }
    }
    fun choices(values: List<String>) = object : ArrayAdapter<String>(activity, android.R.layout.simple_spinner_dropdown_item, values) {
        private fun decorate(view: View) = view.apply {
            (this as TextView).setTextColor(this@BinanceGridAppearance.text)
            setBackgroundColor(surface); minimumHeight = dp(48)
            setPadding(dp(12), dp(10), dp(12), dp(10))
        }
        override fun getView(position: Int, convertView: View?, parent: ViewGroup): View = decorate(super.getView(position, convertView, parent))
        override fun getDropDownView(position: Int, convertView: View?, parent: ViewGroup): View = decorate(super.getDropDownView(position, convertView, parent))
    }
}
