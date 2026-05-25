package com.elon.app

import android.content.SharedPreferences
import com.google.gson.Gson
import java.util.UUID

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
