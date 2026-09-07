package com.elon.app.chatgptweb

import org.json.JSONArray
import org.json.JSONObject

internal object ChatGptWebPrivateRuntimeAssets {
    const val SCHEMA = "elon.private_runtime_assets.v1"
    private val keys = setOf("schema", "assets", "truncated")
    private val filename = Regex("[A-Za-z0-9][A-Za-z0-9_-]{0,95}\\.js")

    fun sanitize(value: JSONObject): String {
        require(value.keys().asSequence().toSet() == keys)
        require(value.opt("schema") == SCHEMA && value.opt("truncated") is Boolean)
        val assets = value.getJSONArray("assets")
        require(assets.length() <= 96)
        val names = linkedSetOf<String>()
        for (index in 0 until assets.length()) {
            val name = assets.get(index) as? String ?: error("asset_type")
            require(filename.matches(name) && names.add(name))
        }
        return JSONObject().put("schema", SCHEMA).put("assets", JSONArray(names.sorted()))
            .put("truncated", value.getBoolean("truncated")).toString()
    }
}
