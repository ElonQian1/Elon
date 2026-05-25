package com.elon.app

import android.content.Intent
import android.content.SharedPreferences
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import com.google.gson.Gson

internal class MainAccountActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val projects: MutableList<AppProject>,
    private val gson: Gson,
    private val prefs: SharedPreferences,
    private val saveProjects: () -> Unit,
    private val renderProjectList: () -> Unit
) {
    fun refreshAccountUi() {
        val loggedIn = AuthManager.isLoggedIn(activity)
        binding.profileLoginButton.visibility = if (loggedIn) View.GONE else View.VISIBLE
        binding.profileLogoutButton.visibility = if (loggedIn) View.VISIBLE else View.GONE
        binding.profileImportGuestButton.visibility =
            if (loggedIn && importableGuestProjects().isNotEmpty()) View.VISIBLE else View.GONE
        binding.userInfoText.text = accountInfoText(activity)
    }

    fun checkAndOfferGuestImport() {
        if (!AuthManager.isLoggedIn(activity)) return
        val guestId = AuthManager.legacyAnonymousUserId(activity)
        val offerKey = "guest_import_offered_$guestId"
        if (prefs.getBoolean(offerKey, false)) return
        val importable = importableGuestProjects()
        if (importable.isEmpty()) return
        prefs.edit().putBoolean(offerKey, true).apply()
        AlertDialog.Builder(activity)
            .setTitle("发现游客记录")
            .setMessage("检测到本机游客状态下有 ${importable.size} 个项目，是否导入到当前账号？")
            .setPositiveButton("导入") { _, _ -> performGuestImport(importable) }
            .setNegativeButton("暂不导入", null)
            .show()
    }

    fun showGuestImportDialog() {
        val importable = importableGuestProjects()
        if (importable.isEmpty()) {
            Toast.makeText(activity, "没有可导入的游客记录", Toast.LENGTH_SHORT).show()
            return
        }
        AlertDialog.Builder(activity)
            .setTitle("导入游客记录")
            .setMessage("将导入 ${importable.size} 个游客项目到当前账号，是否继续？")
            .setPositiveButton("导入") { _, _ -> performGuestImport(importable) }
            .setNegativeButton("取消", null)
            .show()
    }

    fun confirmLogout() {
        AlertDialog.Builder(activity)
            .setTitle("退出登录")
            .setMessage("退出后将切换为游客模式。已经登录的项目数据仍保留在云端，可重新登录恢复。")
            .setPositiveButton("继续退出") { _, _ -> confirmLogoutStep2() }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun importableGuestProjects(): List<AppProject> {
        if (!AuthManager.isLoggedIn(activity)) return emptyList()
        val json = AuthManager.guestDataPrefs(activity).getString("projects_json", null) ?: return emptyList()
        val all = runCatching {
            gson.fromJson(json, Array<AppProject>::class.java)?.toList()
        }.getOrNull() ?: return emptyList()
        val existingIds = projects.map { it.id }.toSet()
        return all.filter { project ->
            project.id != "elon-self" &&
                project.id !in existingIds &&
                project.conversations.any { conversation ->
                    conversation.messages.any { message -> message.role == "user" }
                }
        }
    }

    private fun performGuestImport(importable: List<AppProject>) {
        var count = 0
        for (project in importable) {
            if (projects.none { it.id == project.id }) {
                projects.add(project)
                count++
            }
        }
        if (count > 0) {
            saveProjects()
            renderProjectList()
            refreshAccountUi()
            Toast.makeText(activity, "已导入 $count 个游客项目", Toast.LENGTH_SHORT).show()
        }
    }

    private fun confirmLogoutStep2() {
        AlertDialog.Builder(activity)
            .setTitle("再次确认")
            .setMessage("确认退出当前账号？")
            .setPositiveButton("确认退出") { _, _ -> performLogout() }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun performLogout() {
        AuthManager.clear(activity)
        val intent = Intent(activity, LoginActivity::class.java)
        intent.flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
        activity.startActivity(intent)
        activity.finish()
    }
}
