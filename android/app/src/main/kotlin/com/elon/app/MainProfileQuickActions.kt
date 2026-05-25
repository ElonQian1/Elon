package com.elon.app

import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient

internal class MainProfileQuickActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverVersionUrl: String,
    private val isBindingInitialized: () -> Boolean,
    private val refreshAccountUi: () -> Unit,
    private val fillPlanPrompt: () -> Unit,
    private val sendQuickCommand: (String) -> Unit,
    private val showProjectRecordDialog: () -> Unit,
    private val showGitProjectDialog: () -> Unit,
    private val openSettings: () -> Unit,
    private val showPromotionDialog: () -> Unit,
    private val showGuestImportDialog: () -> Unit,
    private val confirmLogout: () -> Unit
) {
    fun setupQuickActions() {
        MainQuickActionBindings(
            activity = activity,
            binding = binding,
            fillPlanPrompt = fillPlanPrompt,
            sendQuickCommand = sendQuickCommand,
            showProjectRecordDialog = showProjectRecordDialog,
            showGitProjectDialog = showGitProjectDialog,
            openSettings = openSettings,
            showPromotionDialog = showPromotionDialog,
            showGuestImportDialog = showGuestImportDialog,
            confirmLogout = confirmLogout
        ).setupQuickActions()
        refreshAccountUi()
        binding.profileVersionText.text =
            "${localAppVersionLine()}\n服务器版本读取中..."
        refreshServerVersion()
    }

    fun refreshServerVersion() {
        Thread {
            val info = fetchServerVersionInfo(http, serverVersionUrl)
            val serverLine = info?.let { serverVersionLine(it) } ?: "服务器版本暂不可用"
            activity.runOnUiThread {
                if (isBindingInitialized()) {
                    binding.profileVersionText.text = "${localAppVersionLine()}\n$serverLine"
                }
            }
        }.start()
    }
}
