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
    private val deleteActiveChatMessage: (ChatMessage, () -> Unit) -> Unit,
    private val sendMessage: () -> Unit,
    private val isLoggedIn: () -> Boolean,
    private val tokenProvider: () -> String?
) {
    fun sendToCurrentChat(share: ChatProjectShare) {
        if (share.source == "local") {
            publishLocalProjectShare(share)
            return
        }
        sendShareMessage(share)
    }

    private fun sendShareMessage(share: ChatProjectShare) {
        binding.inputEdit.setText(share.toMessageText())
        binding.inputEdit.setSelection(binding.inputEdit.text.length)
        sendMessage()
    }

    fun handleCardAction(share: ChatProjectShare) {
        val existingIndex = findProjectIndex(share.id)
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

    fun revokePublishedShare(message: ChatMessage, share: ChatProjectShare) {
        if (message.role != "user") return
        deleteActiveChatMessage(message) {
            Toast.makeText(activity, "已撤销「${share.name}」发布", Toast.LENGTH_SHORT).show()
        }
    }

    fun restorePersonalProject(message: ChatMessage, share: ChatProjectShare) {
        if (message.role != "user") return
        val index = findProjectIndex(share.id)
        if (index < 0) {
            Toast.makeText(activity, "未找到本地项目", Toast.LENGTH_SHORT).show()
            return
        }
        val project = projects[index]
        if (!project.isJointDevelopmentProject()) {
            deleteActiveChatMessage(message) {
                Toast.makeText(activity, "已恢复为个人项目", Toast.LENGTH_SHORT).show()
            }
            return
        }
        val remoteProjectId = project.projectSpaceId()
        if (remoteProjectId == project.id || remoteProjectId.isBlank()) {
            Toast.makeText(activity, "该项目不能恢复为个人项目", Toast.LENGTH_SHORT).show()
            return
        }
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录后恢复个人项目", Toast.LENGTH_SHORT).show()
            return
        }
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在恢复个人项目...", Toast.LENGTH_SHORT).show()
        thread {
            val result = runCatching {
                setProjectVisibility(http, serverUrl, remoteProjectId, false, "request", token)
            }
            activity.runOnUiThread {
                result
                    .onSuccess {
                        val currentIndex = findProjectIndex(share.id)
                        if (currentIndex >= 0) {
                            projects[currentIndex].markPersonalDevelopment()
                            saveProjects()
                            renderProjectList()
                        }
                        deleteActiveChatMessage(message) {
                            Toast.makeText(activity, "已恢复为个人项目", Toast.LENGTH_SHORT).show()
                        }
                    }
                    .onFailure { error ->
                        Toast.makeText(
                            activity,
                            error.message ?: "恢复个人项目失败",
                            Toast.LENGTH_LONG
                        ).show()
                    }
            }
        }
    }

    private fun publishLocalProjectShare(share: ChatProjectShare) {
        val index = findProjectIndex(share.id)
        if (index < 0) {
            sendShareMessage(share)
            return
        }
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录后发布联合项目", Toast.LENGTH_SHORT).show()
            return
        }
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        val project = projects[index]
        if (project.isJointDevelopmentProject()) {
            sendShareMessage(project.toChatProjectShare().copy(ownerAccount = AuthManager.displayName(activity)))
            return
        }

        Toast.makeText(activity, "正在发布为联合项目...", Toast.LENGTH_SHORT).show()
        val ownerAccount = AuthManager.displayName(activity)
        thread {
            val result = runCatching {
                val created = createStoreProject(
                    http = http,
                    serverUrl = serverUrl,
                    name = project.title,
                    description = project.subtitle.takeIf { it.isNotBlank() },
                    token = token,
                    ownerAccount = ownerAccount
                )
                setProjectVisibility(http, serverUrl, created.id, true, "open", token)
                created
            }
            activity.runOnUiThread {
                result
                    .onSuccess { created ->
                        val currentIndex = findProjectIndex(share.id)
                        if (currentIndex >= 0) {
                            projects[currentIndex].markJointDevelopment(created.id)
                            saveProjects()
                            renderProjectList()
                        }
                        sendShareMessage(
                            created.toChatProjectShare().copy(
                                ownerAccount = ownerAccount,
                                latestLog = share.latestLog
                            )
                        )
                        Toast.makeText(activity, "已发布为联合项目", Toast.LENGTH_SHORT).show()
                    }
                    .onFailure { error ->
                        Toast.makeText(
                            activity,
                            error.message ?: "发布联合项目失败",
                            Toast.LENGTH_LONG
                        ).show()
                    }
            }
        }
    }

    private fun ensureProjectExists(share: ChatProjectShare): Int {
        val existingIndex = findProjectIndex(share.id)
        if (existingIndex >= 0) return existingIndex
        val project = newAppProject(share.name, share.description ?: "联合项目").copy(
            id = share.id,
            isJointProject = share.source != "local",
            collaborationProjectId = share.id.takeIf { share.source != "local" }
        )
        projects.add(project)
        saveProjects()
        renderProjectList()
        return projects.lastIndex
    }

    private fun activateAndOpen(index: Int, share: ChatProjectShare) {
        if (share.source == "local" && !projects[index].isJointDevelopmentProject()) {
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
        openProjectSpace(project.projectSpaceId(), project.title)
    }

    private fun findProjectIndex(projectId: String): Int {
        return projects.indexOfFirst {
            it.id == projectId || it.projectSpaceId() == projectId
        }
    }
}
