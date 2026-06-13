package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.graphics.drawable.GradientDrawable
import android.text.InputType
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.CheckBox
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.RadioGroup
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal object ProjectSpaceMemberManagement {
    private data class RoleOption(val role: String, val label: String)
    private data class InviteBatchResult(
        val successCount: Int,
        val errorCount: Int,
        val firstError: String?
    )

    private val roleOptions = listOf(
        RoleOption("admin", "管理员"),
        RoleOption("editor", "协作者"),
        RoleOption("member", "成员"),
        RoleOption("observer", "只读成员")
    )

    fun inviteRow(
        activity: AppCompatActivity,
        dp: (Int) -> Int,
        selectableForeground: () -> Drawable?,
        onClick: () -> Unit
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(12), dp(20), dp(12))
            background = panelBackground(dp, "#16221A")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            addView(TextView(activity).apply {
                text = "+ 邀请成员"
                textSize = 16f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#7CE38B"))
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = "通过手机号、昵称、邮箱或用户 ID 添加，并设置成员权限"
                textSize = 12f
                setTextColor(Color.parseColor("#8FB998"))
                setPadding(0, dp(5), 0, 0)
            })
        }
    }

    fun actionRow(
        activity: AppCompatActivity,
        dp: (Int) -> Int,
        selectableForeground: () -> Drawable?,
        onChangeRole: () -> Unit,
        onRemove: () -> Unit
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, dp(10), 0, 0)
            addView(actionButton(activity, dp, selectableForeground, "改权限", "#DDE8F8", "#233044", onChangeRole))
            addView(actionButton(activity, dp, selectableForeground, "踢出", "#FFB7B7", "#3A2020", onRemove))
        }
    }

    fun showInviteDialog(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        existingMemberIds: Set<String>,
        dp: (Int) -> Int,
        onChanged: () -> Unit
    ) {
        val accountInput = accountInput(activity, dp)
        val (roleView, selectedRole) = rolePicker(activity, dp, "member")
        val selectedFriends = linkedMapOf<String, AppFriend>()
        val friendList = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
        }
        val friendStatus = TextView(activity).apply {
            text = "正在加载好友..."
            textSize = 13f
            setTextColor(Color.parseColor("#777777"))
            setPadding(0, dp(4), 0, dp(6))
        }
        val friendScroll = ScrollView(activity).apply {
            isFillViewport = false
            addView(friendList)
        }
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(4), dp(4), dp(4), 0)
            addView(TextView(activity).apply {
                text = "从好友列表勾选成员，也可以手动输入账号。"
                textSize = 13f
                setTextColor(Color.parseColor("#777777"))
                setPadding(0, 0, 0, dp(6))
            })
            addView(friendStatus)
            addView(friendScroll, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(220)
            ))
            addView(accountInput)
            addView(roleView)
        }

        val dialog = AlertDialog.Builder(activity)
            .setTitle("邀请成员")
            .setView(body)
            .setNegativeButton("取消", null)
            .setPositiveButton("邀请", null)
            .show()
        thread(name = "project-load-invite-friends") {
            val result = runCatching { fetchProjectInviteFriends(http, serverUrl, activity) }
            activity.runOnUiThread {
                result.onSuccess { friends ->
                    renderFriendRows(
                        activity = activity,
                        dp = dp,
                        friends = friends,
                        existingMemberIds = existingMemberIds,
                        selectedFriends = selectedFriends,
                        friendList = friendList,
                        friendStatus = friendStatus
                    )
                }.onFailure { error ->
                    friendStatus.text = error.message ?: "好友加载失败，可手动输入账号"
                    friendStatus.setTextColor(Color.parseColor("#B35E5E"))
                }
            }
        }
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
            val account = accountInput.text?.toString()?.trim().orEmpty()
            val selected = selectedFriends.values.toList()
            if (account.isBlank() && selected.isEmpty()) {
                Toast.makeText(activity, "请选择好友或输入账号", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }
            val positive = dialog.getButton(AlertDialog.BUTTON_POSITIVE)
            positive.isEnabled = false
            positive.text = "邀请中..."
            thread(name = "project-invite-member") {
                val result = inviteSelectedMembers(
                    http = http,
                    serverUrl = serverUrl,
                    activity = activity,
                    projectId = projectId,
                    selectedFriends = selected,
                    manualAccount = account,
                    role = selectedRole()
                )
                activity.runOnUiThread {
                    positive.isEnabled = true
                    positive.text = "邀请"
                    if (result.successCount > 0) {
                        val tail = result.firstError?.let { "，${result.errorCount} 个失败：$it" }.orEmpty()
                        Toast.makeText(activity, "已邀请 ${result.successCount} 位成员$tail", Toast.LENGTH_SHORT).show()
                        dialog.dismiss()
                        onChanged()
                    } else {
                        Toast.makeText(
                            activity,
                            result.firstError ?: "邀请成员失败",
                            Toast.LENGTH_SHORT
                        ).show()
                    }
                }
            }
        }
    }

    private fun renderFriendRows(
        activity: AppCompatActivity,
        dp: (Int) -> Int,
        friends: List<AppFriend>,
        existingMemberIds: Set<String>,
        selectedFriends: MutableMap<String, AppFriend>,
        friendList: LinearLayout,
        friendStatus: TextView
    ) {
        friendList.removeAllViews()
        selectedFriends.clear()
        if (friends.isEmpty()) {
            friendStatus.text = "暂无好友，可手动输入账号邀请"
            friendStatus.setTextColor(Color.parseColor("#777777"))
            return
        }
        friendStatus.text = "好友列表"
        friendStatus.setTextColor(Color.parseColor("#777777"))
        friends.forEach { friend ->
            val alreadyMember = existingMemberIds.contains(friend.id)
            friendList.addView(CheckBox(activity).apply {
                text = buildString {
                    append(friend.name.ifBlank { "好友" })
                    friend.account.takeIf { it.isNotBlank() }?.let { append(" · ").append(it) }
                    if (alreadyMember) append(" · 已在项目中")
                }
                textSize = 14f
                setTextColor(Color.parseColor(if (alreadyMember) "#9AA1AD" else "#2E2E2E"))
                setPadding(0, dp(5), 0, dp(5))
                isEnabled = !alreadyMember
                setOnCheckedChangeListener { _, checked ->
                    if (checked) selectedFriends[friend.id] = friend
                    else selectedFriends.remove(friend.id)
                }
            }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ))
        }
    }

    private fun inviteSelectedMembers(
        http: OkHttpClient,
        serverUrl: String,
        activity: AppCompatActivity,
        projectId: String,
        selectedFriends: List<AppFriend>,
        manualAccount: String,
        role: String
    ): InviteBatchResult {
        var successCount = 0
        val errors = mutableListOf<String>()
        selectedFriends.forEach { friend ->
            runCatching {
                inviteProjectMember(http, serverUrl, activity, projectId, friend.id, role)
            }.onSuccess {
                successCount += 1
            }.onFailure { error ->
                errors += "${friend.name.ifBlank { "好友" }}：${error.message ?: "失败"}"
            }
        }
        manualAccount.takeIf { it.isNotBlank() }?.let { account ->
            runCatching {
                inviteProjectMember(http, serverUrl, activity, projectId, account, role)
            }.onSuccess {
                successCount += 1
            }.onFailure { error ->
                errors += "手动输入：${error.message ?: "失败"}"
            }
        }
        return InviteBatchResult(
            successCount = successCount,
            errorCount = errors.size,
            firstError = errors.firstOrNull()
        )
    }

    fun showRoleDialog(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        member: ProjectMember,
        dp: (Int) -> Int,
        onChanged: () -> Unit
    ) {
        val (roleView, selectedRole) = rolePicker(activity, dp, member.role)
        val dialog = AlertDialog.Builder(activity)
            .setTitle("${member.account.ifBlank { "成员" }}权限")
            .setView(roleView)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .show()
        dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
            thread(name = "project-update-member-role") {
                val result = runCatching {
                    updateProjectMemberRole(http, serverUrl, activity, projectId, member.userId, selectedRole())
                }
                activity.runOnUiThread {
                    result.onSuccess { updated ->
                        Toast.makeText(
                            activity,
                            "已设置为${projectRoleLabel(updated.role)}",
                            Toast.LENGTH_SHORT
                        ).show()
                        dialog.dismiss()
                        onChanged()
                    }.onFailure { error ->
                        Toast.makeText(activity, error.message ?: "修改权限失败", Toast.LENGTH_SHORT).show()
                    }
                }
            }
        }
    }

    fun confirmRemove(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        member: ProjectMember,
        onChanged: () -> Unit
    ) {
        AlertDialog.Builder(activity)
            .setTitle("踢出成员")
            .setMessage("确定将 ${member.account.ifBlank { "该成员" }} 移出项目？")
            .setNegativeButton("取消", null)
            .setPositiveButton("踢出") { _, _ ->
                thread(name = "project-remove-member") {
                    val result = runCatching {
                        removeProjectMember(http, serverUrl, activity, projectId, member.userId)
                    }
                    activity.runOnUiThread {
                        result.onSuccess {
                            Toast.makeText(activity, "已移出成员", Toast.LENGTH_SHORT).show()
                            onChanged()
                        }.onFailure { error ->
                            Toast.makeText(activity, error.message ?: "移除成员失败", Toast.LENGTH_SHORT).show()
                        }
                    }
                }
            }
            .show()
    }

    private fun accountInput(activity: AppCompatActivity, dp: (Int) -> Int): EditText {
        return EditText(activity).apply {
            hint = "手机号 / 昵称 / 邮箱 / 用户 ID"
            inputType = InputType.TYPE_CLASS_TEXT
            setSingleLine(true)
            setTextColor(Color.parseColor("#2E2E2E"))
            setHintTextColor(Color.parseColor("#777777"))
            setPadding(dp(10), dp(8), dp(10), dp(8))
        }
    }

    private fun rolePicker(
        activity: AppCompatActivity,
        dp: (Int) -> Int,
        currentRole: String
    ): Pair<LinearLayout, () -> String> {
        var selectedRole = roleOptions.firstOrNull { it.role == currentRole }?.role ?: "member"
        val group = RadioGroup(activity).apply {
            orientation = RadioGroup.VERTICAL
            roleOptions.forEach { option ->
                addView(RadioButton(activity).apply {
                    id = View.generateViewId()
                    tag = option.role
                    text = option.label
                    textSize = 15f
                    setTextColor(Color.parseColor("#2E2E2E"))
                    isChecked = option.role == selectedRole
                })
            }
            setOnCheckedChangeListener { view, checkedId ->
                selectedRole = view.findViewById<RadioButton>(checkedId)?.tag as? String ?: selectedRole
            }
        }
        val body = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(4), dp(8), dp(4), 0)
            addView(TextView(activity).apply {
                text = "权限"
                textSize = 13f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#777777"))
                setPadding(0, 0, 0, dp(4))
            })
            addView(group)
        }
        return body to { selectedRole }
    }

    private fun actionButton(
        activity: AppCompatActivity,
        dp: (Int) -> Int,
        selectableForeground: () -> Drawable?,
        textValue: String,
        textColor: String,
        bgColor: String,
        onClick: () -> Unit
    ): TextView {
        return TextView(activity).apply {
            text = textValue
            textSize = 12f
            gravity = Gravity.CENTER
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(Color.parseColor(textColor))
            background = panelBackground(dp, bgColor)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            layoutParams = LinearLayout.LayoutParams(0, dp(32), 1f).apply {
                rightMargin = dp(8)
            }
        }
    }

    private fun panelBackground(dp: (Int) -> Int, color: String): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(color))
            cornerRadius = dp(6).toFloat()
        }
    }
}
