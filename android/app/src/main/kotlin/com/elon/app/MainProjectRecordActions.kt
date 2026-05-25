package com.elon.app

import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity

internal class MainProjectRecordActions(
    private val activity: AppCompatActivity,
    private val appName: () -> String,
    private val currentProjectTitle: () -> String,
    private val setCurrentProjectTitle: (String) -> Unit,
    private val activeProject: () -> AppProject,
    private val projectEvents: () -> MutableList<String>,
    private val currentStage: () -> String,
    private val conversationCount: () -> Int,
    private val currentTimeText: () -> String,
    private val currentStageHint: () -> String,
    private val saveProjects: () -> Unit,
    private val updateProjectViews: (String) -> Unit
) {
    fun compactProjectTitle(): String {
        return currentProjectTitle().trim().ifBlank { appName() }.take(6)
    }

    fun showProjectRecordDialog() {
        val events = projectEvents()
        val recent = if (events.isEmpty()) {
            "暂无进度记录"
        } else {
            events.take(12).joinToString("\n")
        }
        AlertDialog.Builder(activity)
            .setTitle("${currentProjectTitle()} · 项目记录")
            .setMessage("阶段：${currentStage()}\n会话：${conversationCount()} 个\n\n$recent")
            .setPositiveButton("知道了", null)
            .show()
    }

    fun addProjectEvent(text: String) {
        val events = projectEvents()
        events.add(0, "${currentTimeText()}  $text")
        while (events.size > 40) events.removeAt(events.size - 1)
        activeProject().updatedAt = System.currentTimeMillis()
        saveProjects()
        updateProjectViews(currentStageHint())
    }

    fun saveProjectTitle() {
        saveProjects()
    }

    fun updateProjectTitleFromRequest(text: String) {
        val project = activeProject()
        val shouldAutoName = project.title.startsWith("新项目") ||
            project.title == "一龙开发助手" ||
            project.title == "等待你的第一个开发需求"
        if (shouldAutoName) {
            setCurrentProjectTitle(summarize(text, 24))
        }
        project.subtitle = summarize(text, 34)
    }
}
