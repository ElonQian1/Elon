package com.elon.app

import android.widget.EditText
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.elon.app.databinding.ActivityMainBinding

internal class MainProjectActions(
    private val activity: AppCompatActivity,
    private val binding: ActivityMainBinding,
    private val projects: MutableList<AppProject>,
    private val activeProjectIndexProvider: () -> Int,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val setActiveConversationIndex: (Int) -> Unit,
    private val titleEditText: (String) -> EditText,
    private val saveProjects: () -> Unit,
    private val renderProjectList: () -> Unit,
    private val openProject: (Int) -> Unit,
    private val showGitProjectDialog: () -> Unit
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
        val actions = if (projects.size <= 1) {
            arrayOf("编辑项目名称", "Git 仓库")
        } else {
            arrayOf("编辑项目名称", "Git 仓库", "删除项目")
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
                    "删除项目" -> confirmDeleteProject(index)
                }
            }
            .show()
    }

    private fun createProject(title: String) {
        projects.add(newAppProject(title, "新项目 · 点击进入会话"))
        setActiveProjectIndex(projects.lastIndex)
        setActiveConversationIndex(0)
        saveProjects()
        renderProjectList()
        binding.tabChat.performClick()
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
