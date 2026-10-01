package com.elon.app.grid.share

import android.content.Context
import android.graphics.Typeface
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.appcompat.app.AlertDialog
import androidx.core.widget.doAfterTextChanged

/** Private source selection; no source IDs or account identity are rendered or shared. */
internal object GridSharePicker {
    fun show(context: Context, rows: List<Map<String, String?>>, selected: (Map<String, String?>) -> Unit): AlertDialog {
        val s = GridShareStyle(context)
        val source = GridSharePresentation.sortRows(rows)
        var visible = source
        val search = EditText(context).apply {
            hint = "搜索代币，例如 QNT"; setSingleLine(); minHeight = s.dp(48)
            setTextColor(s.ink); setHintTextColor(s.muted); contentDescription = "搜索网格代币"
        }
        val count = s.text("共 ${source.size} 条 · 按网格利润从高到低", 12f, true)
        val list = ListView(context).apply { divider = null; contentDescription = "可分享的币安网格" }
        val adapter = object : BaseAdapter() {
            override fun getCount() = visible.size
            override fun getItem(position: Int) = visible[position]
            override fun getItemId(position: Int) = position.toLong()
            override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
                val item = getItem(position); val symbol = item["symbol"].orEmpty(); val profit = item["profit"]
                return s.column(12).apply {
                    background = s.panel(border = s.color(com.elon.app.R.color.mobile_outline_variant))
                    addView(s.row().apply {
                        addView(s.logo(symbol))
                        addView(s.column(0).apply {
                            setPadding(s.dp(12), 0, 0, 0)
                            addView(s.text(symbol, 17f).apply { typeface = Typeface.DEFAULT_BOLD })
                            addView(s.text("${when(item["direction"]) { "LONG" -> "做多"; "SHORT" -> "做空"; "NEUTRAL" -> "中性"; else -> "方向未读取" }} · ${item["leverage"] ?: "?"}× · ${item["count"] ?: "?"} 格", 12f).apply { setTextColor(s.direction(item["direction"])) })
                        }, LinearLayout.LayoutParams(0, -2, 1f))
                    })
                    addView(s.text("网格利润 · USDT", 12f, true))
                    addView(s.text(GridSharePresentation.format(profit, true), 21f).apply { setTextColor(s.tone(profit)); typeface = Typeface.DEFAULT_BOLD })
                    minimumHeight = s.dp(108)
                }
            }
        }
        list.adapter = adapter
        val body = s.column().apply {
            addView(search); addView(count)
            addView(list, LinearLayout.LayoutParams(-1, minOf(s.dp(400), (context.resources.displayMetrics.heightPixels * .45f).toInt())))
        }
        val dialog = AlertDialog.Builder(context).setCustomTitle(s.dialogTitle("选择要分享的网格"))
            .setView(body).setNegativeButton("取消", null).create()
        search.doAfterTextChanged {
            val query = it.toString().trim()
            visible = source.filter { row -> row["symbol"].orEmpty().contains(query, ignoreCase = true) }
            count.text = if (visible.isEmpty()) "没有匹配的网格" else "${visible.size} 条 · 按网格利润从高到低"
            adapter.notifyDataSetChanged()
        }
        list.setOnItemClickListener { _, _, index, _ -> val item = visible[index]; dialog.dismiss(); selected(item) }
        dialog.show(); s.styleDialog(dialog)
        return dialog
    }
}
