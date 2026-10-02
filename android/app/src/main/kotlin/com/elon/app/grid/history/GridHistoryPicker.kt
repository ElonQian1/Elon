package com.elon.app.grid.history

import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.elon.app.grid.share.GridShareRead
import com.elon.app.grid.share.GridSharePicker
import com.elon.app.grid.share.GridShareViews
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Job
import kotlinx.coroutines.launch

/** Shared explicit history chooser for group sharing and personal ChatGPT. */
internal class GridHistoryPicker(private val activity: AppCompatActivity) {
    private var dialog: AlertDialog? = null
    private var job: Job? = null
    private var generation = 0
    private var last: GridHistoryRead.Page? = null
    fun close() { generation++; job?.cancel(); dialog?.dismiss(); dialog = null; last = null }
    fun open(valid: () -> Boolean, selected: (GridShareRead.Snapshot) -> Unit) {
        close()
        dialog = AlertDialog.Builder(activity).setTitle("读取已结束网格")
            .setItems(arrayOf("最近 7 天", "最近 30 天", "最近 90 天")) { _, index -> load(listOf(7,30,90)[index], 1, valid, selected) }
            .setNegativeButton("取消", null).show()
    }
    private fun load(days: Int, page: Int, valid: () -> Boolean, selected: (GridShareRead.Snapshot) -> Unit) {
        dialog?.dismiss(); val run = ++generation
        val ui = GridShareViews(activity)
        dialog = AlertDialog.Builder(activity).setTitle("历史网格 · 第 $page 页").setMessage("正在读取当前币安账户最近 $days 天记录…")
            .setNegativeButton("取消") { _, _ -> close() }.create().also { it.setOnCancelListener { close() }; it.show() }
        job = activity.lifecycleScope.launch {
            try {
                val result = GridHistoryRead(activity).page(days, page, if(page > 1) last?.source else null)
                if (run != generation || !valid()) return@launch
                last = result; dialog?.dismiss()
                val body = ui.column().apply {
                    addView(ui.text("最近 $days 天 · 第 $page 页 · 共 ${result.total} 条。仅本页 ${result.rows.size} 条。", quiet = true))
                    addView(ui.text("网格利润不等于最终总盈亏；结算与平仓信息未知时不会推算。", quiet = true))
                    addView(ui.button(if(result.rows.isEmpty()) "本页没有记录" else "选择本页网格") {
                        if(valid() && result.rows.isNotEmpty()) {
                            dialog?.dismiss()
                            dialog = GridSharePicker.show(activity, result.rows.map { it.fields }) { row ->
                                if (run == generation && valid()) selected(result.rows.single { it.fields["id"] == row["id"] })
                            }
                        }
                    })
                    if(page > 1) addView(ui.button("上一页") { if(valid()) load(days,page-1,valid,selected) })
                    if(page*20 < result.total) addView(ui.button("下一页") { if(valid()) load(days,page+1,valid,selected) })
                    addView(ui.button("更换时间范围") { if(valid()) open(valid,selected) })
                }
                dialog = AlertDialog.Builder(activity).setCustomTitle(ui.dialogTitle("已结束网格"))
                    .setView(body).setNegativeButton("取消") { _, _ -> close() }.show().also(ui::styleDialog)
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (failure: Exception) { if(run == generation && valid()) {
                dialog?.dismiss(); dialog = AlertDialog.Builder(activity).setTitle("历史读取未完成").setMessage(failure.message ?: "请检查币安会话后重试")
                    .setPositiveButton("重试") { _, _ -> load(days,1,valid,selected) }.setNegativeButton("取消", null).show()
            } }
        }
    }
}
