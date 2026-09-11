package com.elon.app.chatgptweb

import org.json.JSONObject

internal object ChatGptWebLibrarySourceDiagnostic {
    const val SCHEMA = "elon.library_sources.v1"
    private val kinds = setOf("none", "library", "file", "external", "other")
    private val artifacts = setOf("none", "other", "saved_entity", "deep_research_report",
        "flashcards", "learning_quiz", "app_block", "site_preview")
    private val mimeTypes = setOf("none", "other", "image/png", "image/jpeg", "application/pdf", "text/plain")
    private val sourceFlags = setOf("external_account", "cloud_doc_url", "saved_entity", "trashed_at", "is_project",
        "gizmo_id", "project_id", "context_scopes", "preview_file", "mounted_library_file_id",
        "library_file_id", "shared_library_file_id", "library_download_id", "context_connector_info", "library_provider")

    fun sanitize(value: JSONObject): String {
        require(value.keys().asSequence().toSet() == setOf("schema", "observed", "stale", "total", "omitted", "groups"))
        require(value.opt("schema") == SCHEMA && value.opt("observed") is Boolean && value.opt("stale") is Boolean)
        val total = integer(value, "total", 0..500)
        val omitted = integer(value, "omitted", 0..total)
        val groups = value.getJSONArray("groups")
        require(groups.length() <= 16)
        var count = omitted
        val seen = mutableSetOf<String>()
        for (index in 0 until groups.length()) {
            val group = groups.getJSONObject(index)
            require(group.keys().asSequence().toSet() == setOf("kind", "node", "file", "artifact", "mime",
                "flags", "download", "attach", "rename", "trash", "count"))
            require(group.opt("kind") in setOf("file", "directory"))
            require(group.opt("node") in kinds && group.opt("file") in kinds)
            require(group.opt("artifact") in artifacts && group.opt("mime") in mimeTypes)
            for (key in listOf("download", "attach", "rename", "trash")) require(group.opt(key) is Boolean)
            val flags = group.getJSONArray("flags")
            require(flags.length() <= sourceFlags.size)
            val unique = mutableSetOf<String>()
            for (flagIndex in 0 until flags.length()) {
                val flag = flags.get(flagIndex) as? String ?: error("flag_type")
                require(flag in sourceFlags && unique.add(flag))
            }
            val signature = listOf("kind", "node", "file", "artifact", "mime", "download", "attach", "rename", "trash")
                .map { group.get(it).toString() } + unique.sorted()
            require(seen.add(signature.joinToString("|")))
            count += integer(group, "count", 1..500)
        }
        require(count == total)
        if (!value.getBoolean("observed")) require(total == 0 && groups.length() == 0 && !value.getBoolean("stale"))
        return value.toString()
    }

    private fun integer(value: JSONObject, key: String, range: IntRange): Int {
        val raw = value.opt(key)
        require(raw is Int || raw is Long)
        val number = (raw as Number).toLong()
        require(number in range.first.toLong()..range.last.toLong())
        return number.toInt()
    }
}
