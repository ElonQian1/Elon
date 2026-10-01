package com.elon.app

import android.os.Handler
import android.os.Looper
import android.widget.ScrollView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.Call
import org.json.JSONArray
import org.json.JSONObject
import java.util.UUID

internal data class GroupMemberCallbacks(
    val changed: () -> Unit = {}, val exited: () -> Unit = {},
    val mention: (RosterPerson) -> Unit = {}, val message: (RosterPerson) -> Unit = {},
)

internal class GroupMemberActions(
    private val activity: AppCompatActivity, private val repository: GroupMemberRepository,
    private val groupId: String, private val callbacks: GroupMemberCallbacks,
    private val refresh: () -> Unit, private val closeScreen: () -> Unit,
) {
    private val views = GroupMemberViews(activity)
    private val main = Handler(Looper.getMainLooper())
    private var active: AlertDialog? = null
    private var call: Call? = null
    private var closed = false
    fun dismissDetails() { active?.dismiss(); call?.cancel() }
    fun close() { closed = true; dismissDetails() }
    fun show(title: String, body: android.view.View): AlertDialog {
        active?.dismiss()
        return AlertDialog.Builder(activity).setTitle(title).setView(body).setNegativeButton("关闭", null).create().also { active = it; it.show() }
    }
    fun confirm(title: String, explanation: String, command: JSONObject) {
        active?.dismiss()
        val request = JSONObject(command.toString()).put("request_id", UUID.randomUUID().toString())
        val body = views.column().apply { setPadding(views.dp(20), views.dp(12), views.dp(20), views.dp(12)) }
        body.addView(views.label(explanation))
        val status = views.label("", true).apply { accessibilityLiveRegion = android.view.View.ACCESSIBILITY_LIVE_REGION_POLITE }
        body.addView(status)
        val dialog = AlertDialog.Builder(activity).setTitle(title).setView(body).setNegativeButton("取消", null).setPositiveButton(title, null).create()
        active = dialog; dialog.show()
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
            dialog.setCancelable(false); dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = false; dialog.getButton(AlertDialog.BUTTON_NEGATIVE).isEnabled = false
            status.text = "正在处理…"
            call = repository.command(groupId, request) { result -> main.post {
                if (closed || !dialog.isShowing) return@post
                dialog.setCancelable(true); dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled = true; dialog.getButton(AlertDialog.BUTTON_NEGATIVE).isEnabled = true
                result.fold(onSuccess = { receipt ->
                    dialog.dismiss(); callbacks.changed()
                    Toast.makeText(activity, receipt.optString("message", "操作已完成"), Toast.LENGTH_SHORT).show()
                    if (receipt.optBoolean("exited")) { closeScreen(); callbacks.exited() } else refresh()
                }, onFailure = { status.text = (it.message ?: "无法连接服务器") + "。可重试此操作。" })
            } }
        }
    }
    fun profile(person: RosterPerson, roster: GroupRoster) {
        val body = views.column().apply { setPadding(views.dp(16), views.dp(12), views.dp(16), views.dp(12)) }
        body.addView(views.avatar(person, 56)); body.addView(views.label(person.name + if (person.id == roster.selfId) "（我）" else ""))
        body.addView(views.label("${person.roleLabel} · 入群 ${person.joinedAt.take(10)}", true))
        val dialog = show("群成员资料", ScrollView(activity).apply { addView(body) })
        if (person.id == roster.selfId) return
        body.addView(views.button("在群里 @TA") { dialog.dismiss(); closeScreen(); callbacks.mention(person) })
        val status = views.label("", true)
        val message = views.button("添加好友并私聊") {}
        message.setOnClickListener {
            message.isEnabled = false
            call = repository.addFriend(person) { result -> main.post {
                if (closed || !dialog.isShowing) return@post
                message.isEnabled = true
                result.fold(onSuccess = { dialog.dismiss(); closeScreen(); callbacks.message(person) }, onFailure = { status.text = it.message ?: "操作失败，请重试" })
            } }
        }
        body.addView(message); body.addView(status)
        if (roster.isOwner) {
            val label = if (person.role == "admin") "取消管理员" else "设为管理员"
            body.addView(views.button(label) { confirm(label, "修改 ${person.name} 的群管理权限。", command("role", listOf(person.id)).put("role", if (person.role == "admin") "member" else "admin")) })
            body.addView(views.button("转让群主") { confirm("转让群主", "将群主转让给 ${person.name}，你将成为普通成员。", command("transfer", listOf(person.id))) })
        }
        if (roster.canRemove(person)) body.addView(views.button("移出群聊") { confirm("移出群聊", "将 ${person.name} 移出 ${roster.name}。", command("remove", listOf(person.id))) }.apply { setTextColor(views.colors.error) })
    }
    fun invitationPolicy(roster: GroupRoster) {
        val policies = listOf("members" to "所有成员可邀请好友", "admins" to "仅群主和管理员可邀请", "approval" to "普通成员邀请需审核")
        active?.dismiss()
        active = AlertDialog.Builder(activity).setTitle("成员邀请规则").setSingleChoiceItems(policies.map { it.second }.toTypedArray(), policies.indexOfFirst { it.first == roster.policy }) { dialog, index ->
            dialog.dismiss(); confirm("更新邀请规则", "邀请规则将改为“${policies[index].second}”。", command("policy").put("invitation_policy", policies[index].first))
        }.setNegativeButton("取消", null).show()
    }
    fun invitations(offset: Int = 0) {
        val body = views.column().apply { setPadding(views.dp(16), views.dp(12), views.dp(16), views.dp(12)) }
        val status = views.label("正在加载…"); body.addView(status)
        val dialog = show("待审邀请", ScrollView(activity).apply { addView(body) })
        call = repository.invitations(groupId, offset) { result -> main.post {
            if (closed || !dialog.isShowing) return@post
            result.fold(onSuccess = { json ->
                status.text = "共 ${json.getInt("total_count")} 条待审邀请"
                GroupMemberRepository.objects(json.getJSONArray("requests")).forEach { request ->
                    val names = GroupMemberRepository.objects(request.getJSONArray("members")).joinToString("、") { it.getString("display_name") }
                    val copy = "${request.getString("actor_name")} 邀请 $names"
                    body.addView(views.label(copy))
                    listOf("approve" to "通过邀请", "reject" to "拒绝邀请").forEach { (action, label) -> body.addView(views.button(label) { confirm(label, copy, command(action).put("invitation_id", request.getString("id"))) }) }
                }
                if (offset > 0) body.addView(views.button("上一页") { invitations((offset - 50).coerceAtLeast(0)) })
                if (!json.isNull("next_offset")) body.addView(views.button("下一页") { invitations(json.getInt("next_offset")) })
            }, onFailure = { status.text = it.message ?: "加载失败"; body.addView(views.button("重新加载") { invitations(offset) }) })
        } }
    }
    companion object {
        fun command(action: String, users: List<String> = emptyList()) = JSONObject().put("action", action).put("user_ids", JSONArray(users))
    }
}
