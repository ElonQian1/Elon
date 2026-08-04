package com.elon.app

import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.widget.ArrayAdapter
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.ScrollView
import android.widget.Spinner
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal object ProjectWorkspaceRecoveryDialog {
    fun show(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        project: AppProject,
        onProjectUpdated: (ArchiveProjectRecord?) -> Unit
    ) {
        val projectId = project.projectSpaceId()
        val content = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(dp(activity, 20), dp(activity, 12), dp(activity, 20), 0)
            addView(ProgressBar(activity).apply { isIndeterminate = true }, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { gravity = Gravity.CENTER_HORIZONTAL })
            addView(TextView(activity).apply {
                text = "正在检查 PC 工作区..."
                textSize = 14f
                setTextColor(Color.parseColor("#A8A8A8"))
                gravity = Gravity.CENTER
                setPadding(0, dp(activity, 12), 0, 0)
            })
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle("PC 工作区")
            .setView(ScrollView(activity).apply { addView(content) })
            .setNegativeButton("关闭", null)
            .create()
        dialog.setOnShowListener {
            loadHealth(activity, http, serverUrl, projectId, content, onProjectUpdated)
        }
        dialog.show()
    }

    private fun loadHealth(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        content: LinearLayout,
        onProjectUpdated: (ArchiveProjectRecord?) -> Unit
    ) {
        thread(name = "project-workspace-health") {
            val result = runCatching {
                fetchProjectWorkspaceHealth(http, serverUrl, activity, projectId)
            }
            activity.runOnUiThread {
                result
                    .onSuccess { health ->
                        renderHealth(activity, http, serverUrl, projectId, content, health, onProjectUpdated)
                    }
                    .onFailure { error ->
                        content.removeAllViews()
                        content.addView(messageView(activity, "检查失败：${error.message ?: "未知错误"}", "#FFD8D8"))
                    }
            }
        }
    }

    private fun renderHealth(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        content: LinearLayout,
        health: ProjectWorkspaceHealth,
        onProjectUpdated: (ArchiveProjectRecord?) -> Unit
    ) {
        content.removeAllViews()
        content.addView(statusHeader(activity, health))
        if (health.recommendedAction.isNotBlank()) {
            content.addView(messageView(activity, health.recommendedAction, "#A8A8A8"))
        }
        listOf(
            "节点" to health.nodeDisplay,
            "工作区" to health.workspacePath,
            "Git" to health.gitStatus,
            "CLI" to health.cliStatus,
            "磁盘剩余" to health.diskFreeText,
            "最近执行" to health.latestExecution
        ).forEach { (label, value) ->
            content.addView(infoRow(activity, label, value))
        }
        if (health.warnings.isNotEmpty()) {
            content.addView(messageView(activity, health.warnings.joinToString("\n") { "• $it" }, "#F7D28A"))
        }
        val runnable = health.recoveryActions.filter { it.available && it.key != "repair_cli" }
        if (runnable.isNotEmpty()) {
            content.addView(actionList(activity, http, serverUrl, projectId, content, runnable, onProjectUpdated))
        }
    }

    private fun actionList(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        content: LinearLayout,
        actions: List<ProjectWorkspaceRecoveryAction>,
        onProjectUpdated: (ArchiveProjectRecord?) -> Unit
    ): LinearLayout {
        return LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(activity, 12), 0, 0)
            actions.forEach { action ->
                addView(Button(activity).apply {
                    text = action.label
                    setTextColor(Color.parseColor("#101010"))
                    backgroundTintList = android.content.res.ColorStateList.valueOf(Color.parseColor("#C8C8C8"))
                    setOnClickListener {
                        if (action.key == "migrate_workspace" || action.key == "bind_pc_node") {
                            chooseNodeAndRecover(activity, http, serverUrl, projectId, content, action, onProjectUpdated)
                        } else {
                            recover(activity, http, serverUrl, projectId, content, action.key, null, onProjectUpdated)
                        }
                    }
                }, LinearLayout.LayoutParams(
                    LinearLayout.LayoutParams.MATCH_PARENT,
                    LinearLayout.LayoutParams.WRAP_CONTENT
                ).apply { topMargin = dp(activity, 8) })
            }
        }
    }

    private fun chooseNodeAndRecover(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        content: LinearLayout,
        action: ProjectWorkspaceRecoveryAction,
        onProjectUpdated: (ArchiveProjectRecord?) -> Unit
    ) {
        thread(name = "project-workspace-node-options") {
            val result = runCatching {
                fetchProjectCreateNodes(http, serverUrl, activity)
                    .filter { it.online && it.cliProjectReady }
                    .sortedBy { it.displayName }
            }
            activity.runOnUiThread {
                result
                    .onSuccess { nodes ->
                        if (nodes.isEmpty()) {
                            Toast.makeText(activity, "没有可用的在线 PC 节点", Toast.LENGTH_SHORT).show()
                            return@onSuccess
                        }
                        val spinner = Spinner(activity)
                        spinner.adapter = ArrayAdapter(
                            activity,
                            android.R.layout.simple_spinner_dropdown_item,
                            nodes.map { "${it.displayName} · ${it.shortId} · ${it.projectCount}个项目" }
                        )
                        AlertDialog.Builder(activity)
                            .setTitle(action.label)
                            .setView(spinner)
                            .setNegativeButton("取消", null)
                            .setPositiveButton("继续") { _, _ ->
                                val selected = nodes.getOrNull(spinner.selectedItemPosition)
                                recover(activity, http, serverUrl, projectId, content, action.key, selected?.nodeId, onProjectUpdated)
                            }
                            .show()
                    }
                    .onFailure { error ->
                        Toast.makeText(activity, "加载 PC 节点失败：${error.message}", Toast.LENGTH_LONG).show()
                    }
            }
        }
    }

    private fun recover(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        content: LinearLayout,
        action: String,
        nodeId: String?,
        onProjectUpdated: (ArchiveProjectRecord?) -> Unit
    ) {
        Toast.makeText(activity, "正在处理 PC 工作区...", Toast.LENGTH_SHORT).show()
        thread(name = "project-workspace-recover") {
            val result = runCatching {
                recoverProjectWorkspace(http, serverUrl, activity, projectId, action, nodeId)
            }
            activity.runOnUiThread {
                result
                    .onSuccess { recovered ->
                        onProjectUpdated(recovered.archiveProject)
                        Toast.makeText(activity, recovered.message, Toast.LENGTH_SHORT).show()
                        loadHealth(activity, http, serverUrl, projectId, content, onProjectUpdated)
                    }
                    .onFailure { error ->
                        Toast.makeText(activity, "处理失败：${error.message ?: "未知错误"}", Toast.LENGTH_LONG).show()
                    }
            }
        }
    }

    private fun statusHeader(activity: AppCompatActivity, health: ProjectWorkspaceHealth): LinearLayout {
        val color = when (health.healthTone) {
            "ok" -> "#5AC8A0"
            "bad" -> "#C44646"
            else -> "#C99630"
        }
        val toneLabel = when (health.healthTone) {
            "ok" -> "正常"
            "bad" -> "异常"
            else -> "提醒"
        }
        return LinearLayout(activity).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(TextView(activity).apply {
                text = health.healthLabel
                textSize = 17f
                typeface = Typeface.DEFAULT_BOLD
                setTextColor(Color.parseColor("#D6D6D6"))
            }, LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(TextView(activity).apply {
                text = toneLabel
                textSize = 12f
                setTextColor(Color.parseColor("#D6D6D6"))
                background = GradientDrawable().apply {
                    cornerRadius = dp(activity, 10).toFloat()
                    setColor(Color.parseColor(color))
                }
                setPadding(dp(activity, 8), dp(activity, 4), dp(activity, 8), dp(activity, 4))
            })
        }
    }

    private fun infoRow(activity: AppCompatActivity, label: String, value: String): TextView {
        return TextView(activity).apply {
            text = "$label：$value"
            textSize = 13f
            setTextColor(Color.parseColor("#D6D6D6"))
            setPadding(0, dp(activity, 8), 0, 0)
        }
    }

    private fun messageView(activity: AppCompatActivity, textValue: String, color: String): TextView {
        return TextView(activity).apply {
            text = textValue
            textSize = 13f
            setTextColor(Color.parseColor(color))
            setPadding(0, dp(activity, 10), 0, 0)
        }
    }

    private fun dp(activity: AppCompatActivity, value: Int): Int {
        return (value * activity.resources.displayMetrics.density).toInt()
    }
}
