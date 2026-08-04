package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class ProjectPlazaMembershipActionController(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val isJoined: (StoreProject) -> Boolean,
    private val onJoined: (StoreProject) -> Unit,
    private val onStateChanged: () -> Unit
) {
    private val busyProjectIds = mutableSetOf<String>()
    private val pendingRequestIds = mutableSetOf<String>()

    fun presentation(project: StoreProject): ProjectPlazaPrimaryAction = projectPlazaPrimaryAction(
        project = project,
        joined = isJoined(project),
        requestPending = pendingRequestIds.contains(project.id),
        busy = busyProjectIds.contains(project.id)
    )

    fun handle(project: StoreProject, openProjectSpace: (StoreProject) -> Unit) {
        val action = presentation(project)
        if (!action.enabled) return
        when (action.kind) {
            ProjectPlazaPrimaryActionKind.OPEN -> openProjectSpace(project)
            ProjectPlazaPrimaryActionKind.JOIN,
            ProjectPlazaPrimaryActionKind.REQUEST_JOIN -> performMembershipAction(project, action.kind)
        }
    }

    private fun performMembershipAction(project: StoreProject, kind: ProjectPlazaPrimaryActionKind) {
        if (!AuthManager.isLoggedIn(activity)) {
            Toast.makeText(activity, "请先登录后加入项目", Toast.LENGTH_SHORT).show()
            return
        }
        val token = AuthManager.token(activity)?.trim().orEmpty()
        if (token.isBlank()) {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return
        }
        if (!busyProjectIds.add(project.id)) return
        onStateChanged()
        thread(name = "project-plaza-membership") {
            val result = runCatching {
                if (kind == ProjectPlazaPrimaryActionKind.REQUEST_JOIN) {
                    requestJoinStoreProject(http, serverUrl, project.id, token)
                } else {
                    joinStoreProject(http, serverUrl, project.id, token)
                }
            }
            activity.runOnUiThread {
                busyProjectIds.remove(project.id)
                result
                    .onSuccess {
                        if (kind == ProjectPlazaPrimaryActionKind.REQUEST_JOIN) {
                            pendingRequestIds.add(project.id)
                            Toast.makeText(activity, "申请已提交，等待项目管理员审核", Toast.LENGTH_SHORT).show()
                        } else {
                            onJoined(project)
                            Toast.makeText(activity, "已加入项目，点击按钮进入空间", Toast.LENGTH_SHORT).show()
                        }
                    }
                    .onFailure { error ->
                        val message = error.message?.trim().orEmpty().ifBlank { "操作失败，请重试" }
                        Toast.makeText(activity, message.take(160), Toast.LENGTH_LONG).show()
                    }
                onStateChanged()
            }
        }
    }
}
