package com.elon.app

import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal class ProjectPlazaInstallActionController(
    private val activity: AppCompatActivity,
    private val http: OkHttpClient,
    private val serverUrl: String,
    private val onCreated: (StoreProject) -> Unit,
    private val onStateChanged: () -> Unit
) {
    private val busyProjectIds = mutableSetOf<String>()

    fun presentation(project: StoreProject): ProjectPlazaPrimaryAction? =
        projectPlazaInstallAction(project, busyProjectIds.contains(project.id))

    fun handle(project: StoreProject): Boolean {
        val action = presentation(project) ?: return false
        if (!action.enabled) return true
        if (!AuthManager.isLoggedIn(activity)) {
            Toast.makeText(activity, "请先登录后创建店铺", Toast.LENGTH_SHORT).show()
            return true
        }
        val token = AuthManager.token(activity)?.trim().orEmpty()
        if (token.isBlank()) {
            Toast.makeText(activity, "登录已过期，请重新登录", Toast.LENGTH_SHORT).show()
            return true
        }
        ProjectPlazaInstallDialog.show(activity, project) { projectName, industry ->
            create(project, projectName, industry, token)
        }
        return true
    }

    private fun create(
        sourceProject: StoreProject,
        projectName: String,
        industry: String,
        token: String
    ) {
        if (!busyProjectIds.add(sourceProject.id)) return
        onStateChanged()
        thread(name = "project-plaza-install") {
            val result = runCatching {
                createMarketplaceErpInstance(
                    http = http,
                    serverUrl = serverUrl,
                    sourceProjectId = sourceProject.id,
                    projectName = projectName,
                    industry = industry,
                    token = token
                )
            }
            activity.runOnUiThread {
                busyProjectIds.remove(sourceProject.id)
                result
                    .onSuccess { created ->
                        Toast.makeText(activity, "店铺已创建", Toast.LENGTH_SHORT).show()
                        onCreated(created.toStoreProject(sourceProject))
                    }
                    .onFailure { error ->
                        val message = error.message?.trim().orEmpty().ifBlank { "店铺创建失败，请重试" }
                        Toast.makeText(activity, message.take(160), Toast.LENGTH_LONG).show()
                    }
                onStateChanged()
            }
        }
    }

    private fun MarketplaceErpInstanceResult.toStoreProject(sourceProject: StoreProject) = StoreProject(
        id = projectId,
        name = projectName,
        displayName = projectName,
        description = "基于${sourceProject.displayTitle()}创建的独立店铺项目",
        template = "local",
        ownerAccount = "我",
        memberCount = 1,
        isPublic = false,
        joinMode = PROJECT_JOIN_MODE_INVITE,
        viewerRole = "owner",
        lastTaskStatus = null,
        projectOriginType = "self",
        projectOriginLabel = "我创建"
    )
}
