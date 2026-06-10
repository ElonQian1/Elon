package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class ProjectSpaceFeedData(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val route: () -> ProjectSpaceRoute,
    private val activeProjectId: () -> String?,
    private val isSpaceLandingActive: () -> Boolean,
    private val renderLanding: () -> Unit
) {
    private val mutableMessagesByChannel = linkedMapOf<String, List<ProjectChannelMessage>>()
    private var loadedProjectId: String? = null
    private var loadingProjectId: String? = null

    val messagesByChannel: Map<String, List<ProjectChannelMessage>>
        get() = mutableMessagesByChannel

    fun reset() {
        mutableMessagesByChannel.clear()
        loadedProjectId = null
        loadingProjectId = null
    }

    fun isLoading(space: ProjectSpace): Boolean {
        return loadingProjectId == space.project.id
    }

    fun ensure(space: ProjectSpace) {
        if (loadedProjectId == space.project.id || loadingProjectId == space.project.id) return
        load(space)
    }

    fun submit(
        channel: ProjectChannel,
        title: String,
        body: String,
        onComplete: (Result<Unit>) -> Unit
    ) {
        val projectId = activeProjectId() ?: run {
            onComplete(Result.failure(IllegalStateException("项目空间未就绪")))
            return
        }
        val requestRoute = route()
        val content = formatProjectSpacePostContent(title, body)
        thread(name = "project-space-post-submit") {
            val result = runCatching {
                sendProjectChannelMessage(http, serverUrl, activity, projectId, channel.id, content, requestRoute)
            }
            activity.runOnUiThread {
                result.onSuccess { sent ->
                    val existing = mutableMessagesByChannel[channel.id].orEmpty()
                    mutableMessagesByChannel[channel.id] = listOf(sent) + existing.filter { it.id != sent.id }
                    loadedProjectId = projectId
                    onComplete(Result.success(Unit))
                    Toast.makeText(activity, "帖子已发布", Toast.LENGTH_SHORT).show()
                    renderLanding()
                }.onFailure { error ->
                    onComplete(Result.failure(error))
                }
            }
        }
    }

    private fun load(space: ProjectSpace) {
        val projectId = space.project.id
        val channels = space.channels.filter { it.kind == "announcements" || it.isProjectSpaceFeedChannel() }
        if (channels.isEmpty()) {
            loadedProjectId = projectId
            return
        }
        loadingProjectId = projectId
        val requestRoute = route()
        thread(name = "project-space-feed") {
            val result = runCatching {
                channels.associate { channel ->
                    channel.id to fetchProjectChannelMessages(
                        http = http,
                        serverUrl = serverUrl,
                        context = activity,
                        projectId = channel.projectId,
                        channelId = channel.id,
                        limit = FEED_MESSAGE_LIMIT,
                        route = requestRoute
                    )
                }
            }
            activity.runOnUiThread {
                if (activeProjectId() != projectId || !isSpaceLandingActive()) {
                    if (loadingProjectId == projectId) loadingProjectId = null
                    return@runOnUiThread
                }
                if (loadingProjectId == projectId) loadingProjectId = null
                result.onSuccess { messages ->
                    mutableMessagesByChannel.clear()
                    mutableMessagesByChannel.putAll(messages)
                    loadedProjectId = projectId
                    renderLanding()
                }.onFailure { error ->
                    loadedProjectId = projectId
                    Toast.makeText(activity, error.message ?: "加载帖子失败", Toast.LENGTH_SHORT).show()
                    renderLanding()
                }
            }
        }
    }

    private companion object {
        const val FEED_MESSAGE_LIMIT = 24
    }
}
