package com.elon.app

import android.content.Context
import okhttp3.OkHttpClient
import okhttp3.Request
import org.json.JSONArray
import org.json.JSONObject

internal data class ProjectSpaceDocument(
    val title: String,
    val relativePath: String,
    val content: String,
    val sizeBytes: Long,
    val truncated: Boolean
)

internal data class ProjectSpaceDocumentBundle(
    val documents: List<ProjectSpaceDocument>,
    val warnings: List<String>
)

internal fun fetchProjectSpaceDocuments(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): ProjectSpaceDocumentBundle {
    val result = runCatching {
        val request = AuthManager.applyAuth(
            context,
            Request.Builder()
                .url(projectSpaceUrl(serverUrl, projectId, route, "docs"))
                .get()
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (!response.isSuccessful) error(readProjectDocumentError(body, "读取项目文档失败"))
            val bundle = parseProjectSpaceDocuments(body)
            cacheProjectSpaceDocuments(context, projectId, body)
            bundle
        }
    }
    return result.getOrElse { error ->
        cachedProjectSpaceDocuments(context, projectId)?.let { cached ->
            cached.copy(warnings = listOf("服务器暂不可用，显示 APK 缓存的项目文档。") + cached.warnings)
        } ?: defaultProjectSpaceDocuments(
            listOf(error.message ?: "服务器暂不可用，显示 APK 内置默认项目文档。")
        )
    }
}

internal fun fetchProjectSpaceDocument(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
): ProjectSpaceDocument {
    return fetchProjectSpaceDocuments(http, serverUrl, context, projectId, route)
        .documents
        .first()
}

private fun parseProjectSpaceDocuments(body: String): ProjectSpaceDocumentBundle {
    val root = JSONObject(body)
    val documents = mutableListOf<ProjectSpaceDocument>()
    root.optJSONArray("documents")?.let { array ->
        for (index in 0 until array.length()) {
            array.optJSONObject(index)?.let { documents.add(parseProjectSpaceDocument(it)) }
        }
    }
    if (documents.isEmpty()) {
        root.optJSONObject("document")?.let { documents.add(parseProjectSpaceDocument(it)) }
    }
    val warnings = root.optJSONArray("warnings").toStringList()
    return if (documents.isNotEmpty()) {
        ProjectSpaceDocumentBundle(documents, warnings)
    } else {
        defaultProjectSpaceDocuments(warnings + "服务器没有返回项目文档，显示 APK 内置默认项目文档。")
    }
}

private fun parseProjectSpaceDocument(doc: JSONObject): ProjectSpaceDocument {
    return ProjectSpaceDocument(
        title = doc.optString("title", "项目文档"),
        relativePath = doc.optString("path", doc.optString("title", "项目文档")),
        content = doc.optString("content", ""),
        sizeBytes = doc.optLong("size_bytes", doc.optLong("byte_len", 0L)),
        truncated = doc.optBoolean("truncated", false)
    )
}

private fun cacheProjectSpaceDocuments(context: Context, projectId: String, body: String) {
    context.getSharedPreferences(PROJECT_DOCS_CACHE, Context.MODE_PRIVATE)
        .edit()
        .putString(projectId, body)
        .apply()
}

private fun cachedProjectSpaceDocuments(
    context: Context,
    projectId: String
): ProjectSpaceDocumentBundle? {
    val body = context.getSharedPreferences(PROJECT_DOCS_CACHE, Context.MODE_PRIVATE)
        .getString(projectId, null)
        ?: return null
    return runCatching { parseProjectSpaceDocuments(body) }.getOrNull()
}

private fun JSONArray?.toStringList(): List<String> {
    if (this == null) return emptyList()
    val values = mutableListOf<String>()
    for (index in 0 until length()) {
        optString(index).takeIf { it.isNotBlank() }?.let(values::add)
    }
    return values
}

private fun readProjectDocumentError(body: String, fallback: String): String {
    if (body.isBlank()) return fallback
    return runCatching {
        JSONObject(body).optString("error", "").ifBlank { fallback }
    }.getOrDefault(fallback)
}

private fun defaultProjectSpaceDocuments(warnings: List<String> = emptyList()): ProjectSpaceDocumentBundle {
    return ProjectSpaceDocumentBundle(
        documents = listOf(
            ProjectSpaceDocument(
                title = "项目 AI 工作入口",
                relativePath = "AGENTS.md",
                content = """
                    # 项目 AI 工作入口

                    本项目由一龙 APK 创建和维护。AI 代理开始任何开发任务前，必须先读取本文件，再按任务需要读取其它项目文档。

                    ## 基本规则

                    - 先确认当前项目目录、Git 状态、用户需求和可验证的完成标准。
                    - 修改代码前先理解现有结构，不把无关项目的规则套到本项目。
                    - 优先做小而完整的改动：实现、验证、提交，并说明结果。
                    - 不覆盖用户已有文件；发现冲突、脏工作区或缺失依赖时，先诊断再处理。
                    - 需要构建或发布时，使用项目内已有脚本和文档，不手搓发布流程。
                """.trimIndent(),
                sizeBytes = 0L,
                truncated = false
            ),
            ProjectSpaceDocument(
                title = "Codex 执行说明",
                relativePath = "CODEX.md",
                content = """
                    # Codex 执行说明

                    Codex 在本项目中负责把用户的自然语言需求落成可验证的代码改动。

                    ## 工作方式

                    - 先读项目文档和现有源码，再决定实现位置。
                    - 倾向复用项目已有框架、脚本、目录结构和命名风格。
                    - 新功能应有清楚边界，避免把大型逻辑堆进入口文件。
                    - 完成后运行最小但有效的验证命令，例如编译、测试或页面检查。
                    - 每次有意义的代码改动都应提交到 Git；提交信息说明用户可见变化。
                """.trimIndent(),
                sizeBytes = 0L,
                truncated = false
            ),
            ProjectSpaceDocument(
                title = "项目开发流程",
                relativePath = ".github/instructions/project-workflow.instructions.md",
                content = """
                    # 项目开发流程

                    ## 开始任务

                    1. 查看 Git 状态和当前分支。
                    2. 阅读 `AGENTS.md`、`CODEX.md` 和任务相关文档。
                    3. 找到真实入口、数据模型和调用链。

                    ## 完成任务

                    1. 运行必要验证。
                    2. 提交代码。
                    3. 如果有远端，按项目规则推送。
                    4. 汇报验证命令和发布状态。
                """.trimIndent(),
                sizeBytes = 0L,
                truncated = false
            )
        ),
        warnings = warnings
    )
}

private const val PROJECT_DOCS_CACHE = "project_space_docs_cache"
