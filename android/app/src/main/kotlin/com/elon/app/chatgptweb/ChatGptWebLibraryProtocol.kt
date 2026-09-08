package com.elon.app.chatgptweb

import com.elon.app.WebChatLibraryBreadcrumb
import com.elon.app.WebChatLibraryEntry
import com.elon.app.WebChatLibrarySnapshot
import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebLibraryProtocol {
    const val ACTION = "list_library_files"
    val HANDLE = Regex("library_[a-f0-9]{32}")
    private val REQUEST = Regex("mcp_[a-z0-9]{1,32}")

    fun parse(value: JSONObject): WebChatLibrarySnapshot? {
        if (value.optInt("version") != 1) return null
        val requestId = value.optString("requestId").takeIf(REQUEST::matches) ?: return null
        val directory = value.optString("directoryHandle")
        if (directory.isNotEmpty() && !HANDLE.matches(directory)) return null
        val query = value.optString("query").takeIf { it.length <= 200 && it.none(Char::isISOControl) } ?: return null
        val rawTrail = value.optJSONArray("breadcrumbs") ?: return null
        if (rawTrail.length() > 32) return null
        val trail = (0 until rawTrail.length()).map { index ->
            val row = rawTrail.optJSONObject(index) ?: return null
            val handle = row.optString("handle").takeIf(HANDLE::matches) ?: return null
            WebChatLibraryBreadcrumb(handle, label(row.optString("name")) ?: return null)
        }
        if (directory.isEmpty() && trail.isNotEmpty() || directory.isNotEmpty() && trail.lastOrNull()?.handle != directory) return null
        val rows = value.optJSONArray("items") ?: return null
        if (rows.length() > 500) return null
        var partial = value.optBoolean("partial")
        val items = (0 until rows.length()).mapNotNull { index ->
            entry(rows.optJSONObject(index)).also { if (it == null) partial = true }
        }.distinctBy(WebChatLibraryEntry::handle)
        if (items.size != rows.length()) partial = true
        return WebChatLibrarySnapshot(requestId, directory, query, trail, items,
            value.optBoolean("hasMore"), partial, value.optBoolean("stale"))
    }

    private fun entry(row: JSONObject?): WebChatLibraryEntry? {
        row ?: return null
        val handle = row.optString("handle").takeIf(HANDLE::matches) ?: return null
        val kind = row.optString("kind").takeIf { it in setOf("file", "directory") } ?: return null
        val name = label(row.optString("name")) ?: return null
        val mime = row.optString("mediaType")
        if (mime.isNotEmpty() && !Regex("[A-Za-z0-9.+-]{1,63}/[A-Za-z0-9.+-]{1,63}").matches(mime)) return null
        val number = row.opt("sizeBytes") as? Number ?: return null
        val size = number.toLong()
        if (size < -1 || number.toDouble() != size.toDouble() || size > 9_007_199_254_740_991L) return null
        val download = row.optString("downloadHandle")
        if (download.isNotEmpty() && (!ChatGptWebFileDownloadPolicy.HANDLE.matches(download) || kind != "file")) return null
        return WebChatLibraryEntry(handle, kind, name, mime, size, download,
            kind == "file" && row.opt("canRename") == true, kind == "file" && row.opt("canTrash") == true,
            kind == "file" && row.opt("canAttach") == true)
    }

    private fun label(value: String): String? = value.takeIf {
        it.isNotBlank() && it.length <= 180 && it.none(Char::isISOControl)
    }

    fun json(value: WebChatLibrarySnapshot?): Any = value?.let {
        JSONObject().put("request_id", it.requestId).put("directory_handle", it.directoryHandle)
            .put("query", it.query).put("has_more", it.hasMore).put("partial", it.partial).put("stale", it.stale)
            .put("breadcrumbs", JSONArray(it.breadcrumbs.map { row ->
                JSONObject().put("handle", row.handle).put("name", row.name)
            })).put("items", JSONArray(it.items.map { row ->
                JSONObject().put("handle", row.handle).put("name", row.name).put("kind", row.kind)
                    .put("media_type", row.mediaType).put("size_bytes", row.sizeBytes).put("download_handle", row.downloadHandle)
                    .put("can_rename", row.canRename).put("can_trash", row.canTrash).put("can_attach", row.canAttach)
            }))
    } ?: JSONObject.NULL
}
