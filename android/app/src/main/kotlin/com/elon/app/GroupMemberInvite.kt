package com.elon.app

import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.core.widget.doAfterTextChanged

internal class GroupMemberInvite(private val activity: AppCompatActivity, private val repository: GroupMemberRepository, private val actions: GroupMemberActions) {
    fun show(roster: GroupRoster) {
        val ui = GroupMemberViews(activity)
        val body = ui.column().apply { setPadding(ui.dp(16), 0, ui.dp(16), ui.dp(12)) }
        val search = EditText(activity).apply { hint = "搜索好友"; contentDescription = "搜索好友"; setSingleLine(); minHeight = ui.dp(48); setTextColor(ui.colors.text); setHintTextColor(ui.colors.muted) }
        val status = ui.label("正在加载好友…", true)
        val list = ListView(activity)
        val submit = ui.button("邀请（0）") {}.apply { isEnabled = false }
        body.addView(search); body.addView(status); body.addView(list, LinearLayout.LayoutParams(-1, (activity.resources.displayMetrics.heightPixels * 0.4).toInt())); body.addView(submit)
        val dialog = actions.show("邀请群成员", body)
        var friends = emptyList<RosterPerson>(); var visible = friends
        val selected = linkedSetOf<String>()
        val adapter = object : BaseAdapter() {
            override fun getCount() = visible.size
            override fun getItem(position: Int) = visible[position]
            override fun getItemId(position: Int) = position.toLong()
            override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
                val person = visible[position]
                return (convertView as? CheckedTextView ?: CheckedTextView(activity).apply {
                    minHeight = ui.dp(52); setPadding(ui.dp(8), ui.dp(8), ui.dp(8), ui.dp(8)); setTextColor(ui.colors.text)
                    checkMarkDrawable = CheckBox(activity).buttonDrawable
                }).apply { text = person.name; isChecked = selected.contains(person.id); alpha = if (selected.contains(person.id)) 1f else .85f }
            }
        }
        list.adapter = adapter
        fun render() {
            visible = friends.filter { it.name.contains(search.text.toString().trim(), true) }
            adapter.notifyDataSetChanged(); submit.text = "${if (roster.selfRole == "member" && roster.policy == "approval") "提交审核" else "邀请"}（${selected.size}）"; submit.isEnabled = selected.isNotEmpty()
            status.text = if (friends.isEmpty()) "暂无好友，请先添加好友" else if (visible.isEmpty()) "未找到匹配的好友" else "最多选择 100 位好友，已在群中的好友不会重复加入"
        }
        list.setOnItemClickListener { _, _, position, _ ->
            val id = visible[position].id
            if (!selected.remove(id)) { if (selected.size < 100) selected.add(id) else Toast.makeText(activity, "每次最多选择 100 人", Toast.LENGTH_SHORT).show() }
            render()
        }
        search.doAfterTextChanged { render() }
        submit.setOnClickListener {
            val names = friends.filter { selected.contains(it.id) }.joinToString("、") { it.name }
            actions.confirm(if (roster.selfRole == "member" && roster.policy == "approval") "提交邀请" else "邀请成员", "邀请 $names 加入群聊。", GroupMemberActions.command("invite", selected.toList()))
        }
        fun load() {
            status.text = "正在加载好友…"
            val call = repository.friends { result -> Handler(Looper.getMainLooper()).post {
                if (!dialog.isShowing) return@post
                result.fold(onSuccess = { friends = it; render() }, onFailure = { status.text = it.message ?: "加载失败"; body.addView(ui.button("重试") { load() }) })
            } }
            dialog.setOnDismissListener { call.cancel() }
        }
        load()
    }
}
