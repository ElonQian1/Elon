package com.elon.app

import android.content.Context
import org.json.JSONArray
import org.json.JSONObject

internal const val PROJECT_PLAZA_FRESH_MS = 60_000L
internal const val PROJECT_PLAZA_SKELETON_DELAY_MS = 180L

internal data class ProjectPlazaSnapshot(
    val projects: List<StoreProject>,
    val joinedIds: Set<String>,
    val savedAtMillis: Long
)

internal fun isProjectPlazaSnapshotFresh(
    snapshot: ProjectPlazaSnapshot,
    nowMillis: Long
): Boolean = nowMillis - snapshot.savedAtMillis in 0..PROJECT_PLAZA_FRESH_MS

internal fun shouldShowProjectPlazaSkeleton(
    hasVisibleContent: Boolean,
    requestStartedAtMillis: Long,
    nowMillis: Long
): Boolean = !hasVisibleContent && nowMillis - requestStartedAtMillis >= PROJECT_PLAZA_SKELETON_DELAY_MS

internal class ProjectPlazaCache(context: Context) {
    private val preferences = AuthManager.userDataPrefs(context)

    fun read(): ProjectPlazaSnapshot? = runCatching {
        val payload = preferences.getString(KEY_PAYLOAD, null)?.takeIf(String::isNotBlank)
            ?: return null
        val root = JSONObject(payload)
        val savedAt = root.optLong("saved_at_ms", 0L).takeIf { it > 0L } ?: return null
        val projectsJson = root.optJSONArray("projects") ?: return null
        val projects = buildList {
            for (index in 0 until projectsJson.length()) {
                runCatching { parseStoreProject(projectsJson.getJSONObject(index)) }
                    .getOrNull()
                    ?.let(::add)
            }
        }
        if (projects.isEmpty()) return null
        val joined = root.optJSONArray("joined_ids").toStringSet()
        ProjectPlazaSnapshot(projects, joined, savedAt)
    }.getOrNull()

    fun write(snapshot: ProjectPlazaSnapshot) {
        val payload = JSONObject().apply {
            put("saved_at_ms", snapshot.savedAtMillis)
            put("projects", JSONArray().apply {
                snapshot.projects.forEach { put(it.toCacheJson()) }
            })
            put("joined_ids", JSONArray(snapshot.joinedIds.toList()))
        }
        preferences.edit().putString(KEY_PAYLOAD, payload.toString()).apply()
    }

    private fun StoreProject.toCacheJson(): JSONObject = JSONObject().apply {
        put("id", id)
        put("name", name)
        displayName?.let { put("display_name", it) }
        description?.let { put("description", it) }
        put("template", template)
        put("owner_account", ownerAccount)
        put("owner_id", ownerUserId)
        put("member_count", memberCount)
        put("is_public", isPublic)
        put("join_mode", joinMode)
        viewerRole?.let { put("viewer_role", it) }
        lastTaskStatus?.let { put("last_task_status", it) }
        latestApkUrl?.let { put("latest_apk_url", it) }
        installCount?.let { put("install_count", it) }
        commentCount?.let { put("comment_count", it) }
        apkSizeBytes?.let { put("latest_apk_size_bytes", it) }
        apkSizeLabel?.let { put("apk_size_label", it) }
        iconDataUrl?.let { put("icon_data_url", it) }
        projectOriginType?.let { put("project_origin_type", it) }
        projectOriginLabel?.let { put("project_origin_label", it) }
        remoteConversationCount?.let { put("conversation_count", it) }
        workspaceKind?.let { put("workspace_kind", it) }
        workspaceHealthLabel?.let { put("workspace_health_label", it) }
        workspaceHealthTone?.let { put("workspace_health_tone", it) }
        archiveEntryKey?.let { put("archive_entry_key", it) }
        archiveConversationTitle?.let { put("archive_conversation_title", it) }
        memoryScopeType?.let { put("memory_scope_type", it) }
        memoryScopeId?.let { put("memory_scope_id", it) }
    }

    private fun JSONArray?.toStringSet(): Set<String> {
        if (this == null) return emptySet()
        return buildSet {
            for (index in 0 until length()) {
                optString(index).trim().takeIf(String::isNotBlank)?.let(::add)
            }
        }
    }

    private companion object {
        const val KEY_PAYLOAD = "project_plaza_snapshot_v1"
    }
}
