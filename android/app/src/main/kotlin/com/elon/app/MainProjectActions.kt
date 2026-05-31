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
    private val isLoggedIn: () -> Boolean,
    private val removeSentProjectShareCards: (Set<String>) -> Int
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
            if (!isJoint) add("邀请好友协作")
            if (isJoint) add("打开项目空间")
            if (isJoint) add("恢复为个人项目")
            if (isJoint) add("发布到项目商城")
            add("Git 仓库")
            add("协作权限 / 商城公开")
            if (projects.size > 1 && project.id != ELON_SELF_PROJECT_ID) add("删除项目")
        }.toTypedArray()

        AlertDialog.Builder(activity)
            .setTitle(project.title)
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "编辑项目名称" -> showRenameProjectDialog(index)
                    "邀请好友协作" -> confirmUpgradeToJoint(index)
                    "打开项目空间" -> openProjectSpace(project.projectSpaceId(), project.title)
                    "恢复为个人项目" -> confirmRestorePersonalProject(project)
                    "发布到项目商城" -> confirmPublishToMarketplace(project)
                    "Git 仓库" -> {
                        openProject(index)
                        showGitProjectDialog()
                    }
                    "协作权限 / 商城公开" -> showVisibilityDialog(project)
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
            .setTitle("邀请好友协作")
            .setMessage("「${project.title}」将同步至服务端并开启联合项目空间。\n\n这不会发布到项目商城；只有你把项目卡片发给好友或群聊后，收到卡片的人才能接受邀请共同开发。")
            .setPositiveButton("创建邀请") { _, _ -> doUpgradeToJoint(project) }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doUpgradeToJoint(project: AppProject) {
        val token = tokenProvider() ?: run {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        Toast.makeText(activity, "正在创建协作邀请...", Toast.LENGTH_SHORT).show()
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
                setProjectVisibility(http, serverUrl, created.id, true, "invite", token)
                activity.runOnUiThread {
                    project.markJointDevelopment(created.id, "invite")
                    saveProjects()
                    renderProjectList()
                    openProjectSpace(created.id, project.title)
                    Toast.makeText(activity, "已创建邀请协作项目", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "创建协作邀请失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun confirmPublishToMarketplace(project: AppProject) {
        AlertDialog.Builder(activity)
            .setTitle("发布到项目商城")
            .setMessage("「${project.title}」会出现在项目商城，所有用户都能看到并直接加入。\n\n如果只是邀请指定好友共同开发，请继续使用聊天里的项目卡片。")
            .setPositiveButton("发布商城") { _, _ -> doSetVisibility(project, true, "open") }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun confirmRestorePersonalProject(project: AppProject) {
        if (!project.isJointDevelopmentProject()) {
            Toast.makeText(activity, "已经是个人项目", Toast.LENGTH_SHORT).show()
            return
        }
        AlertDialog.Builder(activity)
            .setTitle("恢复为个人项目")
            .setMessage("「${project.title}」会从联合项目移回个人项目。\n\n如果它已经同步到服务端，会先设为私有，之后不会再作为联合项目空间打开。")
            .setPositiveButton("恢复") { _, _ -> doRestorePersonalProject(project) }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doRestorePersonalProject(project: AppProject) {
        val collaborationId = project.collaborationProjectId?.trim().orEmpty()
        val hasRemoteProject = collaborationId.isNotBlank() || project.id.startsWith("prj_")
        val remoteProjectId = collaborationId.ifBlank { project.id }
        val cardProjectIds = setOf(project.id, remoteProjectId).map { it.trim() }.filter { it.isNotEmpty() }.toSet()
        if (!hasRemoteProject) {
            project.markPersonalDevelopment()
            saveProjects()
            renderProjectList()
            val localRemoved = removeSentProjectShareCards(cardProjectIds)
            Toast.makeText(activity, restorePersonalProjectToast(localRemoved), Toast.LENGTH_SHORT).show()
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
        Thread {
            try {
                setProjectVisibility(http, serverUrl, remoteProjectId, false, "invite", token)
                val serverRemoved = cardProjectIds.sumOf { projectId ->
                    revokeProjectShareMessages(http, serverUrl, projectId, token)
                }
                activity.runOnUiThread {
                    project.markPersonalDevelopment()
                    saveProjects()
                    renderProjectList()
                    val localRemoved = removeSentProjectShareCards(cardProjectIds)
                    Toast.makeText(
                        activity,
                        restorePersonalProjectToast(maxOf(serverRemoved, localRemoved)),
                        Toast.LENGTH_SHORT
                    ).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(activity, "恢复个人项目失败：${e.message}", Toast.LENGTH_LONG).show()
                }
            }
        }.start()
    }

    private fun restorePersonalProjectToast(removedCards: Int): String {
        return if (removedCards > 0) {
            "已恢复为个人项目，并撤回 $removedCards 张项目卡片"
        } else {
            "已恢复为个人项目"
        }
    }

    private fun showVisibilityDialog(project: AppProject) {
        if (!isLoggedIn()) {
            Toast.makeText(activity, "请先登录再修改项目可见性", Toast.LENGTH_SHORT).show()
            return
        }
        val options = arrayOf(
            "邀请协作（仅收到项目卡片的人可加入）",
            "发布到商城（所有人可见并可加入）",
            "广场只读体验（可进入、可问 AI、不能改代码）",
            "商城展示但需审批",
            "私有（仅成员可见）"
        )
        AlertDialog.Builder(activity)
            .setTitle("${project.title} · 协作权限")
            .setItems(options) { _, which ->
                val (isPublic, joinMode) = when (which) {
                    0 -> true to "invite"
                    1 -> true to "open"
                    2 -> true to "readonly"
                    3 -> true to "approval"
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
                    joinMode == "invite" -> "已设为邀请协作"
                    joinMode == "open" -> "已发布到项目商城"
                    joinMode == "readonly" -> "已发布到项目广场（只读体验）"
                    else -> "已发布到项目商城（需审批）"
                }
                activity.runOnUiThread {
                    if (isPublic) {
                        project.markJointDevelopment(remoteProjectId, joinMode)
                        saveProjects()
                        renderProjectList()
                    } else if (project.isJointDevelopmentProject()) {
                        project.markJointDevelopment(remoteProjectId, "invite")
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
                setProjectVisibility(http, serverUrl, created.id, true, "invite", token)
                activity.runOnUiThread {
                    val existingIndex = projects.indexOfFirst { it.projectSpaceId() == created.id }
                    val index = if (existingIndex >= 0) {
                        existingIndex
                    } else {
                        projects.add(created.toJointAppProject())
                        projects.lastIndex
                    }
                    projects[index].markJointDevelopment(created.id, "invite")
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
        val project = projects[index]
        if (project.id == ELON_SELF_PROJECT_ID) {
            Toast.makeText(activity, "一龙项目是平台自身项目，不能删除", Toast.LENGTH_SHORT).show()
            return
        }
        AlertDialog.Builder(activity)
            .setTitle("删除项目")
            .setMessage("会先从服务器删除「${project.title}」的项目记录、工作区文件、附件、构建产物和会话 worktree；成功后才从本机移除。正在运行的开发任务需先结束。")
            .setNegativeButton("取消", null)
            .setPositiveButton("删除") { _, _ -> deleteProject(index) }
            .show()
    }

    private fun deleteProject(index: Int) {
        if (index !in projects.indices || projects.size <= 1) return
        val project = projects[index]
        val targetProjectId = project.projectSpaceId()
        val token = tokenProvider()
        val userId = AuthManager.effectiveUserId(activity)
        Toast.makeText(activity, "正在删除服务器项目...", Toast.LENGTH_SHORT).show()
        Thread {
            try {
                val remoteDeleted = deleteServerProject(
                    http = http,
                    serverUrl = serverUrl,
                    projectId = targetProjectId,
                    token = token,
                    userId = userId
                )
                activity.runOnUiThread {
                    removeLocalProject(project.id, targetProjectId)
                    val message = if (remoteDeleted) {
                        "项目已从服务器和本机移除"
                    } else {
                        "服务器没有找到对应项目，已移除本机记录"
                    }
                    Toast.makeText(activity, message, Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                activity.runOnUiThread {
                    Toast.makeText(
                        activity,
                        "删除失败：${e.message}。本机未移除，避免服务器残留。",
                        Toast.LENGTH_LONG
                    ).show()
                }
            }
        }.start()
    }

    private fun removeLocalProject(localProjectId: String, remoteProjectId: String) {
        val deleteIndex = projects.indexOfFirst {
            it.id == localProjectId || it.projectSpaceId() == remoteProjectId
        }
        if (deleteIndex !in projects.indices || projects.size <= 1) return
        projects.removeAt(deleteIndex)
        val currentActive = activeProjectIndexProvider()
        val activeProjectIndex = when {
            currentActive > deleteIndex -> currentActive - 1
            currentActive >= projects.size -> projects.lastIndex
            else -> currentActive
        }.coerceIn(0, projects.lastIndex)
        setActiveProjectIndex(activeProjectIndex)
        val activeProject = projects[activeProjectIndex]
        if (activeProject.conversations.isEmpty()) {
            activeProject.conversations.add(defaultAppConversation())
        }
        setActiveConversationIndex(activeProject.activeConversationIndex.coerceIn(0, activeProject.conversations.lastIndex))
        saveProjects()
        renderProjectList()
    }
}
