package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.TextUtils
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal object ProjectMemberConversationDialog {
    fun show(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        member: ProjectMember,
        dp: (Int) -> Int,
        onOpenConversation: ((String) -> Unit)? = null,
        onCreateConversation: (() -> Unit)? = null,
        onForkConversation: ((ProjectMemberConversation) -> Unit)? = null
    ) {
        val body = bodyContainer(activity, dp)
        val dialog = AlertDialog.Builder(activity)
            .setTitle("${member.account.ifBlank { "成员" }}的项目 AI 会话")
            .setView(scrollBody(activity, body))
            .setPositiveButton("关闭", null)
            .show()

        renderLoading(activity, body, dp, "正在加载会话...")
        thread {
            val result = runCatching {
                fetchProjectMemberConversations(http, serverUrl, activity, projectId, member.userId)
            }
            activity.runOnUiThread {
                if (!dialog.isShowing) return@runOnUiThread
                body.removeAllViews()
                result.onSuccess { conversations ->
                    if (conversations.isEmpty()) {
                        body.addView(emptyRow(activity, dp, "这个成员还没有项目 AI 会话"))
                    } else {
                        conversations.forEach { conversation ->
                            val forkAction = onForkConversation?.let { fork ->
                                { dialog.dismiss(); fork(conversation) }
                            }
                            val openAction = onOpenConversation?.let { open ->
                                { dialog.dismiss(); open(conversation.id) }
                            }
                            body.addView(conversationRow(activity, conversation, dp, forkAction, openAction) {
                                dialog.dismiss()
                                showMessages(activity, http, serverUrl, projectId, member, conversation, dp)
                            })
                        }
                    }
                    if (onOpenConversation != null) {
                        body.addView(createNewConversationButton(activity, dp) { dialog.dismiss(); onCreateConversation?.invoke() })
                    }
                }.onFailure { error ->
                    body.addView(errorRow(activity, dp, error.message ?: "加载成员会话失败"))
                }
            }
        }
    }

    private fun showMessages(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        member: ProjectMember,
        conversation: ProjectMemberConversation,
        dp: (Int) -> Int
    ) {
        val body = bodyContainer(activity, dp)
        val title = conversation.title?.takeIf { it.isNotBlank() } ?: "会话 ${conversation.id.take(8)}"
        val dialog = AlertDialog.Builder(activity)
            .setTitle(title)
            .setView(scrollBody(activity, body))
            .setNegativeButton("返回") { _, _ ->
                show(activity, http, serverUrl, projectId, member, dp)
            }
            .setPositiveButton("关闭", null)
            .show()

        renderLoading(activity, body, dp, "正在加载消息...")
        thread {
            val result = runCatching {
                fetchProjectMemberConversationMessages(
                    http = http,
                    serverUrl = serverUrl,
                    context = activity,
                    projectId = projectId,
                    memberUserId = member.userId,
                    conversationId = conversation.id
                )
            }
            activity.runOnUiThread {
                if (!dialog.isShowing) return@runOnUiThread
                body.removeAllViews()
                result.onSuccess { messages ->
                    if (messages.isEmpty()) {
                        body.addView(emptyRow(activity, dp, "这个会话还没有可查阅的消息"))
                    } else {
                        messages.forEach { message ->
                            body.addView(messageRow(activity, message, dp))
                        }
                    }
                }.onFailure { error ->
                    Toast.makeText(activity, error.message ?: "加载会话消息失败", Toast.LENGTH_SHORT).show()
                    body.addView(errorRow(activity, dp, error.message ?: "加载会话消息失败"))
                }
            }
        }
    }

    private fun bodyContainer(activity: AppCompatActivity, dp: (Int) -> Int): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(18), dp(8), dp(18), dp(8))
        }
    }

    private fun scrollBody(activity: AppCompatActivity, body: LinearLayout): ScrollView {
        return ScrollView(activity).apply {
            isFillViewport = false
            addView(body)
        }
    }

    private fun conversationRow(
        activity: AppCompatActivity,
        conversation: ProjectMemberConversation,
        dp: (Int) -> Int,
        onFork: (() -> Unit)? = null,
        onOpen: (() -> Unit)? = null,
        onClick: () -> Unit
    ): View {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { bottomMargin = dp(8) }
            orientation = LinearLayout.VERTICAL
            setPadding(dp(14), dp(12), dp(14), dp(12))
            background = panelBackground("#181B20", dp)
            isClickable = true
            setOnClickListener { onClick() }
            addView(TextView(activity).apply {
                text = conversation.title?.takeIf { it.isNotBlank() } ?: "会话 ${conversation.id.take(8)}"
                textSize = 16f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(Color.parseColor("#F2F5FA"))
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = conversationMeta(conversation)
                textSize = 12f
                setTextColor(Color.parseColor("#A6AFBD"))
                setPadding(0, dp(5), 0, 0)
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            conversation.lastMessage?.takeIf { it.isNotBlank() }?.let { preview ->
                addView(TextView(activity).apply {
                    text = preview
                    textSize = 13f
                    setTextColor(Color.parseColor("#A6AFBD"))
                    setPadding(0, dp(8), 0, 0)
                    maxLines = 2
                    ellipsize = TextUtils.TruncateAt.END
                })
            }
            if (onOpen != null || onFork != null) {
                addView(LinearLayout(activity).apply {
                    orientation = LinearLayout.HORIZONTAL
                    gravity = android.view.Gravity.END
                    setPadding(0, dp(10), 0, 0)
                    if (onOpen != null) {
                        addView(TextView(activity).apply {
                            text = "继续会话 →"
                            textSize = 13f
                            setTextColor(Color.parseColor("#60A5FA"))
                            setTypeface(typeface, Typeface.BOLD)
                            isClickable = true
                            setOnClickListener { onOpen() }
                        })
                    }
                    if (onFork != null) {
                        addView(TextView(activity).apply {
                            text = "在此基础上分叉 →"
                            textSize = 13f
                            setTextColor(Color.parseColor("#58BE6A"))
                            setTypeface(typeface, Typeface.BOLD)
                            isClickable = true
                            setOnClickListener { onFork() }
                        })
                    }
                })
            }
        }
    }

    private fun messageRow(
        activity: AppCompatActivity,
        message: ProjectMemberConversationMessage,
        dp: (Int) -> Int
    ): View {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { bottomMargin = dp(8) }
            orientation = LinearLayout.VERTICAL
            setPadding(dp(14), dp(12), dp(14), dp(12))
            background = panelBackground("#181B20", dp)
            addView(TextView(activity).apply {
                val label = if (message.role == "discussion") {
                    message.senderName?.takeIf { it.isNotBlank() } ?: "讨论"
                } else {
                    roleLabel(message.role)
                }
                text = "$label · ${message.createdAt}"
                textSize = 12f
                setTypeface(typeface, Typeface.BOLD)
                setTextColor(roleColor(message.role))
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = message.content.ifBlank { "(空消息)" }
                textSize = 14f
                setTextColor(Color.parseColor("#F2F5FA"))
                setPadding(0, dp(6), 0, 0)
            })
        }
    }

    private fun renderLoading(
        activity: AppCompatActivity,
        body: LinearLayout,
        dp: (Int) -> Int,
        text: String
    ) {
        body.removeAllViews()
        body.addView(TextView(activity).apply {
            this.text = text
            textSize = 15f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#A6AFBD"))
            setPadding(dp(20), dp(42), dp(20), dp(42))
        })
    }

    private fun emptyRow(activity: AppCompatActivity, dp: (Int) -> Int, text: String): TextView {
        return TextView(activity).apply {
            this.text = text
            textSize = 15f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#A6AFBD"))
            setPadding(dp(20), dp(42), dp(20), dp(42))
        }
    }

    private fun errorRow(activity: AppCompatActivity, dp: (Int) -> Int, text: String): TextView {
        return TextView(activity).apply {
            this.text = text
            textSize = 15f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#FF7A7A"))
            setPadding(dp(20), dp(42), dp(20), dp(42))
        }
    }

    private fun conversationMeta(conversation: ProjectMemberConversation): String {
        return buildString {
            append(if (conversation.isPublic) "公开" else "私密")
            append(" · ")
            append("${conversation.messageCount} 条消息")
            if (conversation.taskCount > 0) append(" · ${conversation.taskCount} 个任务")
            conversation.lastTaskStatus?.takeIf { it.isNotBlank() }?.let {
                append(" · ").append(taskStatusLabel(it))
            }
            conversation.updatedAt.takeIf { it.isNotBlank() }?.let {
                append(" · ").append(it)
            }
        }
    }

    private fun taskStatusLabel(status: String): String = when (status) {
        "running" -> "运行中"
        "done" -> "已完成"
        "failed" -> "失败"
        else -> status
    }

    private fun roleLabel(role: String): String = when (role) {
        "user" -> "成员"
        "assistant" -> "AI"
        "system" -> "系统"
        "discussion" -> "讨论"
        else -> role
    }

    private fun roleColor(role: String): Int = Color.parseColor(
        when (role) {
            "user" -> "#93C5FD"
            "assistant" -> "#A7F3D0"
            "system" -> "#FCA5A5"
            "discussion" -> "#C4B5FD"
            else -> "#A6AFBD"
        }
    )

    private fun createNewConversationButton(activity: AppCompatActivity, dp: (Int) -> Int, onClick: () -> Unit): TextView {
        return TextView(activity).apply {
            text = "+ 新建个人 AI 会话"
            textSize = 15f
            setTextColor(Color.parseColor("#F2F5FA"))
            setPadding(dp(14), dp(14), dp(14), dp(14))
            isClickable = true
            setOnClickListener { onClick() }
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = dp(8) }
        }
    }

    private fun panelBackground(color: String, dp: (Int) -> Int): GradientDrawable {
        return GradientDrawable().apply {
            setColor(Color.parseColor(color))
            cornerRadius = dp(0).toFloat()
        }
    }
}
