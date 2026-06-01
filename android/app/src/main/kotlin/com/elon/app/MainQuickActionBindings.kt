package com.elon.app

import android.content.Intent
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import com.elon.app.update.AppUpdateManager

internal class MainQuickActionBindings(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val fillPlanPrompt: () -> Unit,
    private val sendQuickCommand: (String) -> Unit,
    private val showProjectRecordDialog: () -> Unit,
    private val showGitProjectDialog: () -> Unit,
    private val openSettings: () -> Unit,
    private val openAgentCenter: () -> Unit,
    private val showPromotionDialog: () -> Unit,
    private val showGuestImportDialog: () -> Unit,
    private val confirmLogout: () -> Unit,
    private val openNodeMarket: (() -> Unit)? = null
) {
    fun setupQuickActions() {
        binding.quickPlanButton.setOnClickListener {
            fillPlanPrompt()
        }
        binding.quickContinueButton.setOnClickListener {
            sendQuickCommand("请继续完成上一次未完成的开发任务，并告诉我当前进度。")
        }
        binding.quickBuildButton.setOnClickListener {
            sendQuickCommand("请编译当前项目并生成 APK 下载链接。")
        }
        binding.quickHistoryButton.setOnClickListener { showProjectRecordDialog() }
        binding.quickSettingsButton.setOnClickListener { openSettings() }

        binding.projectContinueButton.setOnClickListener {
            sendQuickCommand("请继续当前项目的开发，并先说明下一步要做什么。")
        }
        binding.projectBuildButton.setOnClickListener {
            sendQuickCommand("请打包当前项目，生成可以下载安装到手机的 APK。")
        }
        binding.projectRecordButton.setOnClickListener { showProjectRecordDialog() }
        binding.projectGitButton.setOnClickListener { showGitProjectDialog() }
        binding.projectSettingsButton.setOnClickListener { openSettings() }
        binding.profileSettingsButton.setOnClickListener { openSettings() }
        binding.profileAgentButton.setOnClickListener { openAgentCenter() }
        binding.profileNodeMarketButton.setOnClickListener { openNodeMarket?.invoke() }
        binding.profileCheckUpdateButton.setOnClickListener {
            AppUpdateManager(activity).manualCheck()
        }
        binding.profileShareButton.setOnClickListener { showPromotionDialog() }
        binding.profileImportGuestButton.setOnClickListener { showGuestImportDialog() }
        binding.profileLoginButton.setOnClickListener {
            activity.startActivity(Intent(activity, LoginActivity::class.java))
        }
        binding.profileLogoutButton.setOnClickListener { confirmLogout() }
    }
}
