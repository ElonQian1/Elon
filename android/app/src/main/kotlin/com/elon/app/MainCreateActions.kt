package com.elon.app

import android.content.Intent
import android.content.SharedPreferences
import android.view.View
import android.view.inputmethod.EditorInfo
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
    private val setChatAdapter: (ChatAdapter) -> Unit,
    private val setupNavigation: () -> Unit,
    private val setupQuickActions: () -> Unit,
    private val setupBackHandling: () -> Unit,
    private val setupInputComposer: () -> Unit,
    private val restoreCachedModelSelection: () -> Unit,
    private val updateProjectViews: (String) -> Unit,
    private val setTaskAppForeground: (Boolean) -> Unit,
    private val registerTaskWorkReceiver: () -> Unit,
    private val restorePendingActiveWork: () -> Unit,
    private val checkAndOfferGuestImport: () -> Unit,
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
        loadProjects()
        setupAttachmentLaunchers()
        val adapter = ChatAdapter(activeConversation().messages, pauseCurrentWork, showMessageActions)
        setChatAdapter(adapter)
        binding.chatList.adapter = adapter
        setupNavigation()
        setupQuickActions()
        setupBackHandling()
        setupInputComposer()
        restoreCachedModelSelection()
        updateProjectViews("像聊天一样发需求，我会同步整理开发进度和项目记录。")
        setTaskAppForeground(true)
        registerTaskWorkReceiver()
        restorePendingActiveWork()
        checkAndOfferGuestImport()
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
