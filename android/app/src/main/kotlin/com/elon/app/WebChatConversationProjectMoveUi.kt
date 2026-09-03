package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.chatgptweb.ChatGptWebProject
import com.google.android.material.snackbar.Snackbar

internal class WebChatConversationProjectMoveUi(
    private val activity: AppCompatActivity,
    private val host: android.view.View,
) {
    private var activeSheet: WebChatActionSheetHandle? = null
    private var activeDialog: AlertDialog? = null
    private var activeProgress: Snackbar? = null
    private var destinationsById = emptyMap<String, ChatGptWebProject>()
    private val sheetLease = WebChatConversationProjectMoveSheetLease()

    fun showDestinationPicker(
        destinations: List<ChatGptWebProject>,
        onCancelled: () -> Unit,
        onSelected: (ChatGptWebProject) -> Unit,
        openOfficialFallback: () -> Unit,
    ) {
        dismissProgress()
        destinationsById = destinations.associateBy(ChatGptWebProject::id)
        val lease = sheetLease.issue()
        var selectedDestination: ChatGptWebProject? = null
        activeSheet = WebChatActionSheet.showUpdatable(
            activity = activity,
            title = "移动到项目",
            items = destinations.map { project ->
                WebChatActionSheetItem(
                    id = project.id,
                    title = project.title,
                    subtitle = if (project.active) "当前打开" else null,
                    contentDescription =
                        "web-chat-conversation-project-destination:${project.id}",
                )
            },
            footerActions = listOf(
                WebChatActionSheetFooterAction(
                    label = "官网完成",
                    contentDescription = "web-chat-conversation-project-move-official",
                    action = openOfficialFallback,
                ),
            ),
            onCancelled = {
                if (sheetLease.owns(lease)) onCancelled()
            },
            onDismissed = {
                if (sheetLease.owns(lease)) {
                    activeSheet = null
                    destinationsById = emptyMap()
                    selectedDestination?.let { destination ->
                        host.postDelayed({
                            if (sheetLease.owns(lease)) onSelected(destination)
                        }, ACTION_SHEET_HANDOFF_SETTLE_MS)
                    }
                }
            },
        ) { item ->
            selectedDestination = destinationsById[item.id]
            destinationsById = emptyMap()
        }
    }

    fun showProgress(destination: ChatGptWebProject, subtitle: String) {
        dismissDestinationPicker()
        dismissProgress()
        activeProgress = Snackbar.make(
            host,
            progressText(destination, subtitle),
            Snackbar.LENGTH_INDEFINITE,
        ).apply {
            setTextMaxLines(2)
            view.contentDescription = "web-chat-conversation-project-move-progress"
            show()
        }
    }

    fun updateProgress(destination: ChatGptWebProject, subtitle: String) {
        activeProgress?.setText(progressText(destination, subtitle))
            ?: showProgress(destination, subtitle)
    }

    fun complete(destination: ChatGptWebProject) {
        dismissDestinationPicker()
        dismissProgress()
        Toast.makeText(activity, "已移动到“${destination.title}”", Toast.LENGTH_SHORT).show()
    }

    fun showDraftBlocked() {
        if (activity.isFinishing || activity.isDestroyed) return
        dismissDestinationPicker()
        dismissProgress()
        trackDialog(WebChatConversationDraftNavigation.dialog(activity))
    }

    fun showFailure(
        attempted: Boolean,
        detail: String,
        onOfficialFallback: () -> Unit,
        onRefresh: () -> Unit,
        onRetry: () -> Unit,
    ) {
        if (activity.isFinishing || activity.isDestroyed) return
        dismissDestinationPicker()
        dismissProgress()
        val message = if (attempted) {
            "$detail。已经提交过一次操作，为避免重复移动，应用不会自动重试。可以刷新目录或在官网确认。"
        } else {
            "$detail。尚未提交移动操作，可以重试或在官网完成。"
        }
        val builder = AlertDialog.Builder(activity)
            .setTitle(if (attempted) "移动结果待确认" else "暂时无法移动")
            .setMessage(message)
            .setPositiveButton("官网确认") { _, _ -> onOfficialFallback() }
            .setNegativeButton("取消", null)
        if (attempted) {
            builder.setNeutralButton("刷新目录") { _, _ -> onRefresh() }
        } else {
            builder.setNeutralButton("重试") { _, _ -> onRetry() }
        }
        trackDialog(builder.create())
    }

    fun showPendingRecovery(
        onRefresh: () -> Unit,
        onOfficialFallback: () -> Unit,
    ) {
        if (activity.isFinishing || activity.isDestroyed || activeDialog != null) return
        trackDialog(
            AlertDialog.Builder(activity)
                .setTitle("移动结果待确认")
                .setMessage("应用已经提交过一次移动操作，不会重复提交。请刷新目录确认结果，或打开官网查看。")
                .setPositiveButton("刷新目录") { _, _ -> onRefresh() }
                .setNeutralButton("官网确认") { _, _ -> onOfficialFallback() }
                .setNegativeButton("稍后", null)
                .create(),
        )
    }

    fun showNoDestinations(
        onRefresh: () -> Unit,
        onOfficialFallback: () -> Unit,
    ) {
        if (activity.isFinishing || activity.isDestroyed) return
        trackDialog(
            AlertDialog.Builder(activity)
                .setTitle("没有其他项目")
                .setMessage("项目目录已经保留在本机。创建或同步其他项目后即可移动会话。")
                .setNeutralButton("刷新目录") { _, _ -> onRefresh() }
                .setPositiveButton("官网查看") { _, _ -> onOfficialFallback() }
                .setNegativeButton("取消", null)
                .create(),
        )
    }

    fun dismissAll() {
        dismissDestinationPicker()
        dismissProgress()
        activeDialog?.dismiss()
        activeDialog = null
        destinationsById = emptyMap()
    }

    private fun trackDialog(dialog: AlertDialog) {
        activeDialog = dialog
        dialog.setOnDismissListener {
            if (activeDialog === dialog) activeDialog = null
        }
        dialog.show()
    }

    private fun dismissDestinationPicker() {
        sheetLease.invalidate()
        val sheet = activeSheet
        activeSheet = null
        sheet?.dismiss()
    }

    private fun dismissProgress() {
        activeProgress?.dismiss()
        activeProgress = null
    }

    private fun progressText(destination: ChatGptWebProject, subtitle: String): String =
        "${destination.title} · $subtitle"

    private companion object {
        const val ACTION_SHEET_HANDOFF_SETTLE_MS = 48L
    }
}
