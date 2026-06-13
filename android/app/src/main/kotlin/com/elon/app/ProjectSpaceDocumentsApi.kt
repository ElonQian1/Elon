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

private data class DefaultProjectDocumentAsset(
    val title: String,
    val relativePath: String,
    val assetPath: String
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
            val bundle = parseProjectSpaceDocuments(context, body)
            cacheProjectSpaceDocuments(context, projectId, body)
            bundle
        }
    }
    return result.getOrElse { error ->
        cachedProjectSpaceDocuments(context, projectId)?.let { cached ->
            cached.copy(warnings = listOf("服务器暂不可用，显示 APK 缓存的项目文档。") + cached.warnings)
        } ?: defaultProjectSpaceDocuments(
            context,
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

private fun parseProjectSpaceDocuments(context: Context, body: String): ProjectSpaceDocumentBundle {
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
        defaultProjectSpaceDocuments(
            context,
            warnings + "服务器没有返回项目文档，显示 APK 内置默认项目文档。"
        )
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
    return runCatching { parseProjectSpaceDocuments(context, body) }.getOrNull()
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

private fun defaultProjectSpaceDocuments(
    context: Context,
    warnings: List<String> = emptyList()
): ProjectSpaceDocumentBundle {
    val fallbackWarnings = warnings.toMutableList()
    val documents = DEFAULT_PROJECT_DOCUMENT_ASSETS.mapNotNull { asset ->
        runCatching {
            context.assets.open(asset.assetPath).bufferedReader(Charsets.UTF_8).use { reader ->
                reader.readText().trim()
            }
        }.fold(
            onSuccess = { content ->
                ProjectSpaceDocument(
                    title = asset.title,
                    relativePath = asset.relativePath,
                    content = content,
                    sizeBytes = content.toByteArray(Charsets.UTF_8).size.toLong(),
                    truncated = false
                )
            },
            onFailure = { error ->
                fallbackWarnings.add(
                    "APK 内置文档资源 ${asset.assetPath} 不可读取：${error.message ?: "未知错误"}"
                )
                null
            }
        )
    }
    if (documents.isNotEmpty()) {
        return ProjectSpaceDocumentBundle(documents = documents, warnings = fallbackWarnings)
    }

    fallbackWarnings.add("APK 内置文档资源缺失，显示最小项目文档。")
    return ProjectSpaceDocumentBundle(
        documents = listOf(
            ProjectSpaceDocument(
                title = "项目 AI 工作入口",
                relativePath = "AGENTS.md",
                content = """
                    # 项目 AI 工作入口

                    请读取 `.github/copilot-instructions.md` 作为共享规则权威来源。
                """.trimIndent(),
                sizeBytes = 0L,
                truncated = false
            )
        ),
        warnings = fallbackWarnings
    )
}

private val DEFAULT_PROJECT_DOCUMENT_ASSETS = listOf(
    DefaultProjectDocumentAsset(
        title = "项目 AI 工作入口",
        relativePath = "AGENTS.md",
        assetPath = "files/AGENTS.md"
    ),
    DefaultProjectDocumentAsset(
        title = "Copilot 共享项目指令",
        relativePath = ".github/copilot-instructions.md",
        assetPath = "files/github/copilot-instructions.md"
    ),
    DefaultProjectDocumentAsset(
        title = "Codex 桥接说明",
        relativePath = "CODEX.md",
        assetPath = "files/CODEX.md"
    ),
    DefaultProjectDocumentAsset(
        title = "Claude 桥接说明",
        relativePath = "CLAUDE.md",
        assetPath = "files/CLAUDE.md"
    ),
    DefaultProjectDocumentAsset(
        title = "Gemini 桥接说明",
        relativePath = "GEMINI.md",
        assetPath = "files/GEMINI.md"
    ),
    DefaultProjectDocumentAsset(
        title = "项目开发流程",
        relativePath = ".github/instructions/project-workflow.instructions.md",
        assetPath = "files/github/instructions/project-workflow.instructions.md"
    ),
    DefaultProjectDocumentAsset(
        title = "Git 与发布流程",
        relativePath = ".github/instructions/git-workflow.instructions.md",
        assetPath = "files/github/instructions/git-workflow.instructions.md"
    ),
    DefaultProjectDocumentAsset(
        title = "Android 与 APK 任务",
        relativePath = ".github/instructions/android.instructions.md",
        assetPath = "files/github/instructions/android.instructions.md"
    ),
    DefaultProjectDocumentAsset(
        title = "UI 与交互任务",
        relativePath = ".github/instructions/ui.instructions.md",
        assetPath = "files/github/instructions/ui.instructions.md"
    ),
    DefaultProjectDocumentAsset(
        title = "后端与 API 任务",
        relativePath = ".github/instructions/backend.instructions.md",
        assetPath = "files/github/instructions/backend.instructions.md"
    ),
    DefaultProjectDocumentAsset(
        title = "项目说明",
        relativePath = "docs/project-readme.md",
        assetPath = "files/docs/project-readme.md"
    )
)

private const val PROJECT_DOCS_CACHE = "project_space_docs_cache"
