package com.elon.app

import android.graphics.Color
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainHomeListActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val projects: () -> List<AppProject>,
    private val conversations: () -> List<AppConversation>,
    private val friends: () -> List<AppFriend>,
    private val activeProject: () -> AppProject,
    private val compactProjectTitle: () -> String,
    private val formatTime: (Long) -> String,
    private val isTaskRunning: (String, String) -> Boolean,
    private val homeRows: () -> MainHomeRows,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val showCreateProjectDialog: () -> Unit,
    private val showAddFriendDialog: () -> Unit
) {
    fun renderConversationList() {
        val listVisible = binding.conversationPage.visibility == View.VISIBLE &&
            binding.chatPage.visibility != View.VISIBLE
        if (listVisible) {
            binding.topTitleText.text = "好友"
        }

        homeRows().cancelHomeRowShimmer()
        binding.conversationPage.removeAllViews()
        val friendList = friends()
        if (friendList.isEmpty()) {
            binding.conversationPage.addView(
                homeRows().createFriendPlaceholder(AuthManager.isLoggedIn(activity)) {
                    showAddFriendDialog()
                }
            )
            return
        }
        friendList.forEachIndexed { index, friend ->
            if (index > 0) {
                binding.conversationPage.addView(homeRows().createConversationDivider())
            }
            binding.conversationPage.addView(
                homeRows().createFriendRow(friend) {
                    Toast.makeText(activity, "好友会话准备中", Toast.LENGTH_SHORT).show()
                }
            )
        }
    }

    fun renderProjectList() {
        val container = binding.projectContentLayout
        container.removeAllViews()
        container.addView(createNewProjectRow())
        projects().forEachIndexed { index, project ->
            container.addView(homeRows().createProjectRow(index, project))
        }
    }

    fun isConversationWorking(index: Int): Boolean {
        val conversations = conversations()
        if (index !in conversations.indices || conversations[index].ended) return false
        return isTaskRunning(activeProject().id, conversations[index].id)
    }

    private fun createNewProjectRow(): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(52)
            ).apply {
                bottomMargin = dp(8)
            }
            setBackgroundColor(Color.parseColor("#202020"))
            gravity = Gravity.CENTER_VERTICAL
            text = "＋ 新建项目"
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 15f
            setPadding(dp(20), 0, dp(20), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { showCreateProjectDialog() }
        }
    }
}
