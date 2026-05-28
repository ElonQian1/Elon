package com.elon.app

import android.widget.EditText
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient

internal class MainProjectActions(
    private val activity: AppCompatActivity,
    private val projects: MutableList<AppProject>,
    private val activeProjectIndexProvider: () -> Int,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val setActiveConversationIndex: (Int) -> Unit,
    private val titleEditText: (String) -> EditText,
    private val saveProjects: () -> Unit,
    private val renderProjectList: () -> Unit,
    private val openProject: (Int) -> Unit,
    private val openProjectSpace: (String, String) -> Unit,
    private val showGitProjectDialog: () -> Unit,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val tokenProvider: () -> String?,
    private val isLoggedIn: () -> Boolean
) {
    fun showCreateProjectDialog() {
        val input = titleEditText("新项目 ${projects.size + 1}")
        val dialog = AlertDialog.Builder(activity)
            .setTitle("新建项目")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("创建", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入项目名称"
                    return@setOnClickListener
                }
                createProject(title)
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    fun showCreateJointProjectDialog() {
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录后发起联合项目", Toast.LENGTH_SHORT).show()
            return
        }
        val input = titleEditText("联合项目 ${projects.size + 1}")
        val dialog = AlertDialog.Builder(activity)
            .setTitle("发起联合项目")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("创建", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入项目名称"
                    return@setOnClickListener
                }
                createJointProject(title)
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    fun showProjectActions(index: Int) {
        if (index !in projects.indices) return
        val project = projects[index]
        val isJoint = project.isJointDevelopmentProject()

        val actions = buildList {
            add("编辑项目名称")
            if (!isJoint) add("升级为联合项目")
            if (isJoint) add("打开项目空间")
            add("Git 仓库")
            add("设为公开 / 私有")
            if (projects.size > 1) add("删除项目")
        }.toTypedArray()

        AlertDialog.Builder(activity)
            .setTitle(project.title)
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "编辑项目名称" -> showRenameProjectDialog(index)
                    "升级为联合项目" -> confirmUpgradeToJoint(index)
                    "打开项目空间" -> openProjectSpace(project.projectSpaceId(), project.title)
                    "Git 仓库" -> {
                        openProject(index)
                        showGitProjectDialog()
                    }
                    "设为公开 / 私有" -> showVisibilityDialog(project)
                    "删除项目" -> confirmDeleteProject(index)
                }
            }
            .show()
    }

    private fun confirmUpgradeToJoint(index: Int) {
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录再升级为联合项目", Toast.LENGTH_SHORT).show()
            return
        }
        val project = projects.getOrNull(index) ?: return
        AlertDialog.Builder(activity)
            .setTitle("升级为联合项目")
            .setMessage("「${project.title}」将同步至服务端，开启频道空间（公告/讨论/需求/AI开发等），可与他人协作。\n\n升级后可通过「设为公开 / 私有」撤销公开状态。")
            .setPositiveButton("升级") { _, _ -> doUpgradeToJoint(project) }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doUpgradeToJoint(project: AppProject) {
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在升级为联合项目...", Toast.LENGTH_SHORT).show()
        Thread {
            try {
                val created = createStoreProject(
                    http = http,
                    serverUrl = serverUrl,
                    name = project.title,
                    description = project.subtitle.takeIf { it.isNotBlank() },
                    token = token,
                    ownerAccount = AuthManager.displayName(activity)
                )
                setProjectVisibility(http, serverUrl, created.id, true, "open", token)
                activity.runOnUiThread {
                    project.markJointDevelopment(created.id)
                    saveProjects()
                    renderProjectList()
                    openProjectSpace(created.id, project.title)
                    Toast.makeText(activity, "已升级为联合项目 🎉", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "升级失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun showVisibilityDialog(project: AppProject) {
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录再修改项目可见性", Toast.LENGTH_SHORT).show()
            return
        }
        val options = arrayOf("公开（任何人可加入）", "需审批加入", "私有（仅自己可见）")
        AlertDialog.Builder(activity)
            .setTitle("${project.title} · 可见性")
            .setItems(options) { _, which ->
                val (isPublic, joinMode) = when (which) {
                    0 -> true to "open"
                    1 -> true to "approval"
                    else -> false to "invite"
                }
                doSetVisibility(project, isPublic, joinMode)
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doSetVisibility(project: AppProject, isPublic: Boolean, joinMode: String) {
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "未登录", Toast.LENGTH_SHORT).show()
            return
        }
        if (!isPublic && !project.isJointDevelopmentProject()) {
            project.markPersonalDevelopment()
            saveProjects()
            renderProjectList()
            Toast.makeText(activity, "已设为私有", Toast.LENGTH_SHORT).show()
            return
        }
        Thread {
            try {
                val remoteProjectId = if (isPublic && !project.isJointDevelopmentProject()) {
                    val created = createStoreProject(
                        http = http,
                        serverUrl = serverUrl,
                        name = project.title,
                        description = project.subtitle.takeIf { it.isNotBlank() },
                        token = token,
                        ownerAccount = AuthManager.displayName(activity)
                    )
                    setProjectVisibility(http, serverUrl, created.id, true, joinMode, token)
                    created.id
                } else {
                    val targetId = project.projectSpaceId()
                    setProjectVisibility(http, serverUrl, targetId, isPublic, joinMode, token)
                    targetId
                }
                val label = when {
                    !isPublic -> "已设为私有"
                    joinMode == "open" -> "已设为公开（直接加入）"
                    else -> "已设为公开（需审批）"
                }
                activity.runOnUiThread {
                    if (isPublic) {
                        project.markJointDevelopment(remoteProjectId)
                        saveProjects()
                        renderProjectList()
                    }
                    Toast.makeText(activity, label, Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "修改失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun createProject(title: String) {
        projects.add(newAppProject(title, "新项目 · 点击进入会话"))
        setActiveProjectIndex(projects.lastIndex)
        setActiveConversationIndex(0)
        saveProjects()
        renderProjectList()
    }

    private fun createJointProject(title: String) {
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在创建联合项目...", Toast.LENGTH_SHORT).show()
        val ownerAccount = AuthManager.displayName(activity)
        Thread {
            try {
                val created = createStoreProject(
                    http = http,
                    serverUrl = serverUrl,
                    name = title,
                    description = "联合开发项目",
                    token = token,
                    ownerAccount = ownerAccount
                )
                setProjectVisibility(http, serverUrl, created.id, true, "open", token)
                activity.runOnUiThread {
                    val existingIndex = projects.indexOfFirst { it.projectSpaceId() == created.id }
                    val index = if (existingIndex >= 0) {
                        existingIndex
                    } else {
                        projects.add(created.toJointAppProject())
                        projects.lastIndex
                    }
                    projects[index].markJointDevelopment(created.id)
                    setActiveProjectIndex(index)
                    setActiveConversationIndex(0)
                    saveProjects()
                    renderProjectList()
                    openProjectSpace(created.id, created.name)
                    Toast.makeText(activity, "联合项目已创建", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "创建联合项目失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun showRenameProjectDialog(index: Int) {
        if (index !in projects.indices) return
        val project = projects[index]
        val input = titleEditText(project.title)
        val dialog = AlertDialog.Builder(activity)
            .setTitle("编辑项目名称")
            .setView(input)
            .setNegativeButton("取消", null)
            .setPositiveButton("保存", null)
            .create()

        dialog.setOnShowListener {
            dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val title = input.text.toString().trim()
                if (title.isBlank()) {
                    input.error = "请输入项目名称"
                    return@setOnClickListener
                }
                project.title = summarize(title, 24)
                project.updatedAt = System.currentTimeMillis()
                saveProjects()
                renderProjectList()
                dialog.dismiss()
            }
        }
        dialog.show()
        input.selectAll()
    }

    private fun confirmDeleteProject(index: Int) {
        if (index !in projects.indices || projects.size <= 1) return
        AlertDialog.Builder(activity)
            .setTitle("删除项目")
            .setMessage("删除后这个项目下的会话和进度记录会从本机移除。")
            .setNegativeButton("取消", null)
            .setPositiveButton("删除") { _, _ -> deleteProject(index) }
            .show()
    }

    private fun deleteProject(index: Int) {
        if (index !in projects.indices || projects.size <= 1) return
        projects.removeAt(index)
        val activeProjectIndex = activeProjectIndexProvider().coerceAtMost(projects.lastIndex)
        setActiveProjectIndex(activeProjectIndex)
        val activeProject = projects[activeProjectIndex]
        setActiveConversationIndex(activeProject.activeConversationIndex.coerceIn(0, activeProject.conversations.lastIndex))
        saveProjects()
        renderProjectList()
    }
}
