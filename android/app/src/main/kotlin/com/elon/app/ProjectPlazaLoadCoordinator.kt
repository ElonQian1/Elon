package com.elon.app

import android.app.Activity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal data class ProjectPlazaLoadRequest(
    val key: String,
    val search: String?,
    val joinMode: String?,
    val hasApk: Boolean?,
    val sort: String?,
    val isDefault: Boolean
)

internal class ProjectPlazaLoadCoordinator(
    private val activity: Activity,
    private val http: OkHttpClient,
    private val serverUrl: String
) {
    private val cache by lazy { ProjectPlazaCache(activity.applicationContext) }
    private val snapshots = LinkedHashMap<String, ProjectPlazaSnapshot>()
    private val loadingKeys = mutableSetOf<String>()
    private var restored = false
    private var delayedLoading: Runnable? = null
    private var serial = 0

    fun load(
        request: ProjectPlazaLoadRequest,
        force: Boolean,
        hasVisibleContent: Boolean,
        onCached: (snapshot: ProjectPlazaSnapshot, exact: Boolean) -> Boolean,
        onLoading: () -> Unit,
        onSuccess: (projects: List<StoreProject>) -> Unit,
        onStaleFailure: (message: String) -> Unit,
        onEmptyFailure: (message: String) -> Unit,
        onJoined: (joinedIds: Set<String>) -> Unit
    ) {
        restore()
        val exact = snapshots[request.key]
        val fallback = exact ?: snapshots[DEFAULT_KEY]
        var contentVisible = hasVisibleContent
        if (fallback != null) contentVisible = onCached(fallback, exact != null)
        if (!force && exact != null && isProjectPlazaSnapshotFresh(exact, System.currentTimeMillis())) {
            return
        }
        if (!loadingKeys.add(request.key)) return

        val requestSerial = ++serial
        val startedAt = System.currentTimeMillis()
        val joinedAtRequest = fallback?.joinedIds.orEmpty()
        scheduleLoading(requestSerial, request.key, startedAt, contentVisible, onLoading)
        thread(name = "project-plaza-list") {
            val result = runCatching {
                fetchAllStoreProjects(
                    http = http,
                    serverUrl = serverUrl,
                    search = request.search,
                    joinMode = request.joinMode,
                    hasApk = request.hasApk,
                    sort = request.sort,
                    ctx = activity
                )
            }
            val snapshot = result.getOrNull()?.let { projects ->
                ProjectPlazaSnapshot(projects, joinedAtRequest, System.currentTimeMillis())
            }
            if (request.isDefault && snapshot != null) cache.write(snapshot)
            activity.runOnUiThread {
                loadingKeys.remove(request.key)
                if (requestSerial == serial) cancelDelayedLoading()
                snapshot?.let { snapshots[request.key] = it }
                if (requestSerial != serial) return@runOnUiThread
                result
                    .onSuccess(onSuccess)
                    .onFailure { error ->
                        val message = error.message ?: "加载失败"
                        if (contentVisible) onStaleFailure(message) else onEmptyFailure(message)
                    }
            }
            if (result.isSuccess && AuthManager.isLoggedIn(activity)) {
                refreshJoined(requestSerial, onJoined)
            }
        }
    }

    private fun restore() {
        if (restored) return
        restored = true
        cache.read()?.let { snapshots[DEFAULT_KEY] = it }
    }

    private fun scheduleLoading(
        requestSerial: Int,
        key: String,
        startedAt: Long,
        contentVisible: Boolean,
        onLoading: () -> Unit
    ) {
        cancelDelayedLoading()
        delayedLoading = Runnable {
            if (
                requestSerial == serial &&
                loadingKeys.contains(key) &&
                shouldShowProjectPlazaSkeleton(contentVisible, startedAt, System.currentTimeMillis())
            ) {
                onLoading()
            }
        }.also { activity.window.decorView.postDelayed(it, PROJECT_PLAZA_SKELETON_DELAY_MS) }
    }

    private fun cancelDelayedLoading() {
        delayedLoading?.let { activity.window.decorView.removeCallbacks(it) }
        delayedLoading = null
    }

    private fun refreshJoined(requestSerial: Int, onJoined: (Set<String>) -> Unit) {
        val refreshed = runCatching { fetchJoinedProjectIds(http, serverUrl, activity) }.getOrNull() ?: return
        activity.runOnUiThread {
            snapshots[DEFAULT_KEY]?.let { base ->
                val updated = base.copy(joinedIds = refreshed, savedAtMillis = System.currentTimeMillis())
                snapshots[DEFAULT_KEY] = updated
                thread(name = "project-plaza-cache") { cache.write(updated) }
            }
            if (requestSerial == serial) onJoined(refreshed)
        }
    }

    private companion object {
        const val DEFAULT_KEY = "all|"
    }
}
