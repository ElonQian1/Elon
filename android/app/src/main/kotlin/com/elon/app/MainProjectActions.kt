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

    fun showProjectActions(index: Int) {
        if (index !in projects.indices) return
        val project = projects[index]
        val visLabel = "设为公开 / 私有"
        val actions = if (projects.size <= 1) {
            arrayOf("编辑项目名称", "Git 仓库", visLabel)
        } else {
            arrayOf("编辑项目名称", "Git 仓库", visLabel, "删除项目")
        }

        AlertDialog.Builder(activity)
            .setTitle(project.title)
            .setItems(actions) { _, which ->
                when (actions[which]) {
                    "编辑项目名称" -> showRenameProjectDialog(index)
                    "Git 仓库" -> {
                        openProject(index)
                        showGitProjectDialog()
                    }
                    visLabel -> showVisibilityDialog(project)
                    "删除项目" -> confirmDeleteProject(index)
                }
            }
            .show()
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
        Thread {
            try {
                setProjectVisibility(http, serverUrl, project.id, isPublic, joinMode, token)
                val label = when {
                    !isPublic -> "已设为私有"
                    joinMode == "open" -> "已设为公开（直接加入）"
                    else -> "已设为公开（需审批）"
                }
                activity.runOnUiThread {
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
