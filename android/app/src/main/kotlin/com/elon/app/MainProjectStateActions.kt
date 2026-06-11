package com.elon.app

import android.content.SharedPreferences
import com.google.gson.Gson

internal class MainProjectStateActions(
    private val prefs: SharedPreferences,
    private val gson: Gson,
    private val projects: MutableList<AppProject>,
    private val activeProjectIndex: () -> Int,
    private val setActiveProjectIndex: (Int) -> Unit,
    private val normalizeProject: (AppProject) -> Unit,
    private val elonSelfIconDataUrl: () -> String? = { null }
) {
    val conversations: MutableList<AppConversation>
        get() = activeProject().conversations

    val projectEvents: MutableList<String>
        get() = activeProject().events

    var currentProjectTitle: String
        get() = activeProject().title
        set(value) {
            val project = activeProject()
            project.title = value
            project.updatedAt = System.currentTimeMillis()
        }

    var currentStage: String
        get() = activeProject().stage
        set(value) {
            val project = activeProject()
            project.stage = value
            project.updatedAt = System.currentTimeMillis()
        }

    var activeConversationIndex: Int
        get() = activeProject().activeConversationIndex
        set(value) {
            activeProject().activeConversationIndex = value
        }

    fun activeProject(): AppProject {
        if (projects.isEmpty()) {
            projects.add(newAppProject("一龙开发助手", "默认项目 · 点击进入会话"))
        }
        val index = activeProjectIndex().coerceIn(0, projects.lastIndex)
        setActiveProjectIndex(index)
        val project = projects[index]
        if (project.conversations.isEmpty()) project.conversations.add(defaultAppConversation())
        project.activeConversationIndex = project.activeConversationIndex.coerceIn(0, project.conversations.lastIndex)
        return project
    }

    fun activeConversation(): AppConversation {
        val project = activeProject()
        if (project.conversations.isEmpty()) {
            project.conversations.add(defaultAppConversation())
        }
        project.activeConversationIndex = project.activeConversationIndex.coerceIn(0, project.conversations.lastIndex)
        return project.conversations[project.activeConversationIndex]
    }

    fun loadProjects() {
        val loaded = loadStoredProjects(
            prefs = prefs,
            gson = gson,
            normalizeProject = normalizeProject,
            elonSelfIconDataUrl = elonSelfIconDataUrl()
        )
        projects.clear()
        projects.addAll(loaded.projects)
        setActiveProjectIndex(loaded.activeProjectIndex)
        activeProject()
        saveProjects()
    }

    fun saveConversations() {
        saveProjects()
    }

    fun saveProjects() {
        val project = activeProject()
        saveStoredProjects(prefs, gson, projects, activeProjectIndex(), project.id)
    }
}
