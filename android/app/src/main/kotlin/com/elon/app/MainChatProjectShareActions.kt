package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class MainChatProjectShareActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val projects: MutableList<AppProject>,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val saveProjects: () -> Unit,
    private val renderProjectList: () -> Unit,
    private val openLocalProject: (Int) -> Unit,
    private val openProjectSpace: (String, String) -> Unit,
    private val sendMessage: () -> Unit,
    private val isLoggedIn: () -> Boolean,
    private val tokenProvider: () -> String?
) {
    fun sendToCurrentChat(share: ChatProjectShare) {
        binding.inputEdit.setText(share.toMessageText())
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
        sendMessage()
    }

    fun handleCardAction(share: ChatProjectShare) {
        val existingIndex = projects.indexOfFirst { it.id == share.id }
        if (existingIndex >= 0) {
            activateAndOpen(existingIndex, share)
            return
        }
        if (share.source == "local") {
            val index = ensureProjectExists(share)
            Toast.makeText(activity, "已加入「${share.name}」", Toast.LENGTH_SHORT).show()
            activateAndOpenLocal(index)
            return
        }
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录后加入项目", Toast.LENGTH_SHORT).show()
            return
        }
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在加入项目...", Toast.LENGTH_SHORT).show()
        thread {
            val result = runCatching {
                joinStoreProject(http, serverUrl, share.id, token)
            }
            activity.runOnUiThread {
                result
                    .onSuccess {
                        val index = ensureProjectExists(share)
                        Toast.makeText(activity, "已加入「${share.name}」", Toast.LENGTH_SHORT).show()
                        activateAndOpenProjectSpace(index)
                    }
                    .onFailure { error ->
                        Toast.makeText(activity, error.message ?: "加入失败", Toast.LENGTH_LONG).show()
                    }
            }
        }
    }

    private fun ensureProjectExists(share: ChatProjectShare): Int {
        val existingIndex = projects.indexOfFirst { it.id == share.id }
        if (existingIndex >= 0) return existingIndex
        val project = newAppProject(share.name, share.description ?: "联合项目").copy(id = share.id)
        projects.add(project)
        saveProjects()
        renderProjectList()
        return projects.lastIndex
    }

    private fun activateAndOpen(index: Int, share: ChatProjectShare) {
        if (share.source == "local") {
            activateAndOpenLocal(index)
        } else {
            activateAndOpenProjectSpace(index)
        }
    }

    private fun activateAndOpenLocal(index: Int) {
        if (index !in projects.indices) return
        setActiveProjectIndex(index)
        saveProjects()
        openLocalProject(index)
    }

    private fun activateAndOpenProjectSpace(index: Int) {
        if (index !in projects.indices) return
        setActiveProjectIndex(index)
        saveProjects()
        val project = projects[index]
        openProjectSpace(project.id, project.title)
    }
}
