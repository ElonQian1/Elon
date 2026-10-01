package com.elon.app

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.ListView
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.core.widget.doAfterTextChanged
import okhttp3.Call

/** The complete human roster, independent of the nine-avatar conversation preview. */
internal class GroupMembersDialog(private val context: Context, private val directory: GroupMentionDirectory) {
    fun show(groupId: String): AlertDialog {
        val colors = MobileColors(context)
        fun dp(value: Int) = (value * context.resources.displayMetrics.density).toInt()
        val main = Handler(Looper.getMainLooper())
        val status = TextView(context).apply {
            setTextColor(colors.muted)
            accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE
            setPadding(0, dp(12), 0, dp(12))
        }
        val search = EditText(context).apply {
            hint = "搜索群成员"
            contentDescription = "搜索群成员"
            setSingleLine(true)
            setTextColor(colors.text)
            setHintTextColor(colors.muted)
            minHeight = dp(48)
        }
        val adapter = object : ArrayAdapter<String>(context, android.R.layout.simple_list_item_1) {
            override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
                return (super.getView(position, convertView, parent) as TextView).apply {
                    setTextColor(colors.text)
                    minHeight = dp(56)
                    setSingleLine(false)
                }
            }
        }
        val list = ListView(context).apply {
            this.adapter = adapter
            dividerHeight = dp(1)
            contentDescription = "完整群成员名单"
        }
        val retry = Button(context).apply { text = "重新加载"; minHeight = dp(48) }
        val content = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), 0, dp(20), dp(12))
            setBackgroundColor(colors.surface)
            addView(search)
            addView(status)
            addView(retry)
            val listHeight = (context.resources.displayMetrics.heightPixels * 0.4f).toInt()
            addView(list, LinearLayout.LayoutParams(ViewGroup.LayoutParams.MATCH_PARENT, listHeight))
        }
        val dialog = AlertDialog.Builder(context)
            .setTitle("群成员")
            .setView(content)
            .setNegativeButton("关闭", null)
            .create()
        var people = emptyList<GroupMentionTarget>()
        var call: Call? = null
        var generation = 0
        fun render() {
            val query = search.text.toString().trim()
            val visible = people.filter { query.isEmpty() || it.name.contains(query, ignoreCase = true) }
            adapter.clear()
            adapter.addAll(visible.map { it.name })
            status.text = when {
                people.isEmpty() -> "暂无群成员"
                visible.isEmpty() -> "未找到匹配的群成员（共 ${people.size} 人）"
                query.isNotEmpty() -> "找到 ${visible.size} 人 · 共 ${people.size} 人"
                else -> "共 ${people.size} 位成员"
            }
        }
        fun load() {
            call?.cancel()
            val request = ++generation
            people = emptyList()
            adapter.clear()
            search.isEnabled = false
            status.text = "正在加载群成员…"
            retry.visibility = View.GONE
            call = directory.loadMembers(groupId) { result ->
                main.post {
                    if (!dialog.isShowing || request != generation) return@post
                    result.fold(onSuccess = {
                        people = it
                        search.isEnabled = true
                        render()
                    }, onFailure = {
                        status.text = "无法加载群成员，请重试"
                        retry.visibility = View.VISIBLE
                    })
                }
            }
        }
        search.doAfterTextChanged { if (search.isEnabled) render() }
        retry.setOnClickListener { load() }
        dialog.setOnDismissListener { generation++; call?.cancel() }
        dialog.setOnShowListener { load() }
        dialog.show()
        return dialog
    }
}
