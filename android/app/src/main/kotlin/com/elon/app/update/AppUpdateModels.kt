package com.elon.app.update

import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale

internal data class AppUpdateSource(
    val url: String,
    val type: String,
    val priority: Int,
) {
    val displayName: String
        get() = when (type.lowercase(Locale.ROOT)) {
            "peer", "lan", "wifi" -> "同 WiFi 设备"
            else -> "官方服务器"
        }
}

internal data class AppUpdateVersion(
    val versionCode: Int,
    val versionName: String,
    val downloadUrl: String,
    val changelog: String,
    val forceUpdate: Boolean,
    val fileSize: Long,
    val sha256: String,
    val mirrors: List<AppUpdateSource> = emptyList(),
) {
    fun downloadSources(): List<AppUpdateSource> =
        (mirrors.filter { it.url.isNotBlank() }.sortedByDescending { it.priority } +
            AppUpdateSource(downloadUrl, "server", Int.MIN_VALUE))
            .distinctBy { it.url }

    fun toJson(): String = JSONObject()
        .put("versionCode", versionCode)
        .put("versionName", versionName)
        .put("downloadUrl", downloadUrl)
        .put("changelog", changelog)
        .put("forceUpdate", forceUpdate)
        .put("fileSize", fileSize)
        .put("sha256", sha256)
        .put(
            "mirrors",
            JSONArray().apply {
                mirrors.forEach { source ->
                    put(
                        JSONObject()
                            .put("url", source.url)
                            .put("type", source.type)
                            .put("priority", source.priority)
                    )
                }
            }
        )
        .toString()

    companion object {
        fun parse(jsonText: String): AppUpdateVersion? = runCatching {
            val json = JSONObject(jsonText)
            val sources = buildList {
                val mirrors = json.optJSONArray("mirrors") ?: JSONArray()
                for (index in 0 until mirrors.length()) {
                    val mirror = mirrors.optJSONObject(index) ?: continue
                    val url = mirror.optString("url").trim()
                    if (url.isNotBlank()) {
                        add(
                            AppUpdateSource(
                                url = url,
                                type = mirror.optString("type", "server"),
                                priority = mirror.optInt("priority", 0),
                            )
                        )
                    }
                }
            }
            AppUpdateVersion(
                versionCode = json.intAny("versionCode", "version_code", "build"),
                versionName = json.stringAny("versionName", "version_name", "version"),
                downloadUrl = json.stringAny("downloadUrl", "download_url"),
                changelog = json.optString("changelog", "").trim(),
                forceUpdate = json.booleanAny("forceUpdate", "force_update"),
                fileSize = json.longAny("fileSize", "file_size"),
                sha256 = json.stringAny("sha256", "sha_256").lowercase(Locale.ROOT),
                mirrors = sources,
            ).takeIf { it.versionCode > 0 && it.downloadUrl.isNotBlank() }
        }.getOrNull()
    }
}

internal enum class AppUpdatePhase {
    AVAILABLE,
    QUEUED,
    DOWNLOADING,
    VERIFYING,
    READY,
    FAILED,
}

internal data class AppUpdateSnapshot(
    val versionCode: Int,
    val versionName: String,
    val phase: AppUpdatePhase,
    val downloadedBytes: Long = 0L,
    val totalBytes: Long = 0L,
    val bytesPerSecond: Long = 0L,
    val sourceName: String = "",
    val errorMessage: String = "",
    val apkPath: String = "",
    val updatedAt: Long = System.currentTimeMillis(),
) {
    val progressPercent: Int
        get() = if (totalBytes > 0L) {
            ((downloadedBytes * 100L) / totalBytes).toInt().coerceIn(0, 100)
        } else {
            0
        }

    fun toJson(): String = JSONObject()
        .put("versionCode", versionCode)
        .put("versionName", versionName)
        .put("phase", phase.name)
        .put("downloadedBytes", downloadedBytes)
        .put("totalBytes", totalBytes)
        .put("bytesPerSecond", bytesPerSecond)
        .put("sourceName", sourceName)
        .put("errorMessage", errorMessage)
        .put("apkPath", apkPath)
        .put("updatedAt", updatedAt)
        .toString()

    companion object {
        fun parse(jsonText: String?): AppUpdateSnapshot? {
            if (jsonText.isNullOrBlank()) return null
            return runCatching {
                val json = JSONObject(jsonText)
                AppUpdateSnapshot(
                    versionCode = json.optInt("versionCode", 0),
                    versionName = json.optString("versionName", ""),
                    phase = AppUpdatePhase.valueOf(json.optString("phase")),
                    downloadedBytes = json.optLong("downloadedBytes", 0L),
                    totalBytes = json.optLong("totalBytes", 0L),
                    bytesPerSecond = json.optLong("bytesPerSecond", 0L),
                    sourceName = json.optString("sourceName", ""),
                    errorMessage = json.optString("errorMessage", ""),
                    apkPath = json.optString("apkPath", ""),
                    updatedAt = json.optLong("updatedAt", 0L),
                )
            }.getOrNull()?.takeIf { it.versionCode > 0 }
        }
    }
}

internal fun formatUpdateBytes(bytes: Long): String = when {
    bytes >= 1_048_576L -> "%.1f MB".format(Locale.US, bytes / 1_048_576.0)
    bytes >= 1_024L -> "%.1f KB".format(Locale.US, bytes / 1_024.0)
    else -> "$bytes B"
}

internal fun formatUpdateEta(snapshot: AppUpdateSnapshot): String? {
    if (snapshot.bytesPerSecond <= 0L || snapshot.totalBytes <= snapshot.downloadedBytes) return null
    val seconds = (snapshot.totalBytes - snapshot.downloadedBytes) / snapshot.bytesPerSecond
    return when {
        seconds < 1L -> "即将完成"
        seconds < 60L -> "约 ${seconds} 秒"
        else -> "约 ${(seconds + 59L) / 60L} 分钟"
    }
}

private fun JSONObject.stringAny(vararg names: String): String {
    names.forEach { name ->
        val value = optString(name, "").trim()
        if (value.isNotEmpty()) return value
    }
    return ""
}

private fun JSONObject.intAny(vararg names: String): Int {
    names.forEach { name ->
        if (!has(name)) return@forEach
        val value = opt(name)
        if (value is Number) return value.toInt()
        optString(name).toIntOrNull()?.let { return it }
    }
    return 0
}

private fun JSONObject.longAny(vararg names: String): Long {
    names.forEach { name ->
        if (!has(name)) return@forEach
        val value = opt(name)
        if (value is Number) return value.toLong()
        optString(name).toLongOrNull()?.let { return it }
    }
    return 0L
}

private fun JSONObject.booleanAny(vararg names: String): Boolean {
    names.forEach { name ->
        if (!has(name)) return@forEach
        val value = opt(name)
        if (value is Boolean) return value
        optString(name).toBooleanStrictOrNull()?.let { return it }
    }
    return false
}
