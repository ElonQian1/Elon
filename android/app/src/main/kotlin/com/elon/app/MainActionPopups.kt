package com.elon.app

import android.graphics.drawable.Drawable
import android.view.View
import android.view.animation.DecelerateInterpolator
import android.widget.PopupWindow
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
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
    private val canRecallMessage: (ChatMessage) -> Boolean,
    private val recallMessage: (ChatMessage) -> Unit,
    private val deleteMessage: (ChatMessage) -> Unit,
    private val startMultiSelect: (ChatMessage) -> Unit,
    private val revokeProjectShare: (ChatMessage, ChatProjectShare) -> Unit,
    private val quoteMessage: (ChatMessage) -> Unit,
    private val favoriteMessage: (ChatMessage) -> Unit,
    private val canRequestAiReply: () -> Boolean,
    private val requestAiReply: (ChatMessage) -> Unit,
    private val dp: (Int) -> Int,
    private val selectableForeground: () -> Drawable?,
    private val showStoreDialog: () -> Unit,
    private val groupRevisionActions: (ChatMessage) -> List<TopAction> = { emptyList() },
    private val shareAiMessage: ((ChatMessage, () -> Unit) -> Boolean)? = null,
) {
    fun showHomeActionPopup(anchor: View, tab: TextView) {
        val actions = if (tab == binding.tabProject) {
            listOf(
                TopAction("新建项目", R.drawable.ic_home_action_new_project) { showCreateProjectDialog() },
                TopAction("发现项目", R.drawable.ic_popup_project) { showStoreDialog() },
                TopAction("项目记录", R.drawable.ic_popup_history) { showProjectRecordDialog() },
                TopAction("Git 仓库", R.drawable.ic_popup_settings) { showGitProjectDialog() },
                TopAction("打包 APK", R.drawable.ic_popup_build) { sendQuickCommand("请打包当前项目，生成可以下载安装到手机的 APK。") },
                TopAction("AI 设置", R.drawable.ic_popup_settings) { openSettings() }
            )
        } else {
            listOf(
                TopAction("发起群聊", R.drawable.ic_home_action_group) { showCreateGroupDialog() },
                TopAction("添加好友", R.drawable.ic_home_action_add_friend) { showAddFriendDialog() },
                TopAction("新建项目", R.drawable.ic_home_action_new_project) { showCreateProjectDialog() }
            )
        }
        val popup = if (anchor === binding.bottomComposeButton) {
            val rendered = renderer().showBottomActionPopup(anchor, getActionPopup(), actions)
            setActionPopup(rendered)
            rendered
        } else {
            showTopActionPopup(anchor, actions)
        }
        if (anchor === binding.addButton || anchor === binding.bottomComposeButton) {
            setHomeAddButtonExpanded(anchor, true)
            popup.setOnDismissListener {
                setHomeAddButtonExpanded(anchor, false)
                if (getActionPopup() === popup) setActionPopup(null)
            }
        } else {
            popup.setOnDismissListener {
                if (getActionPopup() === popup) setActionPopup(null)
            }
        }
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
        val contentActions = ChatMessageContentActions(message, text)
        val isRecord = contentActions.isRecord
        val hasText = contentActions.hasText
        val actions = mutableListOf<TopAction>()
        if (!isRecord) actions.addAll(groupRevisionActions(message))
        if (contentActions.canCopy) {
            actions.add(TopAction("复制", R.drawable.ic_msg_copy) { shareActions().copyMessage(message, text) })
        }
        if (contentActions.canForward) {
            actions.add(TopAction("转发", R.drawable.ic_msg_forward) {
                val plain = { shareActions().forwardMessage(message, text); Unit }
                if (shareAiMessage?.invoke(message, plain) != true) plain()
            })
        }
        if (hasText || !message.attachments.isNullOrEmpty()) {
            actions.add(TopAction("收藏", R.drawable.ic_msg_favorite) {
                favoriteMessage(message)
                shareActions().toastMessageAction("已收藏")
            })
        }
        actions.add(TopAction("时间", R.drawable.ic_msg_time) { showMessageTime(message) })
        if (canRecallMessage(message)) {
            actions.add(TopAction("撤回", R.drawable.ic_msg_delete) { recallMessage(message) })
        }
        actions.add(TopAction("删除", R.drawable.ic_msg_delete) { deleteMessage(message) })
        actions.add(TopAction("多选", R.drawable.ic_msg_multi) { startMultiSelect(message) })
        if (hasText || isRecord || !message.attachments.isNullOrEmpty()) actions.add(TopAction("引用", R.drawable.ic_msg_quote) { quoteMessage(message) })
        if (hasText) {
            actions.add(TopAction("提醒", R.drawable.ic_msg_remind) { shareActions().toastMessageAction("提醒准备中") })
            actions.add(TopAction("搜一搜", R.drawable.ic_msg_search) { shareActions().searchMessageText(text) })
            actions.add(TopAction("从当前听", R.drawable.ic_msg_listen) { shareActions().toastMessageAction("从当前听准备中") })
        }
        if (contentActions.canAnalyze && canRequestAiReply()) {
            actions.add(TopAction(if (isRecord) "AI分析记录" else "AI回复", R.drawable.ic_msg_ai_reply) { requestAiReply(message) })
        }
        if (contentActions.images.isNotEmpty()) actions.add(TopAction("识别二维码", R.drawable.ic_msg_search) {
            val images = contentActions.images
            fun recognize(index: Int) = com.elon.app.sharing.SourceLinkViews.recognizeImage(anchor, images[index])
            if (images.size == 1) recognize(0)
            else AlertDialog.Builder(activity).setTitle("选择图片")
                .setItems(images.mapIndexed { index, item -> item.displayName ?: "图片 ${index + 1}" }.toTypedArray()) { _, index -> recognize(index) }
                .setNegativeButton("取消", null).show()
        })
        setActionPopup(renderer().showMessageActionPopup(anchor, getActionPopup(), actions))
    }

    private fun showMessageTime(message: ChatMessage) {
        AlertDialog.Builder(activity)
            .setTitle("消息时间")
            .setMessage(formatChatMessageExactTime(message.createdAtMs))
            .setPositiveButton("知道了", null)
            .show()
    }

    fun showProjectShareActionPopup(anchor: View, message: ChatMessage, share: ChatProjectShare) {
        val actions = listOf(
            TopAction("撤销", R.drawable.ic_msg_delete) {
                revokeProjectShare(message, share)
            }
        )
        setActionPopup(renderer().showProjectCardActionPopup(anchor, getActionPopup(), actions))
    }

    private fun showTopActionPopup(anchor: View, actions: List<TopAction>): PopupWindow {
        val popup = renderer().showTopActionPopup(anchor, getActionPopup(), actions)
        setActionPopup(popup)
        return popup
    }

    private fun setHomeAddButtonExpanded(anchor: View, expanded: Boolean) {
        val icon = if (anchor === binding.bottomComposeButton) {
            binding.bottomActionPlusIcon
        } else {
            binding.addButtonPlus
        }
        icon.animate().cancel()
        icon.animate()
            .rotation(if (expanded) 45f else 0f)
            .setDuration(160L)
            .setInterpolator(DecelerateInterpolator())
            .start()
    }

    private fun renderer(): MainActionPopupRenderer {
        return MainActionPopupRenderer(
            activity = activity,
            dp = dp,
            selectableForeground = selectableForeground
        )
    }
}
