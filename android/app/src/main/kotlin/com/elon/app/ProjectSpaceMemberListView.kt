package com.elon.app

import android.graphics.Color
import android.text.TextUtils
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient

internal class ProjectSpaceMemberListView(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val personalConversations: () -> List<AppConversation>,
    private val showCreatePersonalConversation: () -> Unit,
    private val openMemberConversations: (ProjectMember) -> Unit,
    private val reloadActiveSpace: () -> Unit
) {
    fun render(
        container: LinearLayout,
        space: ProjectSpace,
        members: List<ProjectMember>,
        selfId: String,
        userProjectRoute: Boolean
    ) {
        val canManageMembers = canManageProjectMembers(space.project.role) && !userProjectRoute
        if (canManageMembers) {
            container.addView(ProjectSpaceMemberManagement.inviteRow(activity, dp, selectableForeground) {
                ProjectSpaceMemberManagement.showInviteDialog(
                    activity = activity,
                    http = http,
                    serverUrl = serverUrl,
                    projectId = space.project.id,
                    existingMemberIds = space.members.mapTo(mutableSetOf()) { it.userId },
                    dp = dp,
                    onChanged = { reloadActiveSpace() }
                )
            })
        }
        if (members.isEmpty()) {
            container.addView(emptyConversationRow().apply { text = "暂无成员" })
            if (space.project.role != "observer") container.addView(createConversationRow())
            return
        }
        var selfInList = false
        members.forEach { member ->
            val isSelf = member.userId == selfId
            if (isSelf) selfInList = true
            container.addView(memberCard(member, isSelf, space, userProjectRoute))
        }
        if (space.project.role != "observer" && !selfInList) {
            container.addView(createConversationRow())
        }
    }

    private fun memberCard(
        member: ProjectMember,
        isSelf: Boolean,
        space: ProjectSpace,
        userProjectRoute: Boolean
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(12), dp(20), dp(12))
            background = panelBackground(if (isSelf) "#20262E" else "#0E1116")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openMemberConversations(member) }
            addView(TextView(activity).apply {
                text = buildString {
                    append(member.account)
                    if (isSelf) append(" (我)")
                }
                textSize = 16f
                setTextColor(Color.parseColor("#F8F7F4"))
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = buildString {
                    append(projectRoleLabel(member.role))
                    val convCount = personalConversations().takeIf { isSelf }?.size
                    if (convCount != null && convCount > 0) append(" · $convCount 个会话")
                }
                textSize = 12f
                setTextColor(Color.parseColor("#80BEBEBA"))
                setPadding(0, dp(5), 0, 0)
            })
            if (canManageProjectMembers(space.project.role) &&
                !userProjectRoute &&
                !isSelf &&
                member.role != "owner"
            ) {
                addView(ProjectSpaceMemberManagement.actionRow(
                    activity = activity,
                    dp = dp,
                    selectableForeground = selectableForeground,
                    onChangeRole = {
                        ProjectSpaceMemberManagement.showRoleDialog(
                            activity = activity,
                            http = http,
                            serverUrl = serverUrl,
                            projectId = space.project.id,
                            member = member,
                            dp = dp,
                            onChanged = { reloadActiveSpace() }
                        )
                    },
                    onRemove = {
                        ProjectSpaceMemberManagement.confirmRemove(
                            activity = activity,
                            http = http,
                            serverUrl = serverUrl,
                            projectId = space.project.id,
                            member = member,
                            onChanged = { reloadActiveSpace() }
                        )
                    }
                ))
            }
        }
    }

    private fun createConversationRow(): TextView {
        return TextView(activity).apply {
            text = "+ 新建个人 AI 会话"
            textSize = 15f
            setTextColor(Color.parseColor("#F8F7F4"))
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#0E1116")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { showCreatePersonalConversation() }
        }
    }

    private fun emptyConversationRow(): TextView {
        return TextView(activity).apply {
            text = "暂无个人会话"
            textSize = 13f
            setTextColor(Color.parseColor("#80BEBEBA"))
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#0E1116")
        }
    }
}
