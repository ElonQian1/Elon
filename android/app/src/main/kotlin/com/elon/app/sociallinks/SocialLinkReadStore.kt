package com.elon.app.sociallinks

import android.content.Context
import org.json.JSONObject
import java.security.MessageDigest

/** Device-local bounded metadata. Expiry is fixed at observation time, never extended by reads. */
internal object SocialLinkReadStore {
    private const val TTL = 24 * 3600000L
    private fun prefs(context: Context) = context.getSharedPreferences("social-link-read-preview-v1", Context.MODE_PRIVATE)
    private fun key(server: String, owner: String?, url: String) = MessageDigest.getInstance("SHA-256")
        .digest("$server\n$owner\n$url".toByteArray()).joinToString("") { "%02x".format(it) }
    @Synchronized fun get(context: Context, server: String, owner: String?, url: String, now: Long = System.currentTimeMillis()): SocialLink? {
        val store = prefs(context); val key = key(server, owner, url)
        return runCatching {
            val raw = store.getString(key, null) ?: return null
            if (raw.length > 16384) { store.edit().remove(key).apply(); return null }
            val value = JSONObject(raw); val saved = value.getLong("saved")
            if (saved > now || now - saved >= TTL) { store.edit().remove(key).apply(); return null }
            SocialLinkReadPreview.parse(url, value.getJSONObject("preview"))
        }.getOrNull()
    }
    @Synchronized fun put(context: Context, server: String, owner: String?, value: JSONObject, now: Long = System.currentTimeMillis()) {
        val original = value.optString("original")
        val item = SocialLinkReadPreview.parse(original, value) ?: return
        // Rebuild the record to prevent unexpected adapter fields from becoming persisted data.
        val preview = JSONObject().put("schema", 1).put("original", original).put("url", value.optString("url"))
            .put("article", true).put("title", item.title).put("author", item.author).put("description", item.summary).put("image", item.image)
        val record = JSONObject().put("saved", now).put("preview", preview).toString()
        if (record.length > 16384) return
        val store = prefs(context); val editor = store.edit(); val id = key(server, owner, original)
        val remaining = store.all.mapNotNull { (k, v) ->
            val time = runCatching { JSONObject(v as String).getLong("saved") }.getOrDefault(0)
            if (time > now || now - time >= TTL || k == id) { editor.remove(k); null } else k to time
        }.sortedBy { it.second }
        remaining.take((remaining.size - 127).coerceAtLeast(0)).forEach { editor.remove(it.first) }
        editor.putString(id, record).apply()
    }
}
