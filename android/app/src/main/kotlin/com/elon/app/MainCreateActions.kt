package com.elon.app

import android.content.Intent
import android.content.SharedPreferences
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.update.AppUpdateManager
import com.elon.app.update.PeerSeederManager
import com.elon.app.update.UpdateCheckWorker

internal class MainCreateActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val prefs: SharedPreferences,
    private val notificationPermissionRequest: Int,
    private val loadProjects: () -> Unit,
    private val setupAttachmentLaunchers: () -> Unit,
    private val activeConversation: () -> AppConversation,
    private val pauseCurrentWork: () -> Unit,
    private val showMessageActions: (View, ChatMessage) -> Unit,
    private val retryFailedAttachmentMessage: (ChatMessage) -> Unit,
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val setupNavigation: () -> Unit,
    private val setupQuickActions: () -> Unit,
    private val setupBackHandling: () -> Unit,
    private val setupInputComposer: () -> Unit,
    private val setupChatSideMenu: () -> Unit,
    private val restoreCachedModelSelection: () -> Unit,
    private val updateProjectViews: (String) -> Unit,
    private val setTaskAppForeground: (Boolean) -> Unit,
    private val registerTaskWorkReceiver: () -> Unit,
    private val restorePendingActiveWork: () -> Unit,
    private val checkAndOfferGuestImport: () -> Unit,
    private val syncProjectsFromServer: (((Boolean) -> Unit)?) -> Unit,
    private val getWaitingForReply: () -> Boolean,
    private val getBackendConnected: () -> Boolean,
    private val isActiveConversationWorking: () -> Boolean,
    private val startTaskWorkService: (String) -> Boolean,
    private val openConversation: (Int) -> Unit,
    private val loadModelOptions: () -> Unit,
    private val sendMessage: () -> Unit
) {
    fun onCreate(intent: Intent?) {
        setupTaskCompletionAlerts(activity, prefs, notificationPermissionRequest)
        ChatRealtimeService.ensureRunning(activity)
        loadProjects()
        setupAttachmentLaunchers()
        val adapter = ChatAdapter(activeConversation().messages, pauseCurrentWork, showMessageActions, retryFailedAttachmentMessage)
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        setupNavigation()
        setupQuickActions()
        setupBackHandling()
        setupInputComposer()
        setupChatSideMenu()
        restoreCachedModelSelection()
        updateProjectViews("像聊天一样发需求，我会同步整理开发进度和项目记录。")
        setTaskAppForeground(true)
        registerTaskWorkReceiver()
        restorePendingActiveWork()
        checkAndOfferGuestImport()
        setupProjectPullRefresh()
        syncProjectsFromServer(null)
        startTaskWorkService(
            if (getWaitingForReply()) TaskWorkService.ACTION_RESUME_PENDING else TaskWorkService.ACTION_CONNECT
        )
        bindStatusReconnect()
        loadModelOptions()
        bindEditorSendAction()
        AppUpdateManager(activity).autoCheck()
        UpdateCheckWorker.schedule(activity)
        PeerSeederManager.start(activity)
        handleLaunchIntent(intent)
    }

    fun handleLaunchIntent(intent: Intent?) {
        if (intent?.getBooleanExtra(TaskWorkService.EXTRA_SHOW_APP_UPDATE, false) == true) {
            intent.removeExtra(TaskWorkService.EXTRA_SHOW_APP_UPDATE)
            AppUpdateManager(activity).realtimeCheck()
        }
    }

    private fun bindStatusReconnect() {
        binding.statusText.setOnClickListener {
            if (getBackendConnected() || !isActiveConversationWorking()) {
                openConversation(0)
            } else {
                startTaskWorkService(TaskWorkService.ACTION_CONNECT)
            }
        }
    }

    private fun setupProjectPullRefresh() {
        binding.projectPage.setColorSchemeResources(R.color.elon_button_primary_bg)
        binding.projectPage.setProgressBackgroundColorSchemeResource(R.color.elon_surface_card)
        binding.projectPage.setOnRefreshListener {
            if (!AuthManager.isLoggedIn(activity)) {
                binding.projectPage.isRefreshing = false
                Toast.makeText(activity, "请先登录后同步项目", Toast.LENGTH_SHORT).show()
                return@setOnRefreshListener
            }
            syncProjectsFromServer { ok ->
                binding.projectPage.isRefreshing = false
                Toast.makeText(
                    activity,
                    if (ok) "项目已刷新" else "刷新失败，请稍后再试",
                    Toast.LENGTH_SHORT
                ).show()
            }
        }
    }

    private fun bindEditorSendAction() {
        binding.inputEdit.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_SEND) {
                sendMessage()
                true
            } else {
                false
            }
        }
    }
}
