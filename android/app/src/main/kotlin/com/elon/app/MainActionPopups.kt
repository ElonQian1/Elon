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
    private val showCreateConversationDialog: () -> Unit,
    private val openSettings: () -> Unit,
    private val deleteMessage: (ChatMessage) -> Unit,
    private val quoteMessage: (String) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?
) {
    fun showMoreActions() {
        showChatActionPopup(binding.moreButton)
    }

    fun showHomeActionPopup(anchor: View, tab: TextView) {
        val actions = if (tab == binding.tabProject) {
            listOf(
                TopAction("新建项目", R.drawable.ic_popup_project) { showCreateProjectDialog() },
                TopAction("项目记录", R.drawable.ic_popup_history) { showProjectRecordDialog() },
                TopAction("Git 仓库", R.drawable.ic_popup_settings) { showGitProjectDialog() },
                TopAction("打包 APK", R.drawable.ic_popup_build) { sendQuickCommand("请打包当前项目，生成可以下载安装到手机的 APK。") },
                TopAction("AI 设置", R.drawable.ic_popup_settings) { openSettings() }
            )
        } else {
            listOf(
                TopAction("新建会话", R.drawable.ic_popup_chat) { showCreateConversationDialog() },
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
            TopAction("多选", R.drawable.ic_msg_multi) { shareActions().toastMessageAction("多选准备中") },
            TopAction("引用", R.drawable.ic_msg_quote) { quoteMessage(text) },
            TopAction("提醒", R.drawable.ic_msg_remind) { shareActions().toastMessageAction("提醒准备中") },
            TopAction("搜一搜", R.drawable.ic_msg_search) { shareActions().searchMessageText(text) },
            TopAction("从当前听", R.drawable.ic_msg_listen) { shareActions().toastMessageAction("从当前听准备中") }
        )
        setActionPopup(renderer().showMessageActionPopup(anchor, getActionPopup(), actions))
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
