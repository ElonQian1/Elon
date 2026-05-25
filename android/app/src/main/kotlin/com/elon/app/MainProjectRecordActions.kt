package com.elon.app

internal class MainProjectRecordActions(
    private val appName: () -> String,
    private val currentProjectTitle: () -> String,
    private val setCurrentProjectTitle: (String) -> Unit,
    private val activeProject: () -> AppProject,
    private val projectEvents: () -> MutableList<String>,
    private val currentTimeText: () -> String,
    private val currentStageHint: () -> String,
    private val saveProjects: () -> Unit,
    private val updateProjectViews: (String) -> Unit
) {
    fun compactProjectTitle(): String {
        return currentProjectTitle().trim().ifBlank { appName() }.take(6)
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
