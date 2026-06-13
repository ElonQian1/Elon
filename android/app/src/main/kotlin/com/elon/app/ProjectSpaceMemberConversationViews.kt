package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.text.TextUtils
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class ProjectSpaceMemberConversationViews(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val renderActiveSpace: () -> Unit,
    private val renderMessages: (ProjectMemberConversation, ProjectMember, ProjectSpace) -> Unit,
    private val openPersonalConversationById: (String) -> Unit,
    private val showCreateAndOpenPersonalConversation: (String?, (Int) -> Unit) -> Unit,
    private val openPersonalAiChat: (Int) -> Unit
) {
    private var latestContainer: LinearLayout? = null

    fun renderList(
        container: LinearLayout,
        space: ProjectSpace,
        member: ProjectMember,
        isSelf: Boolean
    ) {
        latestContainer = container
        container.removeAllViews()
        container.addView(backRow("← 项目空间") { renderActiveSpace() })
        container.addView(header(member, isSelf))
        container.addView(sectionTitle("项目 AI 会话"))

        val loadingView = inlineStatusRow("正在加载会话...", "#A8A8A8")
        container.addView(loadingView)

        thread {
            val result = runCatching {
                fetchProjectMemberConversations(http, serverUrl, activity, space.project.id, member.userId)
            }
            activity.runOnUiThread {
                if (container.indexOfChild(loadingView) < 0) return@runOnUiThread
                container.removeView(loadingView)
                result.onSuccess { conversations ->
                    if (conversations.isEmpty()) {
                        container.addView(inlineStatusRow("还没有项目 AI 会话", "#777777"))
                    } else {
                        conversations.forEach { conversation ->
                            container.addView(card(conversation, member, isSelf, space))
                        }
                    }
                    if (isSelf && space.project.role != "observer") {
                        if (conversations.isNotEmpty()) {
                            container.addView(projectSpaceDivider(activity, dp))
                        }
                        container.addView(createPersonalConversationRow())
                    }
                }.onFailure { error ->
                    container.addView(inlineStatusRow(error.message ?: "加载失败", "#FF7A7A"))
                }
            }
        }
    }

    private fun backRow(label: String, onClick: () -> Unit): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#222222")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
            addView(TextView(activity).apply {
                text = label
                textSize = 15f
                setTextColor(Color.parseColor("#60A5FA"))
            })
        }
    }

    private fun header(member: ProjectMember, isSelf: Boolean): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(16), dp(20), dp(8))
            addView(TextView(activity).apply {
                text = buildString {
                    append(member.account)
                    if (isSelf) append(" (我)")
                }
                textSize = 20f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#D6D6D6"))
            })
            addView(TextView(activity).apply {
                text = projectRoleLabel(member.role)
                textSize = 13f
                setTextColor(Color.parseColor("#A8A8A8"))
                setPadding(0, dp(6), 0, 0)
            })
        }
    }

    private fun card(
        conversation: ProjectMemberConversation,
        member: ProjectMember,
        isSelf: Boolean,
        space: ProjectSpace
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#222222")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener {
                if (isSelf) openPersonalConversationById(conversation.id) else renderMessages(conversation, member, space)
            }
            if (isSelf) {
                setOnLongClickListener {
                    showVisibilityActions(conversation, member, space)
                    true
                }
            }
            addView(TextView(activity).apply {
                text = conversation.title?.takeIf { it.isNotBlank() } ?: "会话 ${conversation.id.take(8)}"
                textSize = 16f
                setTextColor(Color.parseColor("#D6D6D6"))
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = conversationSummary(conversation)
                textSize = 12f
                setTextColor(Color.parseColor("#777777"))
                setPadding(0, dp(5), 0, 0)
            })
            conversation.lastMessage?.takeIf { it.isNotBlank() }?.let { preview ->
                addView(TextView(activity).apply {
                    text = preview
                    textSize = 13f
                    setTextColor(Color.parseColor("#A8A8A8"))
                    setPadding(0, dp(8), 0, 0)
                    maxLines = 2
                    ellipsize = TextUtils.TruncateAt.END
                })
            }
            if (!isSelf) addView(forkRow(member, conversation))
        }
    }

    private fun forkRow(member: ProjectMember, conversation: ProjectMemberConversation): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.END
            setPadding(0, dp(10), 0, 0)
            addView(TextView(activity).apply {
                text = "在此基础上分叉 →"
                textSize = 13f
                setTextColor(Color.parseColor("#58BE6A"))
                setTypeface(typeface, Typeface.BOLD)
                isClickable = true
                setOnClickListener {
                    val forkTitle = "分叉 ${member.account}：${conversation.title ?: "会话"}"
                    showCreateAndOpenPersonalConversation(forkTitle) { index -> openPersonalAiChat(index) }
                }
            })
        }
    }

    private fun createPersonalConversationRow(): TextView {
        return TextView(activity).apply {
            text = "+ 新建个人 AI 会话"
            textSize = 15f
            setTextColor(Color.parseColor("#D6D6D6"))
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#222222")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener {
                showCreateAndOpenPersonalConversation(null) { index -> openPersonalAiChat(index) }
            }
        }
    }

    private fun showVisibilityActions(
        conversation: ProjectMemberConversation,
        member: ProjectMember,
        space: ProjectSpace
    ) {
        val nextPublic = !conversation.isPublic
        val action = if (nextPublic) "设为公开" else "关闭公开"
        AlertDialog.Builder(activity)
            .setTitle(conversation.title?.takeIf { it.isNotBlank() } ?: "会话 ${conversation.id.take(8)}")
            .setItems(arrayOf(action)) { dialog, _ ->
                dialog.dismiss()
                updateVisibility(conversation, member, space, nextPublic)
            }
            .show()
    }

    private fun updateVisibility(
        conversation: ProjectMemberConversation,
        member: ProjectMember,
        space: ProjectSpace,
        isPublic: Boolean
    ) {
        thread {
            val result = runCatching {
                updateProjectMemberConversationVisibility(http, serverUrl, activity, space.project.id, conversation.id, isPublic)
            }
            activity.runOnUiThread {
                result.onSuccess { updated ->
                    Toast.makeText(activity, if (updated.isPublic) "已设为公开" else "已关闭公开", Toast.LENGTH_SHORT).show()
                    latestContainer?.let { renderList(it, space, member, true) }
                }.onFailure { error ->
                    Toast.makeText(activity, error.message ?: "修改失败", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    private fun inlineStatusRow(text: String, colorHex: String): TextView {
        return TextView(activity).apply {
            this.text = text
            textSize = 14f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor(colorHex))
            setPadding(dp(20), dp(36), dp(20), dp(36))
        }
    }

    private fun sectionTitle(textValue: String): TextView {
        return TextView(activity).apply {
            text = textValue
            textSize = 13f
            setTextColor(Color.parseColor("#777777"))
            setPadding(dp(20), dp(18), dp(20), dp(6))
        }
    }
}

private fun conversationSummary(conversation: ProjectMemberConversation): String {
    return buildString {
        append(if (conversation.isPublic) "公开" else "私密")
        append(" · ")
        append("${conversation.messageCount} 条消息")
        if (conversation.taskCount > 0) append(" · ${conversation.taskCount} 个任务")
        conversation.lastTaskStatus?.takeIf { it.isNotBlank() }?.let { st ->
            append(" · ").append(
                when (st) {
                    "running" -> "运行中"
                    "done" -> "已完成"
                    "failed" -> "失败"
                    else -> st
                }
            )
        }
        conversation.updatedAt.takeIf { it.isNotBlank() }?.let { append(" · ").append(it) }
    }
}
