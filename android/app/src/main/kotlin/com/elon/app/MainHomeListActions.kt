package com.elon.app

import android.graphics.Color
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainHomeListActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val projects: () -> List<AppProject>,
    private val conversations: () -> List<AppConversation>,
    private val friends: () -> List<AppFriend>,
    private val groups: () -> List<AppGroup>,
    private val activeProject: () -> AppProject,
    private val compactProjectTitle: () -> String,
    private val formatTime: (Long) -> String,
    private val isTaskRunning: (String, String) -> Boolean,
    private val homeRows: () -> MainHomeRows,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> android.graphics.drawable.Drawable?,
    private val showCreateProjectDialog: () -> Unit,
    private val showProjectPlaza: () -> Unit,
    private val showAddFriendDialog: () -> Unit,
    private val openFriend: (AppFriend) -> Unit,
    private val openGroup: (AppGroup) -> Unit
) {
    fun renderConversationList() {
        val listVisible = binding.conversationPage.visibility == View.VISIBLE &&
            binding.chatPage.visibility != View.VISIBLE
        if (listVisible) {
            binding.topTitleText.text = "好友"
        }

        homeRows().cancelHomeRowShimmer()
        binding.conversationPage.removeAllViews()
        val chatItems = buildHomeChatItems()
        if (chatItems.isEmpty()) {
            binding.conversationPage.addView(
                homeRows().createFriendPlaceholder(AuthManager.isLoggedIn(activity)) {
                    showAddFriendDialog()
                }
            )
            return
        }
        chatItems.forEachIndexed { index, item ->
            if (index > 0) {
                binding.conversationPage.addView(homeRows().createConversationDivider())
            }
            when (item) {
                is HomeChatItem.FriendItem -> binding.conversationPage.addView(
                    homeRows().createFriendRow(item.friend) {
                        openFriend(item.friend)
                    }
                )
                is HomeChatItem.GroupItem -> binding.conversationPage.addView(
                    homeRows().createGroupRow(item.group) {
                        openGroup(item.group)
                    }
                )
            }
        }
    }

    private fun buildHomeChatItems(): List<HomeChatItem> {
        val friendItems = friends().map { friend ->
            HomeChatItem.FriendItem(
                friend = friend,
                sortTime = friend.lastMessageAt ?: 0L
            )
        }
        val groupItems = groups().map { group ->
            HomeChatItem.GroupItem(
                group = group,
                sortTime = group.lastMessageAt ?: group.createdAt ?: 0L
            )
        }
        return (groupItems + friendItems)
            .sortedWith(compareByDescending<HomeChatItem> { it.sortTime }.thenBy { item ->
                when (item) {
                    is HomeChatItem.FriendItem -> item.friend.name
                    is HomeChatItem.GroupItem -> item.group.name
                }
            })
    }

    private sealed class HomeChatItem(open val sortTime: Long) {
        data class FriendItem(val friend: AppFriend, override val sortTime: Long) : HomeChatItem(sortTime)
        data class GroupItem(val group: AppGroup, override val sortTime: Long) : HomeChatItem(sortTime)
    }

    fun renderProjectList() {
        val container = binding.projectContentLayout
        container.removeAllViews()
        container.addView(createProjectHeaderRow())
        val indexedProjects = projects().mapIndexed { index, project -> index to project }
        val personalProjects = indexedProjects.filter { (_, project) -> !project.isJointDevelopmentProject() }
        val jointProjects = indexedProjects.filter { (_, project) -> project.isJointDevelopmentProject() }

        container.addView(createProjectSectionTitle("个人独立项目"))
        if (personalProjects.isEmpty()) {
            container.addView(createEmptyProjectRow("暂无个人独立项目"))
        }
        personalProjects.forEach { (index, project) ->
            container.addView(homeRows().createProjectRow(index, project))
        }

        container.addView(createProjectSectionTitle("联合开发项目"))
        if (jointProjects.isEmpty()) {
            container.addView(createEmptyProjectRow("暂无联合开发项目"))
        }
        jointProjects.forEach { (index, project) ->
            container.addView(homeRows().createProjectRow(index, project))
        }
    }

    fun isConversationWorking(index: Int): Boolean {
        val conversations = conversations()
        if (index !in conversations.indices || conversations[index].ended) return false
        return isTaskRunning(activeProject().id, conversations[index].id)
    }

    private fun createProjectHeaderRow(): LinearLayout {
        return LinearLayout(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(64)
            ).apply {
                bottomMargin = dp(8)
            }
            orientation = LinearLayout.HORIZONTAL
            setBackgroundColor(Color.parseColor("#202020"))
            addView(
                createProjectHeaderButton("＋ 新建项目") {
                    showCreateProjectDialog()
                }
            )
            addView(View(activity).apply {
                layoutParams = LinearLayout.LayoutParams(dp(1), LinearLayout.LayoutParams.MATCH_PARENT).apply {
                    topMargin = dp(12)
                    bottomMargin = dp(12)
                }
                setBackgroundColor(Color.parseColor("#4A4A4A"))
            })
            addView(
                createProjectHeaderButton("项目广场") {
                    showProjectPlaza()
                }
            )
        }
    }

    private fun createProjectHeaderButton(label: String, onClick: () -> Unit): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                0,
                LinearLayout.LayoutParams.MATCH_PARENT,
                1f
            )
            gravity = Gravity.CENTER
            text = label
            setTextColor(Color.parseColor("#D0D0D0"))
            textSize = 16f
            setPadding(dp(20), 0, dp(20), 0)
            isClickable = true
            foreground = selectableForeground()
            setOnClickListener { onClick() }
        }
    }

    private fun createProjectSectionTitle(title: String): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(38)
            ).apply {
                topMargin = dp(10)
            }
            gravity = Gravity.CENTER_VERTICAL
            text = title
            setTextColor(Color.parseColor("#8E8E8E"))
            textSize = 13f
            setPadding(dp(20), 0, dp(20), 0)
        }
    }

    private fun createEmptyProjectRow(textValue: String): TextView {
        return TextView(activity).apply {
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dp(44)
            )
            gravity = Gravity.CENTER_VERTICAL
            text = textValue
            setTextColor(Color.parseColor("#6F6F6F"))
            textSize = 14f
            setPadding(dp(20), 0, dp(20), 0)
        }
    }
}
