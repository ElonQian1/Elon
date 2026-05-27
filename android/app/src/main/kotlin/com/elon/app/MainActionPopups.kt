package com.elon.app

import android.graphics.drawable.Drawable
import android.view.View
import android.widget.PopupWindow
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainActionPopups(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val getActionPopup: () -> PopupWindow?,
    private val setActionPopup: (PopupWindow?) -> Unit,
    private val shareActions: () -> MainShareActions,
    private val fillPlanPrompt: () -> Unit,
    private val sendQuickCommand: (String) -> Unit,
    private val showProjectRecordDialog: () -> Unit,
    private val showGitProjectDialog: () -> Unit,
    private val showCreateProjectDialog: () -> Unit,
    private val showCreateGroupDialog: () -> Unit,
    private val showAddFriendDialog: () -> Unit,
    private val openSettings: () -> Unit,
    private val deleteMessage: (ChatMessage) -> Unit,
    private val startMultiSelect: (ChatMessage) -> Unit,
    private val revokeProjectShare: (ChatMessage, ChatProjectShare) -> Unit,
    private val restorePersonalProject: (ChatMessage, ChatProjectShare) -> Unit,
    private val quoteMessage: (String) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val showStoreDialog: () -> Unit
) {
    fun showHomeActionPopup(anchor: View, tab: TextView) {
        val actions = if (tab == binding.tabProject) {
            listOf(
                TopAction("新建项目", R.drawable.ic_popup_project) { showCreateProjectDialog() },
                TopAction("发现项目", R.drawable.ic_popup_project) { showStoreDialog() },
                TopAction("项目记录", R.drawable.ic_popup_history) { showProjectRecordDialog() },
                TopAction("Git 仓库", R.drawable.ic_popup_settings) { showGitProjectDialog() },
                TopAction("打包 APK", R.drawable.ic_popup_build) { sendQuickCommand("请打包当前项目，生成可以下载安装到手机的 APK。") },
                TopAction("AI 设置", R.drawable.ic_popup_settings) { openSettings() }
            )
        } else {
            listOf(
                TopAction("发起群聊", R.drawable.ic_popup_group) { showCreateGroupDialog() },
                TopAction("添加好友", R.drawable.ic_popup_add_friend) { showAddFriendDialog() },
                TopAction("新建项目", R.drawable.ic_popup_project) { showCreateProjectDialog() },
                TopAction("继续开发", R.drawable.ic_popup_plan) { sendQuickCommand("请继续完成上一次未完成的开发任务，并告诉我当前进度。") },
                TopAction("AI 设置", R.drawable.ic_popup_settings) { openSettings() }
            )
        }
        showTopActionPopup(anchor, actions)
    }

    fun showChatActionPopup(anchor: View) {
        showTopActionPopup(
            anchor,
            listOf(
                TopAction("需求规划", R.drawable.ic_popup_plan) { fillPlanPrompt() },
                TopAction("继续开发", R.drawable.ic_popup_chat) { sendQuickCommand("请继续完成上一次未完成的开发任务，并告诉我当前进度。") },
                TopAction("打包 APK", R.drawable.ic_popup_build) { sendQuickCommand("请编译当前项目并生成 APK 下载链接。") },
                TopAction("项目记录", R.drawable.ic_popup_history) { showProjectRecordDialog() },
                TopAction("AI 设置", R.drawable.ic_popup_settings) { openSettings() }
            )
        )
    }

    fun showMessageActionPopup(anchor: View, message: ChatMessage, text: String) {
        val actions = listOf(
            TopAction("复制", R.drawable.ic_msg_copy) { shareActions().copyMessageText(text) },
            TopAction("转发", R.drawable.ic_msg_forward) { shareActions().forwardMessageText(text) },
            TopAction("收藏", R.drawable.ic_msg_favorite) { shareActions().toastMessageAction("已收藏") },
            TopAction("删除", R.drawable.ic_msg_delete) { deleteMessage(message) },
            TopAction("多选", R.drawable.ic_msg_multi) { startMultiSelect(message) },
            TopAction("引用", R.drawable.ic_msg_quote) { quoteMessage(text) },
            TopAction("提醒", R.drawable.ic_msg_remind) { shareActions().toastMessageAction("提醒准备中") },
            TopAction("搜一搜", R.drawable.ic_msg_search) { shareActions().searchMessageText(text) },
            TopAction("从当前听", R.drawable.ic_msg_listen) { shareActions().toastMessageAction("从当前听准备中") }
        )
        setActionPopup(renderer().showMessageActionPopup(anchor, getActionPopup(), actions))
    }

    fun showProjectShareActionPopup(anchor: View, message: ChatMessage, share: ChatProjectShare) {
        val actions = listOf(
            TopAction("撤销", R.drawable.ic_msg_delete) {
                revokeProjectShare(message, share)
            },
            TopAction("恢复", R.drawable.ic_popup_project) {
                restorePersonalProject(message, share)
            }
        )
        setActionPopup(renderer().showProjectCardActionPopup(anchor, getActionPopup(), actions))
    }

    private fun showTopActionPopup(anchor: View, actions: List<TopAction>) {
        setActionPopup(renderer().showTopActionPopup(anchor, getActionPopup(), actions))
    }

    private fun renderer(): MainActionPopupRenderer {
        return MainActionPopupRenderer(
            activity = activity,
            dp = dp,
            selectableForeground = selectableForeground
        )
    }
}
