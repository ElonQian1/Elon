package com.elon.app

import android.graphics.Color
import android.text.TextUtils
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity

internal class ProjectSpacePersonalConversationPanel(
    private val activity: AppCompatActivity,
    private val personalConversations: () -> List<AppConversation>,
    private val activePersonalConversationIndex: () -> Int,
    private val isPersonalConversationWorking: (Int) -> Boolean,
    private val openPersonalAiChat: (Int) -> Unit,
    private val showPersonalConversationActions: (Int) -> Unit,
    private val showCreatePersonalConversation: () -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?
) {
    fun render(container: LinearLayout) {
        val conversations = personalConversations()
        if (conversations.isEmpty()) {
            container.addView(emptyRow())
        } else {
            val activeIndex = activePersonalConversationIndex()
            conversations.forEachIndexed { index, conversation ->
                container.addView(
                    row(
                        index = index,
                        conversation = conversation,
                        active = index == activeIndex,
                        working = isPersonalConversationWorking(index)
                    )
                )
            }
            container.addView(projectSpaceDivider(activity, dp))
        }
        container.addView(createRow())
    }

    private fun row(
        index: Int,
        conversation: AppConversation,
        active: Boolean,
        working: Boolean
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(20), dp(12), dp(20), dp(12))
            val rowBackground = panelBackground(if (active) "#283140" else "#181B20")
            background = rowBackground
            if (working) startProjectConversationShimmer(this, rowBackground)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { openPersonalAiChat(index) }
            setOnLongClickListener {
                showPersonalConversationActions(index)
                true
            }
            addView(TextView(activity).apply {
                text = buildString {
                    append(conversation.title.ifBlank { "个人会话 ${index + 1}" })
                    if (active) append("  ·  当前")
                    if (conversation.ended) append("  ·  已结束")
                }
                textSize = 16f
                setTextColor(Color.parseColor("#F2F5FA"))
                maxLines = 1
                ellipsize = TextUtils.TruncateAt.END
            })
            addView(TextView(activity).apply {
                text = hint(conversation)
                textSize = 12f
                setTextColor(Color.parseColor("#6F7785"))
                setPadding(0, dp(5), 0, 0)
                maxLines = 2
                ellipsize = TextUtils.TruncateAt.END
            })
        }
    }

    private fun createRow(): TextView {
        return TextView(activity).apply {
            text = "+ 新建个人 AI 会话"
            textSize = 15f
            setTextColor(Color.parseColor("#F2F5FA"))
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#181B20")
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { showCreatePersonalConversation() }
        }
    }

    private fun emptyRow(): TextView {
        return TextView(activity).apply {
            text = "暂无个人会话"
            textSize = 13f
            setTextColor(Color.parseColor("#6F7785"))
            setPadding(dp(20), dp(14), dp(20), dp(14))
            background = panelBackground("#181B20")
        }
    }

    private fun hint(conversation: AppConversation): String {
        val subtitle = conversation.subtitle.takeIf { it.isNotBlank() } ?: "还没有消息"
        return if (conversation.messages.isEmpty()) {
            subtitle
        } else {
            "${conversation.messages.size} 条消息 · $subtitle"
        }
    }
}
