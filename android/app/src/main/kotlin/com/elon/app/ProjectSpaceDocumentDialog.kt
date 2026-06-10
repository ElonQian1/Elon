package com.elon.app

import android.graphics.Color
import android.text.method.LinkMovementMethod
import android.view.Gravity
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import io.noties.markwon.Markwon
import io.noties.markwon.ext.strikethrough.StrikethroughPlugin
import io.noties.markwon.ext.tables.TablePlugin
import okhttp3.OkHttpClient
import kotlin.concurrent.thread

internal object ProjectSpaceDocumentDialog {
    fun show(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        route: ProjectSpaceRoute,
        projectTitle: String,
        dp: (Int) -> Int
    ) {
        val title = projectTitle.ifBlank { "项目" }
        val status = TextView(activity).apply {
            text = "正在读取仓库文档..."
            textSize = 14f
            gravity = Gravity.CENTER
            setTextColor(Color.parseColor("#A6AFBD"))
            setPadding(dp(20), dp(42), dp(20), dp(42))
        }
        val documentText = TextView(activity).apply {
            textSize = 14f
            setTextColor(Color.parseColor("#F2F5FA"))
            setLineSpacing(dp(4).toFloat(), 1f)
            setTextIsSelectable(true)
            movementMethod = LinkMovementMethod.getInstance()
            setPadding(dp(18), dp(12), dp(18), dp(22))
        }
        val pathText = TextView(activity).apply {
            textSize = 12f
            setTextColor(Color.parseColor("#81B3D9"))
            setPadding(dp(18), dp(14), dp(18), dp(2))
        }
        val column = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(0, dp(6), 0, 0)
            addView(status)
        }
        val scroll = ScrollView(activity).apply {
            isFillViewport = false
            addView(column)
        }
        val dialog = AlertDialog.Builder(activity)
            .setTitle("$title · 项目文档")
            .setView(scroll)
            .setPositiveButton("关闭", null)
            .create()

        dialog.setOnShowListener {
            loadDocument(
                activity = activity,
                http = http,
                serverUrl = serverUrl,
                projectId = projectId,
                route = route,
                dialog = dialog,
                column = column,
                status = status,
                pathText = pathText,
                documentText = documentText,
                dp = dp
            )
        }
        dialog.show()
    }

    private fun loadDocument(
        activity: AppCompatActivity,
        http: OkHttpClient,
        serverUrl: String,
        projectId: String,
        route: ProjectSpaceRoute,
        dialog: AlertDialog,
        column: LinearLayout,
        status: TextView,
        pathText: TextView,
        documentText: TextView,
        dp: (Int) -> Int
    ) {
        thread(name = "project-document-load") {
            val result = runCatching {
                fetchProjectSpaceDocument(http, serverUrl, activity, projectId, route)
            }
            activity.runOnUiThread {
                if (!dialog.isShowing) return@runOnUiThread
                result.onSuccess { document ->
                    column.removeView(status)
                    pathText.text = document.relativePath.ifBlank { document.title }
                    if (document.truncated) {
                        pathText.append(" · 已截断")
                    }
                    documentText.background = panelBackground("#181B20").apply {
                        cornerRadius = dp(8).toFloat()
                    }
                    column.addView(pathText)
                    column.addView(documentText, LinearLayout.LayoutParams(
                        LinearLayout.LayoutParams.MATCH_PARENT,
                        LinearLayout.LayoutParams.WRAP_CONTENT
                    ).apply {
                        setMargins(dp(14), dp(8), dp(14), dp(14))
                    })
                    val markdown = document.content.ifBlank { "（文档为空）" }
                    markwon(activity).setMarkdown(documentText, markdown)
                }.onFailure { error ->
                    status.text = error.message ?: "读取项目文档失败"
                    status.setTextColor(Color.parseColor("#FF7A7A"))
                }
            }
        }
    }

    private fun markwon(activity: AppCompatActivity): Markwon {
        return Markwon.builder(activity)
            .usePlugin(StrikethroughPlugin.create())
            .usePlugin(TablePlugin.create(activity))
            .build()
    }
}
