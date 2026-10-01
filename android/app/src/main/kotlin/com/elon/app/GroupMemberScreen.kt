package com.elon.app

import android.app.Dialog
import android.os.Handler
import android.os.Looper
import android.view.View
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.widget.doAfterTextChanged
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import okhttp3.Call

/** Full-height, paginated directory. The old mention picker stays independently compatible. */
internal class GroupMemberScreen(private val activity: AppCompatActivity, private val repository: GroupMemberRepository, private val callbacks: GroupMemberCallbacks = GroupMemberCallbacks()) {
    fun show(groupId: String, initialMode: String = "all"): Dialog {
        val ui = GroupMemberViews(activity); val handler = Handler(Looper.getMainLooper()); val owner = repository.owner()
        val dialog = Dialog(activity).apply { requestWindowFeature(android.view.Window.FEATURE_NO_TITLE) }
        val root = ui.column(); val header = ui.row(); val title = ui.label("群成员")
        header.addView(ui.button("返回") { dialog.dismiss() }); header.addView(title, LinearLayout.LayoutParams(0, -2, 1f))
        val search = EditText(activity).apply { hint = "搜索全群昵称"; contentDescription = "搜索群成员"; setSingleLine(); minHeight = ui.dp(48); setTextColor(ui.colors.text); setHintTextColor(ui.colors.muted) }
        val status = ui.label("正在加载群成员…", true).apply { setPadding(ui.dp(16), ui.dp(8), ui.dp(16), ui.dp(8)); accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE }
        val filters = ui.row(); val list = ListView(activity).apply { dividerHeight = ui.dp(1); contentDescription = "完整群成员名单" }
        val footer = ui.column(); val more = ui.button("加载更多") {}; list.addFooterView(more)
        root.addView(header); root.addView(search); root.addView(filters); root.addView(status); root.addView(list, LinearLayout.LayoutParams(-1, 0, 1f)); root.addView(footer)
        var data: GroupRoster? = null; var call: Call? = null; var generation = 0; var busy = false; var filter = "all"; var managing = initialMode == "manage"; var initial = true
        val selected = linkedSetOf<String>(); val filterButtons = mutableListOf<Pair<String, Button>>()
        lateinit var load: (Boolean, Boolean) -> Unit
        val actions = GroupMemberActions(activity, repository, groupId, callbacks, { load(false, true) }, dialog::dismiss)
        val adapter = object : BaseAdapter() {
            override fun getCount() = data?.people?.size ?: 0
            override fun getItem(position: Int) = data!!.people[position]
            override fun getItemId(position: Int) = getItem(position).id.hashCode().toLong()
            override fun getView(position: Int, convertView: View?, parent: ViewGroup): View {
                val person = getItem(position); val roster = data!!
                return ui.personRow(person, roster.selfId, if (managing && roster.canRemove(person)) selected.contains(person.id) else null, convertView) {
                    if (managing && roster.canRemove(person)) {
                        if (!selected.remove(person.id)) { if (selected.size < 100) selected.add(person.id) else Toast.makeText(activity, "每次最多选择 100 人", Toast.LENGTH_SHORT).show() }
                        notifyDataSetChanged(); footer.tag?.let { (it as? (() -> Unit))?.invoke() }
                    } else actions.profile(person, roster)
                }
            }
        }
        list.adapter = adapter
        fun renderFooter() {
            footer.removeAllViews(); val roster = data ?: return
            val row = ui.row()
            if (roster.canInvite) row.addView(ui.button("邀请成员") { GroupMemberInvite(activity, repository, actions).show(roster) }, LinearLayout.LayoutParams(0, -2, 1f))
            if (roster.canManage) row.addView(ui.button(if (managing) "完成管理" else "管理成员") { managing = !managing; selected.clear(); adapter.notifyDataSetChanged(); renderFooter() }, LinearLayout.LayoutParams(0, -2, 1f))
            val menu = ui.button("更多") {}; row.addView(menu, LinearLayout.LayoutParams(0, -2, 1f)); footer.addView(row)
            menu.setOnClickListener {
                PopupMenu(activity, menu).apply {
                    val entries = mutableListOf<Pair<String, () -> Unit>>()
                    entries.add("刷新名单" to { load(false, true) })
                    if (roster.canManage) entries.add("待审邀请（${roster.pending}）" to { actions.invitations() })
                    if (roster.isOwner) entries.add("成员邀请规则" to { actions.invitationPolicy(roster) })
                    entries.add("退出群聊" to { actions.confirm("退出群聊", if (roster.isOwner && roster.total > 1) "你是群主，请先转让群主，再退出群聊。" else "退出 ${roster.name} 后将不再接收群消息。", GroupMemberActions.command("leave")) })
                    if (roster.isOwner) entries.add("解散群聊" to { actions.confirm("解散群聊", "全部 ${roster.total} 位成员将失去群聊访问权限。此操作无法撤销。", GroupMemberActions.command("dissolve")) })
                    entries.forEachIndexed { index, entry -> getMenu().add(0, index, index, entry.first) }
                    setOnMenuItemClickListener { entries[it.itemId].second(); true }; show()
                }
            }
            if (managing && roster.canManage) footer.addView(ui.button("移出所选成员（${selected.size}）") {
                actions.confirm("移出所选成员", "将 ${roster.people.filter { selected.contains(it.id) }.joinToString("、") { it.name }} 移出群聊。", GroupMemberActions.command("remove", selected.toList()))
            }.apply { isEnabled = selected.isNotEmpty(); setTextColor(ui.colors.error) })
        }
        footer.tag = { renderFooter() }
        val refresh = object : Runnable { override fun run() { if (dialog.isShowing) { if (repository.owner() != owner) dialog.dismiss() else if (dialog.window?.decorView?.hasWindowFocus() == true) load(false, false) }; if (dialog.isShowing) handler.postDelayed(this, 15000) } }
        load = load@{ next, reset ->
            if (!dialog.isShowing || (busy && !reset) || repository.owner() != owner) return@load
            val cursor = if (next) data?.next ?: return@load else null
            call?.cancel(); val request = ++generation; busy = true; more.isEnabled = false
            if (reset) { data = null; adapter.notifyDataSetChanged(); footer.removeAllViews(); status.text = "正在加载群成员…" }
            call = repository.roster(groupId, search.text.toString(), filter, cursor) { result -> handler.post {
                if (!dialog.isShowing || request != generation) return@post
                busy = false; more.isEnabled = true
                result.fold(onSuccess = { page ->
                    val previous = data; val state = list.onSaveInstanceState()
                    data = if (next && previous != null) page.copy(people = (previous.people + page.people).distinctBy { it.id })
                        else if (previous?.revision == page.revision) previous.copy(pending = page.pending) else page
                    if (previous?.revision != page.revision) selected.clear()
                    title.text = "群成员（${page.total}）"
                    status.text = if (page.matched == 0) "未找到匹配的群成员 · 全群 ${page.total} 人" else if (search.text.isNotBlank() || filter != "all") "找到 ${page.matched} 人 · 全群 ${page.total} 人" else "${page.name} · 共 ${page.total} 位成员"
                    adapter.notifyDataSetChanged(); if (!reset && state != null) list.onRestoreInstanceState(state)
                    more.visibility = if (data?.next != null) View.VISIBLE else View.GONE; more.text = "加载更多（${data?.people?.size}/${page.matched}）"
                    renderFooter()
                    if (initial) { initial = false; if (initialMode == "invite" && page.canInvite) GroupMemberInvite(activity, repository, actions).show(page) }
                }, onFailure = { error ->
                    status.text = error.message ?: "无法连接服务器，请重试"
                    if (error is RosterFailure && error.status in listOf(401, 403, 404)) { data = null; selected.clear(); title.text = "群成员"; actions.dismissDetails(); adapter.notifyDataSetChanged(); footer.removeAllViews() }
                    more.visibility = View.VISIBLE; more.text = "重新加载"; more.setOnClickListener { load(false, true) }
                    if (error is RosterFailure && error.status == 409) load(false, true)
                })
            } }
            more.setOnClickListener { load(true, false) }
        }
        listOf("all" to "全部", "admins" to "群主与管理员", "recent" to "最近加入").forEach { (key, label) ->
            val button = ui.button(label) { filter = key; selected.clear(); filterButtons.forEach { (id, item) -> item.isSelected = id == filter }; load(false, true) }.apply { textSize = 12f; isSelected = key == filter }
            filterButtons.add(key to button); filters.addView(button, LinearLayout.LayoutParams(0, -2, 1f))
        }
        var debounce: Runnable? = null
        search.doAfterTextChanged { debounce?.let(handler::removeCallbacks); debounce = Runnable { selected.clear(); load(false, true) }.also { handler.postDelayed(it, 250) } }
        val lifecycle = object : DefaultLifecycleObserver { override fun onDestroy(owner: LifecycleOwner) { dialog.dismiss() } }
        activity.lifecycle.addObserver(lifecycle)
        dialog.setContentView(root); dialog.setOnDismissListener { generation++; call?.cancel(); actions.close(); handler.removeCallbacksAndMessages(null); activity.lifecycle.removeObserver(lifecycle) }
        dialog.show(); dialog.window?.let { window ->
            window.setLayout(-1, -1); window.setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_ADJUST_RESIZE)
            WindowCompat.setDecorFitsSystemWindows(window, false)
            ViewCompat.setOnApplyWindowInsetsListener(root) { view, insets -> val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.ime()); view.setPadding(bars.left, bars.top, bars.right, bars.bottom); insets }
            ViewCompat.requestApplyInsets(root)
        }
        load(false, true); handler.postDelayed(refresh, 15000)
        return dialog
    }
}
