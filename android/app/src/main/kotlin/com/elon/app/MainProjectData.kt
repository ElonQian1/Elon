package com.elon.app

import android.content.SharedPreferences
import com.google.gson.Gson
import java.util.UUID

internal data class LoadedProjects(
    val projects: MutableList<AppProject>,
    val activeProjectIndex: Int
)

internal fun welcomeChatMessage(): ChatMessage {
    return ChatMessage(
        "ai",
        "你可以直接描述想开发的 App 功能；我会先说明我理解到的意图，再把需求分析、开发实现、编译打包和交付证据折叠同步给你。"
    )
}

internal fun defaultAppConversation(): AppConversation {
    return AppConversation(
        id = "default",
        title = "一龙开发助手",
        subtitle = "连接中...",
        updatedAt = System.currentTimeMillis(),
        messages = mutableListOf(welcomeChatMessage())
    )
}

internal fun newAppConversation(title: String, subtitle: String): AppConversation {
    return AppConversation(
        id = UUID.randomUUID().toString(),
        title = summarize(title, 24),
        subtitle = subtitle,
        updatedAt = System.currentTimeMillis(),
        messages = mutableListOf(welcomeChatMessage())
    )
}

internal fun newAppProject(title: String, subtitle: String): AppProject {
    return AppProject(
        id = UUID.randomUUID().toString(),
        title = summarize(title, 24),
        subtitle = subtitle,
        updatedAt = System.currentTimeMillis(),
        conversations = mutableListOf(defaultAppConversation())
    )
}

internal fun legacyAppProject(prefs: SharedPreferences, gson: Gson): AppProject {
    val saved = prefs.getString("conversations_json", null)
    val legacyConversations = runCatching {
        if (saved.isNullOrBlank()) null
        else gson.fromJson(saved, Array<AppConversation>::class.java)?.toMutableList()
    }.getOrNull().orEmpty().filter { it.title.isNotBlank() }.toMutableList()
    legacyConversations.forEach {
        if (it.messages.isEmpty()) it.messages.add(welcomeChatMessage())
    }
    if (legacyConversations.isEmpty()) legacyConversations.add(defaultAppConversation())

    val savedEvents = prefs.getString("project_events", "").orEmpty()
    val title = prefs.getString("project_title", null)?.takeIf { it.isNotBlank() } ?: "一龙开发助手"
    return AppProject(
        id = UUID.randomUUID().toString(),
        title = summarize(title, 24),
        subtitle = "默认项目 · ${legacyConversations.size} 个会话",
        updatedAt = legacyConversations.maxOfOrNull { it.updatedAt } ?: System.currentTimeMillis(),
        conversations = legacyConversations,
        events = savedEvents.lines().filter { it.isNotBlank() }.toMutableList()
    )
}

internal fun loadStoredProjects(
    prefs: SharedPreferences,
    gson: Gson,
    normalizeProject: (AppProject) -> Unit,
    elonSelfIconDataUrl: String? = null
): LoadedProjects {
    val savedProjects = prefs.getString(PROJECTS_JSON_KEY, null)
    val projects = runCatching {
        if (savedProjects.isNullOrBlank()) null
        else gson.fromJson(savedProjects, Array<AppProject>::class.java)?.toMutableList()
    }.getOrNull()
        .orEmpty()
        .filter { it.title.isNotBlank() }
        .onEach(normalizeProject)
        .toMutableList()

    if (projects.isEmpty()) {
        projects.add(legacyAppProject(prefs, gson))
    }

    ensureElonSelfProject(projects, elonSelfIconDataUrl)
    val activeIndex = prefs.getInt(ACTIVE_PROJECT_INDEX_KEY, 0).coerceIn(0, projects.lastIndex)
    return LoadedProjects(projects, activeIndex)
}

internal fun saveStoredProjects(
    prefs: SharedPreferences,
    gson: Gson,
    projects: List<AppProject>,
    activeProjectIndex: Int,
    activeProjectId: String,
    synchronous: Boolean = false
) {
    val editor = prefs.edit()
        .putString(PROJECTS_JSON_KEY, gson.toJson(projects))
        .putInt(ACTIVE_PROJECT_INDEX_KEY, activeProjectIndex)
        .putString(TaskWorkService.PREF_ACTIVE_PROJECT_ID, activeProjectId)
    if (synchronous) {
        editor.commit()
    } else {
        editor.apply()
    }
}

private fun ensureElonSelfProject(projects: MutableList<AppProject>, iconDataUrl: String?) {
    val existing = projects.firstOrNull { it.id == ELON_SELF_PROJECT_ID }
    if (existing != null) {
        // 升级旧数据：确保一龙自项目始终是联合开发项目
        if (!existing.isJointProject) existing.isJointProject = true
        normalizeElonSelfProject(existing, iconDataUrl)
        return
    }
    projects.add(0, elonSelfProject(iconDataUrl))
}

private fun normalizeElonSelfProject(project: AppProject, iconDataUrl: String?) {
    project.ownerAccount = ELON_SELF_OWNER_ACCOUNT
    project.projectOriginType = "platform_self"
    project.projectOriginLabel = "钱一龙创建"
    project.memberCount = project.memberCount?.coerceAtLeast(1) ?: 1
    if (project.iconDataUrl.cleanElonSelfIconDataUrl() == null) {
        iconDataUrl.cleanElonSelfIconDataUrl()?.let { project.iconDataUrl = it }
    }
}

private fun elonSelfProject(iconDataUrl: String?): AppProject {
    return AppProject(
        id = ELON_SELF_PROJECT_ID,
        title = "一龙项目",
        subtitle = "修改平台自身 · AI 云端迭代",
        updatedAt = 0L,
        isJointProject = true,
        iconDataUrl = iconDataUrl.cleanElonSelfIconDataUrl(),
        ownerAccount = ELON_SELF_OWNER_ACCOUNT,
        projectOriginType = "platform_self",
        projectOriginLabel = "钱一龙创建",
        memberCount = 1,
        conversations = mutableListOf(
            AppConversation(
                id = "elon-self-default",
                title = "一龙项目",
                subtitle = "连接中...",
                updatedAt = 0L,
                messages = mutableListOf(
                    ChatMessage(
                        "ai",
                        "你可以直接告诉我想给 APK 加什么功能，例如「加一个深色模式切换」——我会先确认理解，再修改源码、检查结果并把新 APK 发给你。"
                    )
                )
            )
        )
    )
}

private fun String?.cleanElonSelfIconDataUrl(): String? {
    val text = this?.trim().orEmpty()
    return text.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}

private const val PROJECTS_JSON_KEY = "projects_json"
private const val ACTIVE_PROJECT_INDEX_KEY = "active_project_index"
internal const val ELON_SELF_PROJECT_ID = "elon-self"
