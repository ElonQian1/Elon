package com.elon.app

import android.graphics.Color
import android.text.Editable
import android.text.TextWatcher
import android.text.method.LinkMovementMethod
import android.view.Gravity
import android.widget.EditText
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
            .setNeutralButton("清缓存", null)
            .setNegativeButton("刷新", null)
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
                dp = dp
            )
            dialog.getButton(AlertDialog.BUTTON_NEGATIVE).setOnClickListener {
                loadDocument(
                    activity = activity,
                    http = http,
                    serverUrl = serverUrl,
                    projectId = projectId,
                    route = route,
                    dialog = dialog,
                    column = column,
                    status = status,
                    dp = dp,
                    forceRefresh = true
                )
            }
            dialog.getButton(AlertDialog.BUTTON_NEUTRAL).setOnClickListener {
                clearCachedProjectSpaceDocuments(activity, serverUrl, projectId, route)
                loadDocument(
                    activity = activity,
                    http = http,
                    serverUrl = serverUrl,
                    projectId = projectId,
                    route = route,
                    dialog = dialog,
                    column = column,
                    status = status,
                    dp = dp,
                    forceRefresh = true
                )
            }
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
        dp: (Int) -> Int,
        forceRefresh: Boolean = false
    ) {
        column.removeAllViews()
        status.text = if (forceRefresh) "正在刷新仓库文档..." else "正在读取仓库文档..."
        status.setTextColor(Color.parseColor("#A6AFBD"))
        column.addView(status)
        thread(name = "project-document-load") {
            val result = runCatching {
                fetchProjectSpaceDocuments(
                    http = http,
                    serverUrl = serverUrl,
                    context = activity,
                    projectId = projectId,
                    route = route,
                    forceRefresh = forceRefresh
                )
            }
            activity.runOnUiThread {
                if (!dialog.isShowing) return@runOnUiThread
                result.onSuccess { bundle ->
                    column.removeAllViews()
                    renderMetadata(activity, column, bundle, dp)
                    renderWarnings(activity, column, bundle.warnings, dp)
                    val markwon = markwon(activity)
                    renderSearchableDocuments(activity, column, bundle.documents, markwon, dp)
                }.onFailure { error ->
                    status.text = error.message ?: "读取项目文档失败"
                    status.setTextColor(Color.parseColor("#FF7A7A"))
                }
            }
        }
    }

    private fun renderMetadata(
        activity: AppCompatActivity,
        column: LinearLayout,
        bundle: ProjectSpaceDocumentBundle,
        dp: (Int) -> Int
    ) {
        val parts = mutableListOf<String>()
        parts.add("来源：${sourceLabel(bundle.source.ifBlank { "server" })}")
        parts.add("文档：${bundle.documents.size}")
        bundle.revision.takeIf { it.isNotBlank() }?.let { parts.add("Revision：${it.take(12)}") }
        bundle.generatedAtMs.takeIf { it > 0L }?.let { parts.add("生成：${formatTime(it)}") }
        parts.add(if (bundle.fromCache) "缓存命中" else "在线快照")
        bundle.cachedAtMs.takeIf { it > 0L }?.let {
            parts.add("缓存：${formatTime(it)}")
            parts.add("缓存龄：${formatAge(it)}")
        }
        val metadata = TextView(activity).apply {
            textSize = 12f
            setTextColor(Color.parseColor("#A6AFBD"))
            setPadding(dp(18), dp(10), dp(18), dp(2))
            text = parts.joinToString(" · ")
        }
        column.addView(metadata)
    }

    private fun renderSearchableDocuments(
        activity: AppCompatActivity,
        column: LinearLayout,
        documents: List<ProjectSpaceDocument>,
        markwon: Markwon,
        dp: (Int) -> Int
    ) {
        val search = EditText(activity).apply {
            hint = "搜索文档、路径或内容"
            textSize = 14f
            setSingleLine(true)
            setTextColor(Color.parseColor("#F2F5FA"))
            setHintTextColor(Color.parseColor("#6F7785"))
            setPadding(dp(14), 0, dp(14), 0)
            background = panelBackground("#0F1217").apply {
                cornerRadius = dp(8).toFloat()
                setStroke(dp(1), Color.parseColor("#1E2126"))
            }
        }
        val docsContainer = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
        }
        column.addView(search, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            dp(44)
        ).apply {
            setMargins(dp(14), dp(10), dp(14), dp(6))
        })
        column.addView(docsContainer)

        fun renderFiltered(query: String) {
            docsContainer.removeAllViews()
            val trimmed = query.trim()
            val filtered = if (trimmed.isBlank()) {
                documents
            } else {
                documents.filter { document ->
                    document.title.contains(trimmed, ignoreCase = true) ||
                        document.relativePath.contains(trimmed, ignoreCase = true) ||
                        document.content.contains(trimmed, ignoreCase = true)
                }
            }
            if (filtered.isEmpty()) {
                docsContainer.addView(TextView(activity).apply {
                    text = "没有匹配的项目文档"
                    textSize = 13f
                    setTextColor(Color.parseColor("#6F7785"))
                    gravity = Gravity.CENTER
                    setPadding(dp(18), dp(28), dp(18), dp(28))
                })
            } else {
                filtered.forEach { document ->
                    renderDocument(activity, docsContainer, document, markwon, dp)
                }
            }
        }

        search.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, start: Int, count: Int, after: Int) = Unit
            override fun onTextChanged(s: CharSequence?, start: Int, before: Int, count: Int) = Unit
            override fun afterTextChanged(s: Editable?) {
                renderFiltered(s?.toString().orEmpty())
            }
        })
        renderFiltered("")
    }

    private fun renderWarnings(
        activity: AppCompatActivity,
        column: LinearLayout,
        warnings: List<String>,
        dp: (Int) -> Int
    ) {
        val text = warnings.filter { it.isNotBlank() }.joinToString("\n")
        if (text.isBlank()) return
        val warningText = TextView(activity).apply {
            textSize = 12f
            setTextColor(Color.parseColor("#C8A86B"))
            setPadding(dp(18), dp(10), dp(18), dp(2))
            this.text = text
        }
        column.addView(warningText)
    }

    private fun renderDocument(
        activity: AppCompatActivity,
        column: LinearLayout,
        document: ProjectSpaceDocument,
        markwon: Markwon,
        dp: (Int) -> Int
    ) {
        val pathText = TextView(activity).apply {
            textSize = 12f
            setTextColor(Color.parseColor("#81B3D9"))
            setPadding(dp(18), dp(14), dp(18), dp(2))
            text = document.relativePath.ifBlank { document.title }
            document.source.takeIf { it.isNotBlank() }?.let {
                append(" · ${sourceLabel(it)}")
            }
            if (document.truncated) append(" · 已截断")
        }
        val documentText = TextView(activity).apply {
            textSize = 14f
            setTextColor(Color.parseColor("#F2F5FA"))
            setLineSpacing(dp(4).toFloat(), 1f)
            setTextIsSelectable(true)
            movementMethod = LinkMovementMethod.getInstance()
            setPadding(dp(18), dp(12), dp(18), dp(22))
            background = panelBackground("#181B20").apply {
                cornerRadius = dp(8).toFloat()
            }
        }
        column.addView(pathText)
        column.addView(documentText, LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            setMargins(dp(14), dp(8), dp(14), dp(14))
        })
        markwon.setMarkdown(documentText, document.content.ifBlank { "（文档为空）" })
    }

    private fun markwon(activity: AppCompatActivity): Markwon {
        return Markwon.builder(activity)
            .usePlugin(StrikethroughPlugin.create())
            .usePlugin(TablePlugin.create(activity))
            .build()
    }

    private fun sourceLabel(source: String): String {
        return when {
            source.startsWith("pc_node") -> "PC 节点"
            source.startsWith("server_fallback") -> "服务器回退"
            source == "workspace" -> "仓库"
            source == "workspace_with_defaults" -> "仓库 + 默认"
            source == "platform_default" -> "平台默认"
            source == "apk_default" -> "APK 内置"
            source == "read_error" -> "读取异常"
            else -> source
        }
    }

    private fun formatTime(epochMillis: Long): String {
        return runCatching {
            java.time.Instant.ofEpochMilli(epochMillis).toString()
        }.getOrDefault(epochMillis.toString())
    }

    private fun formatAge(epochMillis: Long): String {
        val minutes = ((System.currentTimeMillis() - epochMillis).coerceAtLeast(0L) / 60000L)
        return when {
            minutes < 1L -> "刚刚"
            minutes < 60L -> "${minutes} 分钟"
            minutes < 24L * 60L -> "${minutes / 60L} 小时"
            else -> "${minutes / (24L * 60L)} 天"
        }
    }
}
