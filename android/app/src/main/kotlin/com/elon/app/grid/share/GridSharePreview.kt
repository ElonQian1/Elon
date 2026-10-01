package com.elon.app.grid.share

import android.widget.CheckBox
import android.widget.EditText
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import org.json.JSONObject

internal class GridSharePreview(activity: AppCompatActivity, private val snapshot: GridShareRead.Snapshot,
    targetName: String, previous: String?, submit: (JSONObject) -> Unit, close: () -> Unit) {
    private val ui = GridShareViews(activity)
    private val withNote = CheckBox(activity).apply { text = "附加个人说明"; minHeight = ui.dp(48); contentDescription = "grid-share-with-note" }
    private val note = EditText(activity).apply { hint = "个人说明"; filters = arrayOf(android.text.InputFilter.LengthFilter(200)); visibility = android.view.View.GONE }
    private val preview = ui.column(0)
    private val error: TextView = ui.text("")
    private var showing = false
    private var busy = false
    private fun grid(previous: String?) = GridShareModel.project(snapshot.fields, snapshot.observed, true,
        if (withNote.isChecked) note.text.toString() else "", previous)
    private val dialog = AlertDialog.Builder(activity).setCustomTitle(ui.dialogTitle(if (previous == null) "分享网格到群聊" else "更新网格快照"))
        .setView(ScrollView(activity).apply { addView(ui.column().apply {
            addView(ui.text("发送到：$targetName", 17f)); addView(ui.text("已公开金额与数量 · 群成员将看到以下数据", quiet = true)); addView(withNote); addView(note); addView(preview)
            addView(ui.text("仅分享这次快照。未读取的持仓和收益不会推算；更新会生成新卡片。", quiet = true)); addView(error)
            snapshot.positionNotice?.let { addView(ui.text(it)) }
        }) }).setNegativeButton("取消", null).setPositiveButton("确认发送", null).create()
    init {
        fun render() { preview.removeAllViews(); val data = grid(previous); preview.addView(ui.summary(data)); preview.addView(ui.details(data)) }
        withNote.setOnCheckedChangeListener { _, checked -> note.visibility = if (checked) android.view.View.VISIBLE else android.view.View.GONE }
        dialog.setOnDismissListener { showing = false; close() }
        render()
        dialog.show(); ui.styleDialog(dialog); showing = true
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).apply {
            contentDescription = "grid-share-submit"
            setOnClickListener { if (!busy) submit(grid(previous)) }
        }
    }
    fun progress() {
        busy = true; error.text = "发送中…"; withNote.isEnabled = false; note.isEnabled = false
        dialog.setCancelable(false); dialog.getButton(AlertDialog.BUTTON_NEGATIVE).isEnabled = false
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = false
    }
    fun failed(message: String) {
        if (!showing) return
        busy = false; error.text = message; withNote.isEnabled = true; note.isEnabled = true
        dialog.setCancelable(true); dialog.getButton(AlertDialog.BUTTON_NEGATIVE).isEnabled = true
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = true
    }
    fun close() = dialog.dismiss()
}
