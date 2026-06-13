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
    val truncated: Boolean,
    val source: String
)

internal data class ProjectSpaceDocumentBundle(
    val documents: List<ProjectSpaceDocument>,
    val warnings: List<String>,
    val revision: String,
    val source: String,
    val generatedAtMs: Long,
    val cachedAtMs: Long,
    val fromCache: Boolean
)

internal fun fetchProjectSpaceDocuments(
    http: OkHttpClient,
    serverUrl: String,
    context: Context,
    projectId: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute(),
    forceRefresh: Boolean = false
): ProjectSpaceDocumentBundle {
    val cacheKey = projectDocsCacheKey(serverUrl, projectId, route)
    val cached = cachedProjectSpaceDocuments(context, cacheKey)
    val result = runCatching {
        val builder = Request.Builder()
            .url(projectSpaceUrl(serverUrl, projectId, route, "docs"))
            .get()
        if (!forceRefresh) {
            cached?.revision
                ?.takeIf { it.isNotBlank() }
                ?.let { builder.header("If-None-Match", "\"$it\"") }
        }
        val request = AuthManager.applyAuth(
            context,
            builder
        ).build()
        http.newCall(request).execute().use { response ->
            val body = response.body?.string().orEmpty()
            if (response.code == 304 && cached != null) {
                return@use cached.copy(
                    warnings = cacheFallbackWarnings(
                        "服务器文档 revision 未变化，显示 APK 缓存。",
                        cached
                    ) + cached.warnings,
                    fromCache = true
                )
            }
            if (!response.isSuccessful) error(readProjectDocumentError(body, "读取项目文档失败"))
            val bundle = parseProjectSpaceDocuments(context, body)
            cacheProjectSpaceDocuments(context, cacheKey, body)
            bundle
        }
    }
    return result.getOrElse { error ->
        cached?.let { cachedBundle ->
            cachedBundle.copy(
                warnings = cacheFallbackWarnings(
                    "服务器暂不可用，显示 APK 缓存的项目文档。",
                    cachedBundle
                ) + cachedBundle.warnings,
                fromCache = true
            )
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
        ProjectSpaceDocumentBundle(
            documents = documents,
            warnings = warnings,
            revision = root.optString("revision", ""),
            source = root.optString("source", ""),
            generatedAtMs = root.optLong("generated_at_ms", 0L),
            cachedAtMs = 0L,
            fromCache = false
        )
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
        truncated = doc.optBoolean("truncated", false),
        source = doc.optString("source", "")
    )
}

internal fun clearCachedProjectSpaceDocuments(
    context: Context,
    serverUrl: String,
    projectId: String,
    route: ProjectSpaceRoute = ProjectSpaceRoute()
) {
    context.getSharedPreferences(PROJECT_DOCS_CACHE, Context.MODE_PRIVATE)
        .edit()
        .remove(projectDocsCacheKey(serverUrl, projectId, route))
        .apply()
}

private fun cacheProjectSpaceDocuments(context: Context, cacheKey: String, body: String) {
    val wrapped = JSONObject()
        .put("schema", PROJECT_DOCS_CACHE_SCHEMA)
        .put("cached_at_ms", System.currentTimeMillis())
        .put("body", JSONObject(body))
        .toString()
    context.getSharedPreferences(PROJECT_DOCS_CACHE, Context.MODE_PRIVATE)
        .edit()
        .putString(cacheKey, wrapped)
        .apply()
}

private fun cachedProjectSpaceDocuments(
    context: Context,
    cacheKey: String
): ProjectSpaceDocumentBundle? {
    val body = context.getSharedPreferences(PROJECT_DOCS_CACHE, Context.MODE_PRIVATE)
        .getString(cacheKey, null)
        ?: return null
    return runCatching { parseCachedProjectSpaceDocuments(context, body) }.getOrNull()
}

private fun parseCachedProjectSpaceDocuments(
    context: Context,
    cachedBody: String
): ProjectSpaceDocumentBundle {
    val root = JSONObject(cachedBody)
    val cachedAtMs = root.optLong("cached_at_ms", 0L)
    val body = root.optJSONObject("body")?.toString() ?: cachedBody
    return parseProjectSpaceDocuments(context, body).copy(
        cachedAtMs = cachedAtMs,
        fromCache = true
    )
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
    val manifest = readDefaultDocsManifest(context, fallbackWarnings)
    val documents = manifest.documentPaths.mapNotNull { relativePath ->
        if (!relativePath.endsWith(".md", ignoreCase = true)) return@mapNotNull null
        val assetPath = defaultDocAssetPath(relativePath)
        runCatching {
            context.assets.open(assetPath).bufferedReader(Charsets.UTF_8).use { reader ->
                reader.readText().trim()
            }
        }.fold(
            onSuccess = { content ->
                ProjectSpaceDocument(
                    title = markdownTitle(content) ?: relativePath.substringAfterLast('/'),
                    relativePath = relativePath,
                    content = content,
                    sizeBytes = content.toByteArray(Charsets.UTF_8).size.toLong(),
                    truncated = false,
                    source = "apk_default"
                )
            },
            onFailure = { error ->
                fallbackWarnings.add(
                    "APK 内置文档资源 $assetPath 不可读取：${error.message ?: "未知错误"}"
                )
                null
            }
        )
    }
    if (documents.isNotEmpty()) {
        return ProjectSpaceDocumentBundle(
            documents = documents,
            warnings = fallbackWarnings,
            revision = "apk-default-${manifest.templateVersion}",
            source = "apk_default",
            generatedAtMs = System.currentTimeMillis(),
            cachedAtMs = 0L,
            fromCache = false
        )
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
                truncated = false,
                source = "apk_default"
            )
        ),
        warnings = fallbackWarnings,
        revision = "apk-default-minimal",
        source = "apk_default",
        generatedAtMs = System.currentTimeMillis(),
        cachedAtMs = 0L,
        fromCache = false
    )
}

private fun projectDocsCacheKey(
    serverUrl: String,
    projectId: String,
    route: ProjectSpaceRoute
): String {
    val server = serverUrl.trim().trimEnd('/').ifBlank { "server" }
    val scope = route.userId?.trim()?.takeIf { it.isNotBlank() } ?: "self"
    return "$server|$scope|$projectId"
}

private fun cacheFallbackWarnings(
    message: String,
    cachedBundle: ProjectSpaceDocumentBundle
): List<String> {
    val warnings = mutableListOf(message)
    if (cachedBundle.cachedAtMs > 0L) {
        val ageHours = ((System.currentTimeMillis() - cachedBundle.cachedAtMs)
            .coerceAtLeast(0L) / (60L * 60L * 1000L))
        if (ageHours >= 24L) {
            warnings.add("APK 文档缓存已超过 ${ageHours} 小时，联网后建议点刷新。")
        }
    }
    return warnings
}

private data class DefaultDocsManifest(
    val templateVersion: String,
    val documentPaths: List<String>
)

private fun readDefaultDocsManifest(
    context: Context,
    warnings: MutableList<String>
): DefaultDocsManifest {
    val fallback = DefaultDocsManifest(
        templateVersion = "bundled",
        documentPaths = listOf("AGENTS.md")
    )
    return runCatching {
        val raw = context.assets.open(DEFAULT_DOCS_MANIFEST_ASSET)
            .bufferedReader(Charsets.UTF_8)
            .use { it.readText() }
        val root = JSONObject(raw)
        val paths = mutableListOf<String>()
        root.optJSONArray("documents")?.let { array ->
            for (index in 0 until array.length()) {
                array.optJSONObject(index)
                    ?.optString("path")
                    ?.takeIf { it.isNotBlank() }
                    ?.let(paths::add)
            }
        }
        DefaultDocsManifest(
            templateVersion = root.optString("template_version", "bundled"),
            documentPaths = paths.ifEmpty { fallback.documentPaths }
        )
    }.getOrElse { error ->
        warnings.add("APK 内置默认文档 manifest 不可读取：${error.message ?: "未知错误"}")
        fallback
    }
}

private fun defaultDocAssetPath(relativePath: String): String {
    val normalized = relativePath.replace('\\', '/').trimStart('/')
    val assetRelative = when {
        normalized.startsWith(".github/") -> "github/" + normalized.removePrefix(".github/")
        normalized.startsWith(".elon/") -> "elon/" + normalized.removePrefix(".elon/")
        else -> normalized
    }
    return "files/$assetRelative"
}

private fun markdownTitle(content: String): String? {
    return content.lineSequence()
        .map { it.trim() }
        .firstOrNull { it.startsWith("# ") && it.length > 2 }
        ?.removePrefix("# ")
        ?.trim()
        ?.takeIf { it.isNotBlank() }
}

private const val DEFAULT_DOCS_MANIFEST_ASSET = "files/elon/default-docs.json"
private const val PROJECT_DOCS_CACHE = "project_space_docs_cache"
private const val PROJECT_DOCS_CACHE_SCHEMA = 2
