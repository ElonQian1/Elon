package com.elon.app

import android.content.SharedPreferences
import org.json.JSONObject

private const val PREF_PROJECT_TASK_COMPLETION_BADGES = "project_task_completion_badges_v1"
private const val PROJECT_TASK_BADGE_MAX = 99

internal fun markProjectTaskCompletionBadge(
    prefs: SharedPreferences,
    projectId: String?
): Boolean {
    val id = cleanProjectTaskBadgeId(projectId) ?: return false
    val badges = loadProjectTaskCompletionBadges(prefs)
    val next = (badges.optInt(id, 0).coerceAtLeast(0) + 1).coerceAtMost(PROJECT_TASK_BADGE_MAX)
    badges.put(id, next)
    prefs.edit().putString(PREF_PROJECT_TASK_COMPLETION_BADGES, badges.toString()).apply()
    return true
}

internal fun projectTaskCompletionBadgeCount(
    prefs: SharedPreferences,
    projectIds: Iterable<String?>
): Int {
    val badges = loadProjectTaskCompletionBadges(prefs)
    return projectIds
        .mapNotNull(::cleanProjectTaskBadgeId)
        .distinct()
        .sumOf { badges.optInt(it, 0).coerceAtLeast(0) }
        .coerceAtMost(PROJECT_TASK_BADGE_MAX)
}

internal fun clearProjectTaskCompletionBadges(
    prefs: SharedPreferences,
    projectIds: Iterable<String?>
): Boolean {
    val ids = projectIds.mapNotNull(::cleanProjectTaskBadgeId).distinct()
    if (ids.isEmpty()) return false
    val badges = loadProjectTaskCompletionBadges(prefs)
    var changed = false
    ids.forEach { id ->
        if (badges.has(id)) {
            badges.remove(id)
            changed = true
        }
    }
    if (!changed) return false
    prefs.edit().putString(PREF_PROJECT_TASK_COMPLETION_BADGES, badges.toString()).apply()
    return true
}

internal fun AppProject.projectTaskBadgeIds(): List<String?> {
    return listOf(
        id,
        projectSpaceId(),
        collaborationProjectId
    )
}

private fun loadProjectTaskCompletionBadges(prefs: SharedPreferences): JSONObject {
    val raw = prefs.getString(PREF_PROJECT_TASK_COMPLETION_BADGES, null)
    return runCatching { JSONObject(raw ?: "{}") }.getOrDefault(JSONObject())
}

private fun cleanProjectTaskBadgeId(value: String?): String? {
    return value
        ?.trim()
        ?.takeIf { it.isNotBlank() && !it.equals("null", ignoreCase = true) }
}
